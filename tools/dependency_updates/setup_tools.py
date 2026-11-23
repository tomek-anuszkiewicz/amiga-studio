"""Check the tools declared by project setup documents and callers."""

import ast
from hashlib import sha256
from importlib import metadata
from pathlib import Path
import platform
import re
import shutil
import sys
import tomllib

from .common import DATA_ERRORS, ProbeError, triplet, version_text
from .python_packages import PYPI, packaging_api, pip_index


GITHUB_RELEASES = "https://api.github.com/repos"
DEVELOPER_SETUP = ".agents/references/developer-setup.md"
RUSTUP_RELEASE = "https://static.rust-lang.org/rustup/release-stable.toml"
NEXTEST_BASELINE = (0, 9, 152)
NEXTEST_BASELINE_SOURCE = "tools/verification/README.md at 0fa1472 (historical pin)"


def github_release(probe, repository):
    source = f"{GITHUB_RELEASES}/{repository}/releases/latest"
    data = probe.http(source)
    if data["draft"] or data["prerelease"]:
        raise ProbeError("upstream returned a prerelease or draft")
    return triplet(data["tag_name"]), source


def check_tool_release(area, probe, item, command, repository, setup, *, optional=False):
    observed = None
    installed = None
    source = setup
    try:
        if optional and shutil.which(command[0]) is None:
            area.record("optional missing", item, observed="not on PATH", source=setup)
            return
        installed = triplet(probe.run(command).splitlines()[0])
        observed = version_text(installed)
        source = f"{GITHUB_RELEASES}/{repository}/releases/latest"
        candidate, source = github_release(probe, repository)
        area.compare(item, installed, candidate, source)
    except DATA_ERRORS as error:
        area.incomplete(item, source, error if isinstance(error, ProbeError) else "invalid tool or release data",
                        observed=observed)
    return installed


def check_rustup(area, probe):
    observed = None
    try:
        installed = triplet(probe.run(["rustup", "--version"]))
        observed = version_text(installed)
        candidate = tomllib.loads(probe.http(RUSTUP_RELEASE, "text/plain"))["version"]
        area.compare("rustup", installed, triplet(candidate), RUSTUP_RELEASE)
    except DATA_ERRORS as error:
        area.incomplete("rustup", RUSTUP_RELEASE,
                        error if isinstance(error, ProbeError) else "invalid rustup version or release data",
                        observed=observed)


def rust_components(area, probe, channel, installed, setup):
    try:
        components = set(probe.run(["rustup", "component", "list", "--installed", "--toolchain", channel]).splitlines())
        targets = set(probe.run(["rustup", "target", "list", "--installed", "--toolchain", channel]).splitlines())
        host = installed.split("-", 1)[1]
        required = [("cargo", "cargo", ["cargo", "--version"])]
        if channel == "stable":
            required += [("clippy", "clippy", ["cargo", "clippy", "--version"]),
                         ("rustfmt", "rustfmt", ["rustfmt", "--version"]),
                         ("llvm-tools", "llvm-tools-preview", None)]
        else:
            required += [("rust-src", "rust-src", None)]
        for name, label, command in required:
            item = f"Rust {channel} {label}"
            matches = {name, f"{name}-{host}", label, f"{label}-{host}"} & components
            if not matches:
                area.record("missing", item, observed="not installed", declared=label, source=setup)
                continue
            if command is not None:
                try:
                    probe.run(["rustup", "run", channel, *command])
                except ProbeError as error:
                    area.incomplete(item, setup, error)
                    continue
            area.record("current", item)
        target = "wasm32-unknown-unknown" if channel == "nightly" else host
        if target in targets:
            area.record("current", f"Rust {channel} target {target}")
        else:
            area.record("missing", f"Rust {channel} target {target}", observed="not installed", declared=target,
                        source=setup)
    except DATA_ERRORS as error:
        area.incomplete(f"Rust {channel} components and targets", setup,
                        error if isinstance(error, ProbeError) else "invalid rustup component or target data")


def rust_one_channel(area, probe, channel, installed, setup):
    source = f"https://static.rust-lang.org/dist/channel-rust-{channel}.toml"
    observed = None
    item = f"Rust {channel.title()}"
    try:
        compiler = probe.run(["rustup", "run", installed, "rustc", "--version"])
        known = re.match(r"rustc (\d+\.\d+\.\d+(?:-nightly)?)(?: \([0-9a-f]+ (\d{4}-\d\d-\d\d)\))?", compiler)
        if known:
            observed = known[1] + (f" ({known[2]})" if channel == "nightly" and known[2] else "")
        manifest = tomllib.loads(probe.http(source, "text/plain"))
        remote = manifest["pkg"]["rust"]["version"]
        if channel == "stable":
            pattern = r"(\d+)\.(\d+)\.(\d+)(?: \([^\r\n]+\))?"
            local_match = re.fullmatch(f"rustc {pattern}", compiler)
            remote_match = re.fullmatch(pattern, remote)
            if not local_match or not remote_match:
                raise ProbeError("installed or official Stable metadata did not identify a compiler version")
            area.compare(item, tuple(map(int, local_match.groups())), tuple(map(int, remote_match.groups())), source,
                         context=installed)
        else:
            pattern = r"(\d+\.\d+\.\d+-nightly) \(([0-9a-f]+) (\d{4}-\d\d-\d\d)\)"
            local_match = re.fullmatch(f"rustc {pattern}", compiler)
            remote_match = re.fullmatch(pattern, remote)
            if not local_match or not remote_match:
                raise ProbeError("installed or official Nightly metadata did not identify a compiler date")
            local_version, local_hash, local_date = local_match.groups()
            remote_version, remote_hash, remote_date = remote_match.groups()
            if remote_date > local_date:
                area.record("update", item, observed=f"{local_version} ({local_date})",
                            candidate=f"{remote_version} ({remote_date})", source=source, context=installed)
            elif local_date == remote_date and local_hash != remote_hash:
                raise ProbeError("Nightly compiler builds differ on the same commit date")
            else:
                area.record("current", "Rust Nightly")
    except DATA_ERRORS as error:
        area.incomplete(item, source, error if isinstance(error, ProbeError) else "could not read comparable channel versions",
                        observed=observed, context=installed)
    rust_components(area, probe, channel, installed, setup)


def check_rust(area, probe, _tracked):
    try:
        active = probe.run(["rustup", "show", "active-toolchain"])
        area.notes.append(f"Active Rust toolchain: {active.split()[0]}.")
        area.record("current", "active toolchain identified")
    except (ProbeError, IndexError) as error:
        area.incomplete("active Rust toolchain", DEVELOPER_SETUP,
                        error if isinstance(error, ProbeError) else "rustup did not identify an active toolchain")
    try:
        installed = [row.split()[0] for row in probe.run(["rustup", "toolchain", "list"]).splitlines() if row]
    except ProbeError as error:
        area.incomplete("installed Rust toolchains", DEVELOPER_SETUP, error)
    else:
        for channel, setup in (("stable", "tools/verification/README.md"),
                               ("nightly", "tools/web.ps1 and developer setup")):
            channels = [toolchain for toolchain in installed if toolchain.startswith(channel + "-")
                        and not re.match(r"nightly-\d{4}-\d{2}-\d{2}", toolchain)]
            if not channels:
                area.record("missing", f"Rust {channel.title()}", observed="not installed", declared=channel, source=setup)
            for toolchain in channels:
                rust_one_channel(area, probe, channel, toolchain, setup)
    check_rustup(area, probe)
    check_tool_release(area, probe, "Trunk", ["trunk", "--version"], "trunk-rs/trunk", "tools/web.ps1")
    check_tool_release(area, probe, "cargo-llvm-cov", ["cargo", "llvm-cov", "--version"],
                       "taiki-e/cargo-llvm-cov", "tools/verification/commands.py")
    nextest = check_tool_release(area, probe, "cargo-nextest", ["cargo", "nextest", "--version"],
                                 "nextest-rs/nextest", "tools/verification/commands.py")
    baseline = version_text(NEXTEST_BASELINE)
    if nextest is None:
        area.incomplete("cargo-nextest historical baseline", NEXTEST_BASELINE_SOURCE,
                        "could not read the installed nextest version", declared=baseline)
    elif nextest != NEXTEST_BASELINE:
        area.record("baseline difference", "cargo-nextest", observed=version_text(nextest),
                    declared=baseline, source=NEXTEST_BASELINE_SOURCE,
                    detail=f"historical version {baseline}")
    else:
        area.record("current", "cargo-nextest historical baseline")


def git_version(value):
    match = re.search(r"(\d+)\.(\d+)\.(\d+)[.-]windows[.-](\d+)", value)
    if not match:
        raise ProbeError("Git for Windows did not report a comparable version")
    return tuple(map(int, match.groups()))


def check_git(area, probe):
    observed = None
    try:
        raw = probe.run(["git", "--version"])
        observed = raw.removeprefix("git version ")
        if re.search(r"\.windows\.\d+", observed):
            source = f"{GITHUB_RELEASES}/git-for-windows/git/releases/latest"
            installed = git_version(observed)
            candidate = git_version(probe.http(source)["tag_name"])
            if candidate > installed:
                area.record("update", "Git for Windows", observed=observed, source=source,
                            candidate=f"{candidate[0]}.{candidate[1]}.{candidate[2]}.windows.{candidate[3]}")
            else:
                area.record("current", "Git for Windows")
        elif re.fullmatch(r"git version \d+\.\d+\.\d+", raw):
            source = "https://www.kernel.org/pub/software/scm/git/"
            page = probe.http(source, "text/plain")
            versions = [triplet(version) for version in re.findall(r'href="git-(\d+\.\d+\.\d+)\.tar\.xz"', page)]
            if not versions:
                raise ProbeError("Git's official release listing contained no stable versions")
            area.compare("Git", triplet(observed), max(versions), source)
        else:
            raise ProbeError("this Git build is distributed through a separately managed channel")
    except DATA_ERRORS as error:
        area.incomplete("Git", DEVELOPER_SETUP,
                        error if isinstance(error, ProbeError) else "invalid Git version or release data",
                        observed=observed)


def check_powershell(area, probe):
    setup = "tools/web.ps1 and developer setup"
    if sys.platform == "win32":
        try:
            version = probe.run(["powershell", "-NoProfile", "-NonInteractive",
                                 "-Command", "$PSVersionTable.PSVersion.ToString()"])
            if not re.fullmatch(r"\d+\.\d+(?:\.\d+){0,2}", version):
                raise ProbeError("the executable did not report a version")
            area.record("current", "Windows PowerShell availability")
        except ProbeError as error:
            area.incomplete("Windows PowerShell", setup, error)
    if sys.platform != "win32" or shutil.which("pwsh") is not None:
        check_tool_release(area, probe, "PowerShell (pwsh)", ["pwsh", "-NoProfile", "-NonInteractive", "--version"],
                           "PowerShell/PowerShell", setup)


def graphify_pin(probe):
    path = probe.root / "tools/graphify/refresh.py"
    source = ast.parse(path.read_text(encoding="utf-8"))
    for node in source.body:
        if isinstance(node, ast.Assign) and any(isinstance(target, ast.Name) and target.id == "GRAPHIFY_VERSION"
                                               for target in node.targets):
            value = ast.literal_eval(node.value)
            if isinstance(value, str):
                return value
    raise ProbeError("the refresh implementation did not expose a Graphify version")


def check_graphify(area, probe):
    source = "tools/graphify/refresh.py"
    pin = None
    observed = None
    try:
        pin = graphify_pin(probe)
        _, _, Version = packaging_api()
        try:
            observed = metadata.version("graphifyy")
            environment = "selected Python environment"
        except metadata.PackageNotFoundError:
            environment = "installed uv tool environment"
            try:
                directory = Path(probe.run(["uv", "tool", "dir"])) / "graphifyy"
                filename = "Scripts/python.exe" if sys.platform == "win32" else "bin/python"
                interpreter = directory / filename
                if interpreter.is_file():
                    observed = probe.run([str(interpreter), "-B", "-c",
                                          "from importlib.metadata import version; print(version('graphifyy'))"])
            except ProbeError as error:
                area.incomplete("Graphify environment", source, error, declared=pin)
        if observed is None:
            area.record("optional missing", "Graphify", observed="not installed in either environment",
                        declared=pin, source=source)
        elif Version(observed) != Version(pin):
            area.record("drift", "Graphify", observed=observed, declared=pin, source=source,
                        detail=environment)
        else:
            area.record("current", "Graphify installed pin")
        candidate = pip_index(probe, "graphifyy", Version)[0]
        if candidate > Version(pin):
            use_pin = observed is None or Version(observed) != Version(pin)
            area.record("pin review", "Graphify", observed=pin if use_pin else observed, declared=pin,
                        candidate=str(candidate), source=PYPI, detail="project pin" if use_pin else None)
        else:
            area.record("current", "Graphify PyPI release")
    except (*DATA_ERRORS, SyntaxError) as error:
        area.incomplete("Graphify", source, error if isinstance(error, ProbeError) else "invalid Graphify data",
                        observed=observed, declared=pin)


def check_assembler(area, probe):
    source = "tools/bootstrap/assembler-manifest.json"
    installed = None
    pin = None
    try:
        manifest = probe.json(source)
        pin = manifest["release"]
        target = {"win32": "x86_64-pc-windows-msvc", "linux": "x86_64-unknown-linux-gnu",
                  "darwin": "x86_64-apple-darwin"}.get(sys.platform)
        if platform.machine().lower() not in {"amd64", "x86_64"} or target not in manifest["platforms"]:
            area.incomplete("m68k-asm installation", source, "no supported assembler for this host", declared=pin)
        else:
            platform_info = manifest["platforms"][target]
            directory = probe.root / "tools/m68k-asm" / pin / target
            executable = directory / platform_info["executable_name"]
            if not executable.is_file():
                area.record("optional missing", "m68k-asm", observed="not present at the pinned path",
                            declared=pin, source=source, detail="CPU performance tests")
            elif sha256(executable.read_bytes()).hexdigest() != platform_info["executable_sha256"].lower():
                area.record("drift", "m68k-asm", observed="installed executable hash differs",
                            declared=pin, source=source)
            else:
                installed = pin
                area.record("current", "m68k-asm pinned installation")
        candidate, upstream = github_release(probe, "Attackwave/m68k")
        area.compare("m68k-asm", triplet(pin), candidate, upstream, declared=pin, category="pin review",
                     observed_text=installed or f"{version_text(triplet(pin))} (project pin)")
    except DATA_ERRORS as error:
        area.incomplete("m68k-asm", source, error if isinstance(error, ProbeError) else "invalid assembler data",
                        observed=installed, declared=pin)


def check_setup(area, probe, _tracked):
    check_git(area, probe)
    check_powershell(area, probe)
    check_tool_release(area, probe, "uv", ["uv", "--version"], "astral-sh/uv", "tools/graphify/refresh.py",
                       optional=True)
    check_graphify(area, probe)
    check_assembler(area, probe)
