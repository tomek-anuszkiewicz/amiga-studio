"""Measure and compare four release Mixed Scene FPS cases serially."""

from dataclasses import dataclass
import hashlib
import importlib.metadata
import json
import math
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import sys
import tempfile
import time

from execution import VERIFIER_LOG_PATH
from repository import REPO_ROOT, git
from run_mixed_scene import find_executable, read_metrics

SCENE = REPO_ROOT / "tests/mixed_scene"
PROGRAM = SCENE / "program.bin"
ADF = SCENE / "mixed_scene.adf"
KICKSTART_ROM = Path("tests/kickstarts/kickstart-1.3.rom")
ROM_SIZE = 256 * 1024
REFERENCE = REPO_ROOT / "benchmarks/mixed_scene_fps_reference.json"
TEMP_ROOT = REPO_ROOT / ".agent/tmp/mixed-scene-fps"
SCHEMA = 2
SERIES = 3
WARMUP = 32
FIELDS = 120
SECONDS = 10
GAME_SPEED = 4.0
GAME_CAP_WARNING_FPS = 190
BUILD_TIMEOUT = 900
SAMPLE_TIMEOUT = 90
CASES = {
    "FPS-01": "Native machine loop, uncapped",
    "FPS-02": "Browser worker machine loop, uncapped",
    "FPS-03": "Native Game, emulated fields at 4x",
    "FPS-04": "Browser Game, emulated fields at 4x",
}
GAME_CASES = ("FPS-03", "FPS-04")
BROWSER_ENV = "AMIGA_FPS_BROWSER_EXECUTABLE"
REFRESH_ENV = "AMIGA_FPS_NATIVE_REFRESH_HZ"


@dataclass(frozen=True)
class Settings:
    kickstart_rom: Path
    browser_executable: Path | None
    native_refresh_hz: float | None


def optional_path(name):
    value = os.environ.get(name)
    try:
        return Path(value).expanduser().resolve() if value else None
    except (OSError, RuntimeError, ValueError):
        raise ValueError(f"The path in {name} could not be resolved.") from None


def require_kickstart_rom():
    fixture = REPO_ROOT / KICKSTART_ROM
    expected = f"{KICKSTART_ROM.as_posix()} must be a readable {ROM_SIZE}-byte Kickstart 1.3 ROM."
    try:
        rom = fixture.resolve()
        with rom.open("rb") as stream:
            size = os.fstat(stream.fileno()).st_size
    except (OSError, RuntimeError):
        raise FileNotFoundError(f"{expected} The file is missing or unreadable.") from None
    if size != ROM_SIZE:
        raise ValueError(f"{expected} Found {size} bytes.")
    return rom


def load_settings():
    rom = require_kickstart_rom()
    refresh = os.environ.get(REFRESH_ENV)
    if refresh is not None:
        try:
            refresh = float(refresh)
        except ValueError:
            refresh = None
        if not positive_number(refresh):
            raise ValueError(f"{REFRESH_ENV} must be a positive finite number.")
    return Settings(rom, optional_path(BROWSER_ENV), refresh)


def command(args, *, env=None, timeout=BUILD_TIMEOUT):
    completed = subprocess.run(args, cwd=REPO_ROOT, env=env, timeout=timeout,
                               capture_output=True, encoding="utf-8", errors="replace")
    if completed.returncode:
        detail = (completed.stderr + "\n" + completed.stdout).strip()
        raise RuntimeError(f"{args[0]} exited {completed.returncode}\n{detail}")
    return completed.stdout


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def game_media_identity():
    try:
        adf_digest = sha256(ADF)
    except OSError:
        raise FileNotFoundError("The checked-in Mixed Scene ADF is unavailable.") from None
    return {"boot_mode": "manual-setup-df0", "adf_sha256": adf_digest}


def source_identity():
    revision = git(REPO_ROOT, "rev-parse", "HEAD").decode().strip()
    diff = git(REPO_ROOT, "diff", "--no-ext-diff", "--binary", "HEAD")
    untracked = git(REPO_ROOT, "ls-files", "--others", "--exclude-standard", "-z").split(b"\0")
    active_log = os.environ.get(VERIFIER_LOG_PATH)
    active_log = Path(active_log).resolve() if active_log else None
    digest = hashlib.sha256(diff)
    dirty = bool(diff)
    for name in sorted(filter(None, untracked)):
        path = REPO_ROOT / name.decode("utf-8")
        # A redirected command log can grow inside the checkout during this run.
        if path == active_log:
            continue
        dirty = True
        digest.update(name + b"\0")
        digest.update(sha256(path).encode())
    return {"revision": revision, "worktree_sha256": digest.hexdigest(),
            "dirty": dirty}


def host_identity():
    cpu = platform.processor() or platform.machine()
    power = None
    if sys.platform == "win32":
        import winreg

        with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE,
                           r"HARDWARE\DESCRIPTION\System\CentralProcessor\0") as processor:
            cpu = winreg.QueryValueEx(processor, "ProcessorNameString")[0].strip()
        power = {"scheme": command(["powercfg", "/getactivescheme"], timeout=15).strip(),
                 "settings_sha256": hashlib.sha256(
                     command(["powercfg", "/query"], timeout=15).encode()).hexdigest()}
    else:
        settings = Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
        if settings.is_file():
            power = {"scaling_governor": settings.read_text().strip()}
    return {"cpu": cpu, "os": platform.platform(), "architecture": platform.machine(),
            "logical_cpus": os.cpu_count(), "power": power}


def build_settings(wasm=False):
    toolchain = ["+nightly"] if wasm else []
    settings = {"profile": "release",
                "compiler": command(["rustc", *toolchain, "-vV"], timeout=30).strip(),
                "cargo": command(["cargo", *toolchain, "-V"], timeout=30).strip(),
                "release_overrides": {key: value for key, value in os.environ.items()
                                      if key.startswith("CARGO_PROFILE_RELEASE_")}}
    if wasm:
        settings.update(trunk=command(["trunk", "--version"], timeout=30).strip(),
                        web_pipeline_sha256=sha256(REPO_ROOT / "tools/web.ps1"),
                        target="wasm32-unknown-unknown", shared_memory=True)
    else:
        settings["rustflags"] = {key: os.environ[key] for key in (
            "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_TARGET",
        ) if key in os.environ}
    return settings


def native_bench():
    output = command([sys.executable, str(REPO_ROOT / "tools/verification/run_mixed_scene.py")])
    report = read_metrics(output)
    if report is None:
        raise ValueError("Native machine loop did not report benchmark metrics")
    return report


def web_build(dist, *, machine_loop=False):
    env = os.environ.copy()
    for name in ("TRUNK_CONFIG", "TRUNK_BUILD_EXAMPLE", "TRUNK_BUILD_FEATURES"):
        env.pop(name, None)
    env.update(TRUNK_BUILD_DIST=str(dist), TRUNK_SKIP_VERSION_CHECK="true")
    if machine_loop:
        target = REPO_ROOT / "crates/machine_loop/examples/mixed_scene.html"
        config = dist.with_suffix(".toml")
        config.write_text(f"[build]\ntarget = {json.dumps(target.as_posix())}\n"
                          "filehash = false\ninject_scripts = false\n", encoding="utf-8")
        env.update(TRUNK_CONFIG=str(config), TRUNK_BUILD_EXAMPLE="mixed_scene_wasm")
    else:
        env["TRUNK_BUILD_FEATURES"] = "game-fps-benchmark"
    command(["pwsh", "-NoProfile", "-File", str(REPO_ROOT / "tools/web.ps1"), "build"], env=env)
    module = dist / ("machine_loop_bg.wasm" if machine_loop else "gui_bg.wasm")
    return sha256(module)


def chromium_settings(browser):
    return {"engine": "Chromium", "version": browser.version,
            "playwright": importlib.metadata.version("playwright"), "headless": False,
            "window_size": [1600, 1000], "normal_background_throttling": True}


def chromium(playwright, executable):
    return playwright.chromium.launch(
        executable_path=str(executable) if executable is not None else None,
        headless=False, args=["--window-size=1600,1000"],
        ignore_default_args=["--disable-background-timer-throttling",
                             "--disable-backgrounding-occluded-windows",
                             "--disable-renderer-backgrounding"],
        timeout=SAMPLE_TIMEOUT * 1000,
    )


def browser_bench(url, browser):
    page = browser.new_page(no_viewport=True)
    try:
        page.goto(url, timeout=SAMPLE_TIMEOUT * 1000)
        page.evaluate("""timeout => {
            const worker = new Worker('mixed_scene_worker.js', {type: 'module'});
            const complete = result => {
                clearTimeout(timer);
                worker.terminate();
                window.mixedSceneResult = result;
            };
            const timer = setTimeout(() => complete({error: 'Worker timed out'}), timeout);
            worker.onmessage = event => complete(event.data);
            worker.onerror = event => complete({error: event.message});
            worker.onmessageerror = () => complete({error: 'Unreadable worker result'});
        }""", SAMPLE_TIMEOUT * 1000)
        page.wait_for_function("window.mixedSceneResult !== undefined", polling=250,
                               timeout=SAMPLE_TIMEOUT * 1000)
        result = page.evaluate("window.mixedSceneResult")
        if result.get("error"):
            raise RuntimeError(result["error"])
        return result["report"]
    finally:
        page.close()


def uncapped_samples(report):
    if (report.get("mode") != "bench" or report.get("standard") != "Pal"
            or report.get("warmup_fields") != WARMUP or report.get("fields_per_series") != FIELDS
            or report.get("emulated_ccks_per_series", 0) <= 0 or report.get("halted")):
        raise ValueError("Unexpected machine-loop workload or execution status")
    check_progress(report)
    return validate_samples([{"count": FIELDS, "seconds": seconds}
                             for seconds in report["series_seconds"]])


def check_progress(report):
    before, after = report["guest_before"], report["guest_after"]
    if (before["fault"] != 0 or after["fault"] != 0
            or before["fields"] >= after["fields"] or before["work_loops"] >= after["work_loops"]):
        raise ValueError("Guest counters failed to advance or guest fault flag is set")


def verify_matching_work(native, wasm):
    keys = ("emulated_ccks_per_series", "guest_before", "guest_after", "height", "width", "pc")
    differences = [key for key in keys if native[key] != wasm[key]]
    if differences:
        raise ValueError("Native and browser guest work differs in " + ", ".join(differences))


def validate_samples(samples):
    if len(samples) != SERIES:
        raise ValueError(f"Expected {SERIES} complete samples")
    result = []
    for sample in samples:
        count, seconds = sample["count"], sample["seconds"]
        if type(count) is not int or count < 0 or not positive_number(seconds):
            raise ValueError("Invalid sample count or duration")
        result.append({**sample, "fps": count / seconds})
    return result


def positive_number(number):
    return type(number) in (int, float) and math.isfinite(number) and number > 0


def unsigned_64(value):
    return type(value) is int and 0 <= value < (1 << 64)


def game_sample(result):
    if result.get("error"):
        raise RuntimeError(result["error"])
    generation, sample = result.get("generation"), result.get("sample")
    if not unsigned_64(generation) or generation <= 1 or not isinstance(sample, dict):
        raise ValueError("Expected a fresh Game generation and a timed interval")
    warmup_start, start, end = (sample.get(key) for key in (
        "warmup_start_field", "start_field", "end_field",
    ))
    seconds = sample.get("seconds")
    if not all(unsigned_64(field) for field in (warmup_start, start, end)):
        raise ValueError("Expected completed-field boundary counts for the Game interval")
    if not positive_number(seconds) or seconds < SECONDS:
        raise ValueError(f"Expected a Game interval of at least {SECONDS} seconds")
    if sample.get("speed") != GAME_SPEED:
        raise ValueError("Expected the Game interval at 4x speed")

    counters = ("cck", "field_number", "fields", "work_loops")
    observed = [result.get(key) for key in ("boot_first", "guest_before", "guest_after")]
    for observation in observed:
        if not isinstance(observation, dict) or any(
            not unsigned_64(observation.get(key))
            for key in (*counters, "fault")
        ):
            raise ValueError("Incomplete Game boot or post-timing observation")
        if observation["fault"]:
            raise ValueError("Game guest fault flag is set")
    for before, after in zip(observed, observed[1:]):
        if any(before[key] >= after[key] for key in counters):
            raise ValueError("Game boot or post-timing counters failed to advance")
    _, before, after = observed
    if not before["field_number"] <= warmup_start <= start <= end <= after["field_number"]:
        raise ValueError("Game field boundaries regressed or fall outside guest observations")
    if start - warmup_start < WARMUP:
        raise ValueError(f"Expected at least {WARMUP} completed Game fields before timing")
    count = end - start
    return {**sample, "count": count, "fps": count / seconds}


def record_game_sample(row, report):
    row.setdefault("raw", []).append(report)
    sample = game_sample(report)
    settings = {"renderer": report["renderer"], **report["display"],
                "display_refresh_hz": report["display_refresh_hz"]}
    row.setdefault("samples", []).append(sample)
    if row["compatibility"].setdefault("display", settings) != settings:
        raise ValueError("Game display settings differed between sessions")


def game_cap_warning(case, samples):
    for index, sample in enumerate(samples, start=1):
        fps = sample["fps"]
        if fps >= GAME_CAP_WARNING_FPS:
            return (f"Warning: {case} sample {index} reached {fps:.3f} FPS and is approaching "
                    "the 4x PAL playback limit. Measuring higher throughput requires a decision "
                    "about raising the application's playback speed limit.")
    return None


def load_reference(path):
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return {}, "Reference file is missing"
    except (ValueError, OSError) as error:
        return {}, f"Reference file could not be read: {error}"
    if not isinstance(data, dict) or data.get("schema") != SCHEMA or not isinstance(data.get("cases"), dict):
        return {}, "Reference schema is incompatible"
    return data["cases"], None


def reference_compatibility(case, compatibility):
    if case in GAME_CASES and isinstance(compatibility, dict):
        workload = compatibility.get("workload")
        if isinstance(workload, dict) and "rom_sha256" in workload:
            # The approved reference retains this digest as calibration provenance.
            return {**compatibility, "workload": {
                key: value for key, value in workload.items() if key != "rom_sha256"
            }}
    return compatibility


def compare_case(case, row, reference, missing_reason=None, *, cap_warning=None):
    result = {**row, "median_fps": None, "reference_fps": None,
              "tolerance": None, "change_percent": None,
              "minimum_fps": None, "maximum_fps": None, "reason": None}
    samples = row.get("samples", [])
    if len(samples) == SERIES:
        result["median_fps"] = statistics.median(sample["fps"] for sample in samples)
    if cap_warning:
        return {**result, "verdict": "failed", "reason": cap_warning}
    if row.get("error"):
        return {**result, "verdict": "failed", "reason": row["error"]}
    if row.get("missing_input"):
        return {**result, "verdict": "unverified", "reason": row["missing_input"]}
    if result["median_fps"] is None:
        return {**result, "verdict": "failed", "reason": "No complete measurement"}
    if not isinstance(reference, dict):
        return {**result, "verdict": "unverified", "reason": missing_reason or "Reference case is missing"}
    fps, tolerance = reference.get("reference_fps"), reference.get("tolerance")
    if (not positive_number(fps) or type(tolerance) not in (int, float)
            or not 0 <= tolerance < 1):
        return {**result, "verdict": "unverified", "reason": "Reference FPS or tolerance is invalid"}
    result.update(reference_fps=fps, tolerance=tolerance)
    if reference_compatibility(case, reference.get("compatibility")) != row["compatibility"]:
        return {**result, "verdict": "unverified", "reason": "Reference settings are incompatible"}
    median = result["median_fps"]
    minimum, maximum = fps * (1 - tolerance), fps * (1 + tolerance)
    change = 100 * ((median - fps) / fps)
    result.update(change_percent=change, minimum_fps=minimum, maximum_fps=maximum)
    if minimum <= median <= maximum:
        return {**result, "verdict": "passed"}
    direction = "below" if median < minimum else "above"
    return {**result, "verdict": "failed",
            "reason": f"{abs(change):.3f}% {direction} reference (limit {100 * tolerance:g}%)"}


def compare_all(report, path):
    if (not isinstance(report, dict) or report.get("schema") != SCHEMA
            or not isinstance(report.get("cases"), dict)):
        raise ValueError("Result schema is incompatible")
    cases = {key: report["cases"].get(key, {}) for key in CASES}
    warnings = {key: game_cap_warning(key, cases[key].get("samples", [])) for key in GAME_CASES}
    references, reason = load_reference(path)
    rows = {key: compare_case(key, cases[key], None if warnings.get(key) else references.get(key),
                              reason, cap_warning=warnings.get(key)) for key in CASES}
    return rows, all(row["verdict"] == "passed" for row in rows.values())


def windows_game_window(pid):
    import ctypes
    from ctypes import wintypes

    user32 = ctypes.windll.user32
    callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    user32.EnumWindows.argtypes = (callback_type, wintypes.LPARAM)
    user32.GetWindowThreadProcessId.argtypes = (wintypes.HWND, ctypes.POINTER(wintypes.DWORD))
    user32.IsWindowVisible.argtypes = (wintypes.HWND,)
    windows = []

    @callback_type
    def visible_window(handle, _):
        owner = wintypes.DWORD()
        user32.GetWindowThreadProcessId(handle, ctypes.byref(owner))
        if owner.value == pid and user32.IsWindowVisible(handle):
            windows.append(handle)
            return False
        return True

    user32.EnumWindows(visible_window, 0)
    return next(iter(windows), None)


def request_windows_game_focus(window):
    import ctypes
    from ctypes import wintypes

    user32 = ctypes.windll.user32
    user32.GetForegroundWindow.restype = wintypes.HWND
    user32.SetForegroundWindow.argtypes = (wintypes.HWND,)
    return user32.GetForegroundWindow() == window or bool(user32.SetForegroundWindow(window))


def windows_refresh(window):
    """Get the refresh rate of the monitor hosting this visible desktop window."""
    import ctypes
    from ctypes import wintypes

    class MonitorInfo(ctypes.Structure):
        _fields_ = [("size", wintypes.DWORD), ("monitor", wintypes.RECT),
                    ("work", wintypes.RECT), ("flags", wintypes.DWORD),
                    ("device", wintypes.WCHAR * 32)]

    user32, gdi32 = ctypes.windll.user32, ctypes.windll.gdi32
    user32.MonitorFromWindow.argtypes = (wintypes.HWND, wintypes.DWORD)
    user32.MonitorFromWindow.restype = wintypes.HMONITOR
    user32.GetMonitorInfoW.argtypes = (wintypes.HMONITOR, ctypes.POINTER(MonitorInfo))
    gdi32.CreateDCW.argtypes = (wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.LPVOID)
    gdi32.CreateDCW.restype = wintypes.HDC
    gdi32.GetDeviceCaps.argtypes = (wintypes.HDC, ctypes.c_int)
    gdi32.DeleteDC.argtypes = (wintypes.HDC,)
    monitor = user32.MonitorFromWindow(window, 2)  # MONITOR_DEFAULTTONEAREST
    info = MonitorInfo(size=ctypes.sizeof(MonitorInfo))
    if not monitor or not user32.GetMonitorInfoW(monitor, ctypes.byref(info)):
        return None
    dc = gdi32.CreateDCW("DISPLAY", info.device, None, None)
    if not dc:
        return None
    try:
        rate = gdi32.GetDeviceCaps(dc, 116)  # VREFRESH
        return rate if rate > 1 else None
    finally:
        gdi32.DeleteDC(dc)


def native_game(executable, directory, refresh_override, rom):
    output = directory / "result.json"
    log = directory / "application.log"
    directory.mkdir()
    env = {**os.environ, "AMIGA_GAME_FPS_ROM": str(rom), "AMIGA_GAME_FPS_ADF": str(ADF),
           "AMIGA_GAME_FPS_RESULT": str(output)}
    refresh = refresh_override
    with log.open("wb") as stream, subprocess.Popen(
        [executable], cwd=REPO_ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT,
    ) as process:
        try:
            deadline = time.monotonic() + SAMPLE_TIMEOUT
            if sys.platform == "win32":
                window = None
                startup_deadline = time.monotonic() + 5
                while process.poll() is None and time.monotonic() < startup_deadline:
                    if window is None:
                        window = windows_game_window(process.pid)
                        if window is not None and not request_windows_game_focus(window):
                            print("[INFO] Bring the Amiga Studio Game window to the foreground.",
                                  flush=True)
                    if window is not None:
                        if refresh is None:
                            refresh = windows_refresh(window)
                        if refresh is not None:
                            break
                    time.sleep(0.05)
            code = process.wait(timeout=max(0, deadline - time.monotonic()))
        except subprocess.TimeoutExpired as error:
            process.kill()
            process.wait()
            detail = log.read_text(encoding="utf-8", errors="replace").strip()
            message = f"Native Game did not finish within {SAMPLE_TIMEOUT} seconds."
            raise TimeoutError(f"{message}\n{detail}" if detail else message) from error
        except BaseException:
            process.kill()
            raise
    if code or not output.is_file():
        raise RuntimeError(f"Game application exited {code} without a result\n"
                           f"{log.read_text(encoding='utf-8', errors='replace')}")
    report = json.loads(output.read_text(encoding="utf-8"))
    if report.get("error"):
        return report
    if refresh is None:
        raise ValueError(f"Native display refresh rate is unavailable. Supply {REFRESH_ENV}.")
    report["display_refresh_hz"] = refresh
    return report


def install_browser_media(page, url, rom):
    # Serve the inputs only to this page. The distribution and loopback server
    # directory contain only built assets.
    rom_url = url + "game-fps-input/rom"
    disk_url = url + "game-fps-input/disk"

    try:
        rom_bytes, disk_bytes = rom.read_bytes(), ADF.read_bytes()
    except OSError:
        raise FileNotFoundError("Game boot media became unreadable before browser launch.") from None

    def serve(route, body):
        route.fulfill(body=body, content_type="application/octet-stream",
                      headers={"cache-control": "no-store"})

    page.route(rom_url, lambda route: serve(route, rom_bytes))
    page.route(disk_url, lambda route: serve(route, disk_bytes))
    page.add_init_script("""
        if (window === window.top) {
            const readMedia = async (url, label) => {
                const response = await fetch(url, {cache: 'no-store'});
                if (!response.ok) throw new Error(`Unable to load ${label}`);
                return new Uint8Array(await response.arrayBuffer());
            };
            window.amigaGameFpsMedia = Promise.all([
                readMedia(ROM_URL, 'Kickstart ROM'),
                readMedia(DISK_URL, 'Mixed Scene ADF'),
            ]).then(([rom, disk]) => ({rom, disk}));
        }
    """.replace("ROM_URL", json.dumps(rom_url)).replace("DISK_URL", json.dumps(disk_url)))


def browser_game(url, browser, rom):
    page = browser.new_page(no_viewport=True)
    errors, workers = [], []
    page.on("pageerror", lambda error: errors.append(str(error)))
    page.on("worker", lambda worker: workers.append(worker.url))
    page.add_init_script("""
        const checkFocus = () => {
            if (window.amigaGameFpsArmed && !window.amigaGameFpsResult) {
                window.amigaGameFpsLostFocus = true;
            }
        };
        window.addEventListener('blur', checkFocus);
        document.addEventListener('visibilitychange', () => { if (document.hidden) checkFocus(); });
    """)
    try:
        install_browser_media(page, url, rom)
        page.goto(url, timeout=SAMPLE_TIMEOUT * 1000)
        page.bring_to_front()
        page.wait_for_function("window.amigaGameFpsResult || window.amigaGameFpsLostFocus", polling=250,
                               timeout=SAMPLE_TIMEOUT * 1000)
        if page.evaluate("Boolean(window.amigaGameFpsLostFocus)"):
            raise RuntimeError("Browser Game lost focus")
        result = json.loads(page.evaluate("window.amigaGameFpsResult"))
        if result.get("error"):
            return result
        if errors or not any(url.endswith("/machine_worker.js") for url in workers):
            raise RuntimeError(f"Browser Game worker or page failed. Workers {workers}. Errors {errors}.")
        telemetry = page.evaluate("""async () => {
            const canvas = document.getElementById('amiga_canvas');
            const gl = canvas.getContext('webgl2') ?? canvas.getContext('webgl');
            if (!gl) throw new Error('Game canvas has no WebGL renderer');
            const info = gl.getExtension('WEBGL_debug_renderer_info');
            const renderer = gl.getParameter(info ? info.UNMASKED_RENDERER_WEBGL : gl.RENDERER);
            const intervals = [];
            let previous;
            const read = timestamp => {
                if (previous !== undefined) intervals.push(timestamp - previous);
                previous = timestamp;
            };
            await Promise.race([
                (async () => {
                    for (let frame = 0; frame <= 24; frame++) {
                        read(await new Promise(requestAnimationFrame));
                    }
                })(),
                new Promise((_, reject) => setTimeout(() => reject(new Error('Display cadence timed out')), 3000)),
            ]);
            intervals.sort((a, b) => a - b);
            const median = (intervals[11] + intervals[12]) / 2;
            return {renderer, refresh_hz: Math.round(1000 / median), raf_median_ms: median};
        }""")
        if not positive_number(telemetry["refresh_hz"]):
            raise ValueError("Browser display cadence is unavailable")
        result["renderer"] += f": {telemetry['renderer']}"
        result["display_refresh_hz"] = telemetry["refresh_hz"]
        result["raf_median_ms"] = telemetry["raf_median_ms"]
        return result
    finally:
        page.close()


def measure(row, case, host, directory, settings, native):
    wasm = case in ("FPS-02", "FPS-04")
    game = case in GAME_CASES
    media = {"rom": False, "disk": False}
    if game:
        try:
            media = game_media_identity()
        except FileNotFoundError as error:
            row["missing_input"] = str(error)
            return
    compatibility = {
        "workload": {"sha256": sha256(PROGRAM), "entry": "0x001000", "standard": "PAL",
                     "memory_mib": 1, "model": "A500", **media},
        "host": host,
        "build": build_settings(wasm),
        "sampling": {"series": SERIES, "warmup": WARMUP, **(
            {"seconds": SECONDS, "speed": GAME_SPEED, "metric": "completed-emulated-fields"}
            if game else {"fields": FIELDS}
        )},
    }
    row["compatibility"] = compatibility
    if wasm:
        from playwright.sync_api import sync_playwright

        sys.path.insert(0, str(REPO_ROOT / "crates/gui/tests"))
        from browser_test_server import serve_distribution

        dist = directory / case
        row["artifact_sha256"] = web_build(dist, machine_loop=not game)
        if game:
            compatibility["build"]["features"] = ["game-fps-benchmark"]
        with serve_distribution(dist) as url, sync_playwright() as playwright, chromium(
            playwright, settings.browser_executable,
        ) as browser:
            compatibility["browser"] = chromium_settings(browser)
            if game:
                for _ in range(SERIES):
                    report = browser_game(url, browser, settings.kickstart_rom)
                    record_game_sample(row, report)
            else:
                row["raw"] = browser_bench(url, browser)
    elif game:
        build = command(["cargo", "build", "--locked", "--release", "-p", "gui", "--bin",
                         "amiga-studio", "--features", "game-fps-benchmark",
                         "--message-format=json-render-diagnostics"])
        executable = find_executable(build, "amiga-studio", "bin")
        row["artifact_sha256"] = sha256(Path(executable))
        compatibility["build"]["features"] = ["game-fps-benchmark"]
        for index in range(SERIES):
            report = native_game(executable, directory / f"game-{index}", settings.native_refresh_hz,
                                 settings.kickstart_rom)
            record_game_sample(row, report)
    else:
        row["raw"] = native_bench()
    if game:
        if game_media_identity() != media:
            raise ValueError("The Mixed Scene ADF changed during the measurements")
    else:
        row["samples"] = uncapped_samples(row["raw"])
        if wasm:
            if not native.get("samples"):
                raise ValueError("Native measurement is required to verify matching emulated work")
            verify_matching_work(native["raw"], row["raw"])


def print_report(rows):
    def metric(value, *, signed=False):
        if value is None:
            return "—"
        return f"{value:+.3f}%" if signed else f"{value:.3f}"

    print(f"[INFO] {'Case':<7} {'Median FPS':>11} {'Ref. FPS':>11} {'Change':>10}  Verdict")
    for key, row in rows.items():
        print(f"[INFO] {key:<7} {metric(row['median_fps']):>11} {metric(row['reference_fps']):>11} "
              f"{metric(row['change_percent'], signed=True):>10}  {row['verdict']}")
        if row.get("samples"):
            print("  " + ", ".join(f"{sample['fps']:.3f} FPS ({sample['count']} fields "
                                    f"in {sample['seconds']:.6f}s)"
                                   for sample in row["samples"]))
        if reason := row.get("reason"):
            label = "[WARN]" if reason.startswith("Warning:") else "[INFO]"
            print(f"{label} {reason}")


def private_path_pattern(settings):
    # Child diagnostics may render paths with native separators, forward slashes
    # or escaped backslashes. Keep the surrounding failure details in the report.
    spellings = set()
    for path in filter(None, (settings.kickstart_rom, settings.browser_executable)):
        for variant in (str(path), path.as_posix()):
            spellings.update((variant, json.dumps(variant, ensure_ascii=False)[1:-1]))
    if not spellings:
        return None
    alternatives = "|".join(re.escape(path) for path in sorted(spellings, key=len, reverse=True))
    return re.compile(alternatives, re.IGNORECASE if os.name == "nt" else 0)


def redact_paths(text, pattern):
    return pattern.sub("[private path]", text) if pattern else text


def redact_diagnostics(value, pattern):
    if isinstance(value, list):
        for item in value:
            redact_diagnostics(item, pattern)
    elif isinstance(value, dict):
        for key, item in value.items():
            if key in ("error", "reason", "missing_input") and isinstance(item, str):
                value[key] = redact_paths(item, pattern)
            else:
                redact_diagnostics(item, pattern)


def run_measurement(output_path, settings, private_paths):
    resolved_output = output_path.resolve()
    if resolved_output == REFERENCE.resolve():
        raise ValueError("The report path must differ from the approved reference file.")
    if resolved_output in (PROGRAM.resolve(), ADF.resolve(), settings.kickstart_rom):
        raise ValueError("The report path must differ from the workload and boot media paths.")
    TEMP_ROOT.mkdir(parents=True, exist_ok=True)
    source = source_identity()
    host = host_identity()
    cases = {case: {"name": name} for case, name in CASES.items()}
    with tempfile.TemporaryDirectory(dir=TEMP_ROOT, prefix="run-") as staging:
        for case, row in cases.items():
            print(f"[INFO] Measuring {case} {row['name']}", flush=True)
            try:
                measure(row, case, host, Path(staging), settings, cases["FPS-01"])
            except Exception as error:
                row["error"] = f"{type(error).__name__}: {error}"
        if source_identity() != source:
            for row in cases.values():
                row.setdefault("error", "The source checkout changed during the measurements")
        result = {"schema": SCHEMA, "source": source, "cases": cases}
        rows, passed = compare_all(result, REFERENCE)
    result.update(cases=rows, passed=passed)
    redact_diagnostics(rows, private_paths)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=output_path.parent,
                                     prefix="fps-", suffix=".json", delete=False) as stream:
        json.dump(result, stream, indent=2, allow_nan=False)
        stream.write("\n")
    Path(stream.name).replace(output_path)
    print_report(result["cases"])
    print(f"[INFO] Mixed Scene FPS report: {output_path}", flush=True)
    return 0 if passed else 1


def run(results_dir=None):
    private_paths = None
    try:
        settings = load_settings()
        private_paths = private_path_pattern(settings)
        report_dir = TEMP_ROOT if results_dir is None else Path(results_dir) / "mixed-scene-fps"
        output_path = report_dir / "result.json"
        return run_measurement(output_path, settings, private_paths)
    except (OSError, RuntimeError, ValueError) as error:
        print(f"[ERROR] {redact_paths(str(error), private_paths)}", flush=True)
        return 1
    except KeyboardInterrupt:
        print("[ERROR] Mixed Scene FPS interrupted.", flush=True)
        return 130
