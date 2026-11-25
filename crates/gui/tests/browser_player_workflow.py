"""Exercise manual WASM setup, file save/import and disk swaps in Chromium.

Build with tools/web.ps1, then provide local ROM/ADF files and an installed
Chromium executable. The test serves only the built distribution on loopback.
File selection exercises the ordinary HTML input fallback; physical controls
and File System Access permission dialogs remain interactive checks.

The media case exports a paused browser state and generated inputs into the
output's media directory. Supply --native-state with the native re-export from
that directory to run the return trip in a fresh Chromium context.
"""
import argparse
from base64 import b64decode, b64encode
from functools import partial
from hashlib import sha256
from io import BytesIO
import json
from pathlib import Path
from struct import pack
from urllib.parse import urlparse
from zipfile import ZIP_DEFLATED, ZipFile

from playwright.sync_api import sync_playwright

from browser_test_server import serve_distribution


# eframe renders these controls into one canvas without individual browser
# accessibility nodes. Use control centers in the fixed viewport at 100% zoom.
VIEWPORT = {"width": 1280, "height": 800}
APPLICATION_TITLE = "Amiga 500 Developer Studio"
CONTROL_POINTS = {
    "Select Kickstart": (65, 108),
    "Select disk": (204, 131),
    "Start": (30, 223),
    "Game Save": (1090, 218),
    "Game Load": (1142, 218),
    "Game Manual setup": (98, 17),
    "Game Controls": (1096, 42),
    "Game Instructions": (1168, 42),
    "Configurations": (161, 12),
    "Configuration Name": (148, 149),
    "Configuration Cancel": (86, 788),
    "Pause / Resume binding": (122, 600),
    "Amiga port 2 device": (58, 462),
    "Amiga port 2 Disconnected": (62, 489),
    "Configuration Save": (29, 788),
    "Return to session": (278, 12),
    "DF0 dropdown": (1198, 71),
    "DF0 Eject / Empty": (1198, 98),
    "DF0 first disk": (1198, 121),
    "DF0 second disk": (1198, 144),
}

TRACK_BYTES = 12_500


def disk_reference(snapshot):
    return snapshot["state"]["floppy"]["drives"][0]["track_data"]


def decode_base64(value):
    assert isinstance(value, str)
    decoded = b64decode(value, validate=True)
    assert b64encode(decoded).decode("ascii") == value, "Non-canonical Base64"
    return decoded


def check_standard_encoding(page, snapshot):
    """Decode the real download independently of the Rust or Wasm codec."""
    assert snapshot["header"]["version"] == 16
    assert snapshot["header"]["timestamp"] > 0
    memory = snapshot["state"]["physical_memory"]
    assert len(decode_base64(memory["chip_ram"])) == 512 * 1024
    assert len(decode_base64(memory["slow_ram"])) == 512 * 1024
    assert memory["fast_ram"] is None
    frame = snapshot["state"]["denise"]["frame_builder"]["buffer"]
    assert len(decode_base64(frame)) == 720 * 568 * 4
    preview = snapshot["header"]["screenshot_png"]
    assert decode_base64(preview).startswith(b"\x89PNG\r\n\x1a\n")
    result = page.evaluate("""async ({preview, frame}) => {
        const decode = data => Uint8Array.from(atob(data), char => char.charCodeAt(0));
        const bitmap = await createImageBitmap(new Blob([decode(preview)], {type: "image/png"}));
        const {width, height} = bitmap;
        if (width !== 720 || height !== 568) return {width, height};
        const canvas = new OffscreenCanvas(width, height);
        const context = canvas.getContext("2d");
        context.drawImage(bitmap, 0, 0);
        bitmap.close();
        const rgba = context.getImageData(0, 0, width, height).data;
        const words = new DataView(decode(frame).buffer);
        for (let offset = 0; offset < rgba.length; offset += 4) {
            const word = words.getUint32(offset, true);
            if (rgba[offset] !== ((word >>> 16) & 255) ||
                rgba[offset + 1] !== ((word >>> 8) & 255) ||
                rgba[offset + 2] !== (word & 255) ||
                rgba[offset + 3] !== 255 || (word >>> 24) !== 255) {
                return {width, height, firstDifferentPixel: offset / 4};
            }
        }
        return {width, height};
    }""", {"preview": preview, "frame": frame})
    assert result == {"width": 720, "height": 568}, result


def check_source(snapshot, rom, disk, mounted_id):
    assert snapshot["header"]["association"] == {
        "game_id": None, "mounted": [mounted_id, None, None, None],
    }
    rom_reference = snapshot["state"]["physical_memory"]["kickstart_rom"]
    assert rom_reference == {
        "sha256": sha256(rom.read_bytes()).hexdigest(), "name": rom.name, "synthetic": False,
    }
    drives = snapshot["state"]["floppy"]["drives"]
    assert len(drives) == 4
    assert drives[0]["connected"] and drives[0]["disk_inserted"]
    reference = disk_reference(snapshot)
    assert reference["name"] == disk.name
    assert reference["sha256"] == sha256(disk.read_bytes()).hexdigest()
    assert set(reference) == {"name", "sha256", "modified_tracks"}
    for drive in drives[1:]:
        assert not drive["connected"] and not drive["disk_inserted"] and drive["track_data"] is None


def check_restored_state(actual, expected):
    assert actual["state"]["cck"] == expected["state"]["cck"], "Load changed the paused CCK"
    assert actual["state"]["cpu"] == expected["state"]["cpu"], "Load changed the paused CPU"
    assert actual["state"] == expected["state"], "Load changed the paused machine"
    assert actual["header"]["screenshot_png"] == expected["header"]["screenshot_png"]


def player_context(browser):
    context = browser.new_context(viewport=VIEWPORT, device_scale_factor=1, accept_downloads=True)

    def observe_generation(route):
        response = route.fetch()
        source = response.text()
        marker = "export function set_host_input_enabled(mouse, gamepads, interval_ms, generation) {"
        assert source.count(marker) == 1
        observation = "\n    globalThis.observedGeneration = Number(generation);"
        route.fulfill(response=response, body=source.replace(marker, marker + observation))

    context.route("**/host_bridge.js", observe_generation)
    context.add_init_script("""
        window.showOpenFilePicker = undefined;
        const OriginalWorker = Worker;
        window.workerReport = {outputs:0, failures:[]};
        window.Worker = class extends OriginalWorker {
            constructor(url, options) {
                super(url, options);
                this.addEventListener('message', event => {
                    if (event.data.type === 'output') workerReport.outputs++;
                    if (event.data.type === 'failed') workerReport.failures.push(event.data.message);
                });
            }
        };
        Object.defineProperty(navigator.clipboard, "writeText", {
            value: async text => { globalThis.copiedText = text; }
        });
    """)
    return context


def click(page, control):
    x, y = CONTROL_POINTS[control]
    # Let the canvas consume hover and the pressed state on separate frames.
    page.mouse.move(x, y)
    page.wait_for_timeout(120)
    page.mouse.click(x, y, delay=80)
    page.wait_for_timeout(100)


def select_file(page, control, path):
    with page.expect_file_chooser() as chooser:
        click(page, control)
    chooser.value.set_files(path)


def export_state(page, path):
    with page.expect_download(timeout=30000) as download:
        click(page, "Game Save")
    download.value.save_as(path)
    return json.loads(path.read_text(encoding="utf-8"))


def wait_generation(page, generation):
    page.wait_for_function("generation => globalThis.observedGeneration === generation",
                           arg=generation, timeout=30000)


def import_state(page, path, generation):
    select_file(page, "Game Load", path)
    wait_generation(page, generation)
    page.wait_for_timeout(300)


def drive_label(page):
    # Sample the DF0 text, leaving out the activity indicator and hover.
    _, y = CONTROL_POINTS["DF0 dropdown"]
    page.mouse.move(1040, y)
    return page.screenshot(clip={"x": 1128, "y": y - 7, "width": 108, "height": 14})


def select_disk(page, option, description):
    before = drive_label(page)
    click(page, "DF0 dropdown")
    click(page, option)
    for _ in range(20):
        after = drive_label(page)
        if after != before:
            return after
        page.wait_for_timeout(100)
    raise AssertionError(f"DF0 did not confirm {description}")


def check_remount(page, snapshot, output):
    # Seed a single altered raw track in a disposable file. The last 16 bytes
    # are in the track gap and cannot be represented by a sector write.
    reference = disk_reference(snapshot)
    assert reference["modified_tracks"] == {}
    altered_track = bytes([0xAA]) * (TRACK_BYTES - 16) + bytes.fromhex("deadbeef") * 4
    reference["modified_tracks"]["0"] = b64encode(altered_track).decode("ascii")
    fixture = output / "modified-track-fixture.json"
    fixture.write_text(json.dumps(snapshot, separators=(",", ":")), encoding="utf-8")
    import_state(page, fixture, 6)
    mounted_label = drive_label(page)
    replacement_label = select_disk(page, "DF0 first disk", "the replacement disk")
    empty_label = select_disk(page, "DF0 Eject / Empty", "ejection")
    assert len({mounted_label, replacement_label, empty_label}) == 3
    assert select_disk(page, "DF0 second disk", "reinsertion") == mounted_label
    assert page.evaluate("observedGeneration") == 6, "Disk swap replaced the machine"

    # Observe the track after all three disk commands have completed.
    remounted = export_state(page, output / "modified-track-remounted.json")
    assert remounted["header"]["association"]["mounted"][0] == "manual-1"
    assert remounted["state"]["floppy"]["drives"][0]["disk_inserted"]
    restored = disk_reference(remounted)
    assert restored["sha256"] == reference["sha256"]
    assert list(restored["modified_tracks"]) == ["0"]
    assert decode_base64(restored["modified_tracks"]["0"]) == altered_track


def check_workflow(browser, url, rom, disk, output):
    context = player_context(browser)
    page = context.new_page()
    errors = []
    page.on("pageerror", lambda error: errors.append(str(error)))
    press = partial(click, page)
    save = partial(export_state, page)
    load = partial(import_state, page)

    stage = "startup"
    try:
        with page.expect_response(
            lambda response: urlparse(response.url).path.endswith("/gui_bg.wasm")
        ) as wasm:
            page.goto(url)
        page.wait_for_function("globalThis.observedGeneration === 1", timeout=30000)
        assert page.title() == APPLICATION_TITLE
        page.wait_for_timeout(800)
        counts = []
        for _ in range(3):
            page.wait_for_timeout(1100)
            counts.append(page.evaluate("workerReport.outputs"))
        assert len(set(counts)) == 1, f"Paused notifications continued without input: {counts}"
        page.screenshot(path=str(output / "setup.png"))
        stage = "ROM selection"
        select_file(page, "Select Kickstart", rom)
        page.wait_for_timeout(400)
        stage = "disk selection"
        disk_digest = sha256(disk.read_bytes()).digest()
        with page.expect_file_chooser() as chooser:
            press("Select disk")
        chooser.value.set_files({
            "name": "replacement.adf" if disk.name != "replacement.adf" else "other.adf",
            "mimeType": "application/octet-stream",
            "buffer": bytes([255]) * (880 * 1024),
        })
        page.wait_for_timeout(400)
        select_file(page, "Select disk", disk)
        page.wait_for_timeout(400)
        page.screenshot(path=str(output / "ready.png"))
        stage = "manual start"
        page.evaluate("document.title = 'Before manual start'")
        press("Start")
        page.wait_for_function("globalThis.observedGeneration === 2", timeout=30000)
        page.wait_for_timeout(600)
        assert page.title() == APPLICATION_TITLE
        page.screenshot(path=str(output / "game.png"))
        stage = "browser toolbar eligibility"
        # The canvas toolbar exposes no DOM or AccessKit nodes. Check the loaded
        # Wasm for a shared control and the native button's application-specific ID.
        application = wasm.value.body()
        assert b"Step Frame" in application
        assert b"Debug View" not in application
        assert b"Amiga 500 player guide" in application
        for retired in (
            b"Load Binary from File", b"Load Binary into Memory",
            b"Recent binaries", b"Load ROM from File",
        ):
            assert retired not in application, f"Retired application control found in Wasm: {retired!r}"
        assert b"Select Kickstart for Setup" in application
        assert b"amiga-game-screenshot-button" not in application, (
            "Native screenshot button ID found in browser Wasm"
        )
        assert b"amiga-game-cover-button" not in application, (
            "Native cover button ID found in browser Wasm"
        )
        stage = "save export"
        saved_path = output / "exported.json"
        saved = save(saved_path)
        check_standard_encoding(page, saved)
        check_source(saved, rom, disk, "manual-1")
        assert disk_reference(saved)["modified_tracks"] == {}
        assert saved["state"]["cck"] > 0
        stage = "state import"
        page.evaluate("document.title = 'Before state restore'")
        load(saved_path, 3)
        assert page.title() == APPLICATION_TITLE
        restored = save(output / "restored.json")
        check_restored_state(restored, saved)
        check_source(restored, rom, disk, "manual-1")
        page.screenshot(path=str(output / "restored.png"))
        stage = "retired program inputs"
        unexpected_pickers = []

        def reject_program_picker(chooser):
            unexpected_pickers.append("file chooser")
            chooser.set_files({
                "name": "retired.bin",
                "mimeType": "application/octet-stream",
                "buffer": bytes([0x60, 0xFE]),
            })

        page.on("filechooser", reject_program_picker)
        try:
            page.keyboard.press("Control+o")
            page.evaluate('''() => {
                const transfer = new DataTransfer();
                transfer.items.add(new File([new Uint8Array([0x60, 0xfe])], "retired.bin"));
                document.querySelector("canvas").dispatchEvent(new DragEvent("drop", {
                    dataTransfer: transfer, bubbles: true, cancelable: true,
                }));
            }''')
            page.wait_for_timeout(400)
        finally:
            page.remove_listener("filechooser", reject_program_picker)
        assert not unexpected_pickers, "Retired program shortcut opened a picker"
        assert page.evaluate("observedGeneration") == 3
        after_retired_inputs = save(output / "after-retired-inputs.json")
        assert after_retired_inputs["state"] == restored["state"], "Retired program inputs changed the machine"
        stage = "offline instructions"
        panel = {"x": VIEWPORT["width"] - 219, "y": 56, "width": 219, "height": 325}
        controls = page.screenshot(clip=panel)
        context.set_offline(True)
        try:
            press("Game Instructions")
            page.wait_for_timeout(300)
            assert page.screenshot(clip=panel) != controls, "Instructions did not change the player panel"
            page.screenshot(path=str(output / "instructions-offline.png"))
            assert page.evaluate("observedGeneration") == 3
            press("Game Controls")
            page.wait_for_timeout(300)
            assert page.evaluate("observedGeneration") == 3
        finally:
            context.set_offline(False)
        after_instructions = save(output / "after-instructions.json")
        assert after_instructions["state"] == restored["state"], "Opening Instructions changed the paused machine"
        stage = "configuration Cancel"
        stored_before_edit = page.evaluate("localStorage.getItem('player_configurations')")
        press("Game Manual setup")  # The existing machine remains paused.
        page.wait_for_timeout(300)
        press("Configurations")
        page.wait_for_timeout(300)
        page.screenshot(path=str(output / "configuration.png"))
        press("Configuration Name")
        page.keyboard.press("Control+a")
        page.keyboard.type("Cancelled edit")
        press("Configuration Cancel")
        page.wait_for_timeout(400)
        assert page.evaluate("localStorage.getItem('player_configurations')") == stored_before_edit
        assert page.evaluate("observedGeneration") == 3, "Cancel changed the machine generation"
        stage = "record browser keyboard shortcut"
        press("Configurations")
        page.wait_for_timeout(300)
        page.screenshot(path=str(output / "shortcut-editor.png"))
        press("Pause / Resume binding")  # Initially F5.
        page.screenshot(path=str(output / "shortcut-recording.png"))
        page.keyboard.press("F7")
        page.wait_for_timeout(300)
        stage = "configuration Save and changed-port return"
        press("Amiga port 2 device")
        page.wait_for_timeout(200)
        page.screenshot(path=str(output / "port-options.png"))
        press("Amiga port 2 Disconnected")
        press("Configuration Save")
        page.wait_for_function("globalThis.observedGeneration === 4", timeout=30000)
        page.wait_for_timeout(300)
        press("Return to session")
        page.wait_for_timeout(300)
        changed = save(output / "changed-ports.json")
        expected = json.loads(json.dumps(saved["state"]))
        expected["game_ports"]["state"]["port2"] = "None"
        assert changed["state"] == expected, "Stopped controls save changed machine progress or receivers"
        stage = "load older endpoint types with current controls"
        load(saved_path, 5)
        current = save(output / "current-controls-load.json")
        assert current["state"] == expected, "Import restored historical attachments or changed progress"
        page.screenshot(path=str(output / "current-controls-load.png"))
        stage = "swap and remount changed disk with a source ADF"
        check_remount(page, current, output)
        assert sha256(disk.read_bytes()).digest() == disk_digest, "The original ADF changed"
        stage = "dispatch rebound browser shortcut"
        stored = page.evaluate("localStorage.getItem('player_configurations')")
        assert stored and 'pause:"F7"' in stored, "Recorded key was not published"
        paused = save(output / "before-rebound-shortcut.json")
        updates = page.evaluate("workerReport.outputs")
        page.keyboard.press("F7")
        page.wait_for_function("updates => workerReport.outputs > updates + 3", arg=updates)
        page.keyboard.press("F7")
        page.wait_for_timeout(300)
        advanced = save(output / "after-rebound-shortcut.json")
        assert advanced["state"]["cck"] > paused["state"]["cck"]
        page.wait_for_timeout(300)
        stopped = page.evaluate("workerReport.outputs")
        page.wait_for_timeout(500)
        assert page.evaluate("workerReport.outputs") == stopped, "F7 did not pause"

        stage = "configuration persistence"
        assert rom.name in stored, "Selected configuration was not persisted before reopening"
        page.reload()
        page.wait_for_function("globalThis.observedGeneration === 1", timeout=30000)
        page.wait_for_timeout(500)
        page.screenshot(path=str(output / "reopened.png"))
        assert page.evaluate("localStorage.getItem('player_configurations')") == stored
        assert not errors, errors
        assert not page.evaluate("workerReport.failures")
        return {"browser": browser.version, "paused_notifications": counts,
                "saved_cck": saved["state"]["cck"], "restored_cck": restored["state"]["cck"],
                "schema": saved["header"]["version"], "current_ports": current["state"]["game_ports"]["state"]["port2"],
                "configuration_cancel": True, "stopped_configuration_save": True,
                "retired_program_inputs_ignored": True,
                "recorded_keyboard_shortcut": True,
                "disk_swap_retained_raw_tracks": True,
                "manual_setup_and_restore_title": APPLICATION_TITLE, "offline_instructions": True,
                "native_screenshot_action_excluded": True,
                "native_cover_action_excluded": True, "errors": errors}
    except Exception:
        page.screenshot(path=str(output / "failure.png"))
        print(f"Failed browser workflow stage: {stage}")
        print(json.dumps({"errors": errors, "worker": page.evaluate("workerReport")}, indent=2))
        raise
    finally:
        context.close()


def create_media_fixtures(directory):
    rom = bytearray(256 * 1024)
    rom[:8] = pack(">II", 0x7F000, 0xF80100)
    # Set D0 to $2A, set COLOR00 to RGB $19E, then loop at the branch.
    rom[0x100:0x10C] = bytes.fromhex("702a 33fc 019e 00dff180 60fe")
    files = {"test.rom": rom, "first.adf": bytes([0x31]) * (880 * 1024),
             "second.adf": bytes([0xA6]) * (880 * 1024)}
    for name, contents in files.items():
        path = directory / name
        if path.exists():
            assert path.read_bytes() == contents, f"Fixture contents differ at {path}"
        else:
            path.write_bytes(contents)
    return tuple(directory / name for name in files)


def start_page(context, url, errors):
    page = context.new_page()
    page.on("pageerror", lambda error: errors.append(str(error)))
    page.goto(url)
    wait_generation(page, 1)
    page.wait_for_timeout(500)
    return page


def setup_files(page, rom, disks):
    select_file(page, "Select Kickstart", rom)
    page.wait_for_timeout(400)
    for disk in disks:
        select_file(page, "Select disk", disk)
        page.wait_for_timeout(400)


def reject_import(page, saved_path, expected_text, screenshot):
    select_file(page, "Game Load", saved_path)
    # The error window is rendered into the canvas. Locate its red error label in
    # the centered window, then read the same selectable text the player sees.
    clip = {"x": 360, "y": 350, "width": 560, "height": 110}
    bounds = None
    for _ in range(40):
        encoded = b64encode(page.screenshot(clip=clip)).decode("ascii")
        bounds = page.evaluate("""async encoded => {
            const bytes = Uint8Array.from(atob(encoded), char => char.charCodeAt(0));
            const bitmap = await createImageBitmap(new Blob([bytes], {type: "image/png"}));
            const {width, height} = bitmap;
            const canvas = new OffscreenCanvas(width, height);
            const context = canvas.getContext("2d");
            context.drawImage(bitmap, 0, 0);
            bitmap.close();
            const pixels = context.getImageData(0, 0, width, height).data;
            let left = width, top = height, bottom = 0;
            for (let y = 0; y < height; y++) {
                for (let x = 0; x < width; x++) {
                    const i = (y * width + x) * 4;
                    const [red, green, blue] = pixels.subarray(i, i + 3);
                    if (red >= 125 && red > 2 * green && red > 2 * blue) {
                        left = Math.min(left, x);
                        top = Math.min(top, y);
                        bottom = Math.max(bottom, y);
                    }
                }
            }
            return left === width ? null : {left, top, bottom};
        }""", encoded)
        if bounds:
            break
        page.wait_for_timeout(100)
    page.screenshot(path=str(screenshot))
    assert bounds, f"No visible error for {saved_path.name}"
    x = clip["x"] + bounds["left"]
    y = clip["y"] + bounds["top"]
    page.evaluate("globalThis.copiedText = ''")
    page.mouse.click(x + 12, y + 4, click_count=3)
    page.wait_for_timeout(120)
    page.keyboard.press("Control+c")
    page.wait_for_function("globalThis.copiedText.length > 0", timeout=5000)
    text = page.evaluate("globalThis.copiedText")
    assert expected_text in " ".join(text.split()), text
    # Close sits immediately below the displayed error label.
    page.mouse.click(x + 18, clip["y"] + bounds["bottom"] + 14, delay=80)
    page.wait_for_timeout(250)
    return text


def check_rejected_imports(browser, url, rom, disk, saved_path, output):
    context = player_context(browser)
    errors = []
    page = start_page(context, url, errors)
    try:
        setup_files(page, rom, ())
        click(page, "Return to session")
        before = export_state(page, output / "before-rejected-loads.json")
        assert before["header"]["association"]["mounted"] == [None] * 4
        missing = reject_import(page, saved_path, f"DF0 ({disk.name}) source is unavailable",
                                output / "missing-adf.png")
        assert page.evaluate("observedGeneration") == 1
        after = export_state(page, output / "after-missing-adf.json")
        check_restored_state(after, before)
        assert after["header"]["association"] == before["header"]["association"]

        click(page, "Game Manual setup")
        select_file(page, "Select disk", {
            "name": disk.name,
            "mimeType": "application/octet-stream",
            "buffer": bytes([0xEE]) * (880 * 1024),
        })
        page.wait_for_timeout(400)
        click(page, "Return to session")
        wrong = reject_import(page, saved_path, f"DF0 ({disk.name}) content does not match",
                              output / "wrong-adf.png")
        assert page.evaluate("observedGeneration") == 1
        after = export_state(page, output / "after-wrong-adf.json")
        check_restored_state(after, before)
        assert after["header"]["association"] == before["header"]["association"]
        assert not errors and not page.evaluate("workerReport.failures"), errors
        return {"missing_adf": missing, "wrong_adf": wrong, "cck": before["state"]["cck"],
                "generation": 1}
    finally:
        context.close()


def check_fresh_restore(browser, url, rom, first, second, saved_path, output):
    context = player_context(browser)
    errors = []
    page = start_page(context, url, errors)
    try:
        expected = json.loads(saved_path.read_text(encoding="utf-8"))
        setup_files(page, rom, (second, first))
        # First is currently selected in setup. Load must restore second,
        # using its new page-local manual-0 ID, without choosing Start.
        click(page, "Return to session")
        assert page.evaluate("observedGeneration") == 1
        import_state(page, saved_path, 2)
        restored = export_state(page, output / "fresh-restored.json")
        check_standard_encoding(page, restored)
        check_restored_state(restored, expected)
        check_source(restored, rom, second, "manual-0")

        restored_label = drive_label(page)
        swapped_label = select_disk(page, "DF0 second disk", "the first ADF")
        swapped = export_state(page, output / "fresh-swapped.json")
        check_source(swapped, rom, first, "manual-1")
        empty_label = select_disk(page, "DF0 Eject / Empty", "ejection")
        assert len({restored_label, swapped_label, empty_label}) == 3
        empty = export_state(page, output / "fresh-ejected.json")
        assert empty["header"]["association"]["mounted"] == [None] * 4
        assert not empty["state"]["floppy"]["drives"][0]["disk_inserted"]
        assert disk_reference(empty) is None
        assert select_disk(page, "DF0 first disk", "the restored ADF") == restored_label
        reinserted = export_state(page, output / "fresh-reinserted.json")
        check_source(reinserted, rom, second, "manual-0")
        for snapshot in (swapped, empty, reinserted):
            assert snapshot["state"]["cck"] == restored["state"]["cck"]
        assert page.evaluate("observedGeneration") == 2
        assert not errors and not page.evaluate("workerReport.failures"), errors
        return {"cck": restored["state"]["cck"], "generation": 2,
                "restored_drive_id": "manual-0", "swapped_drive_id": "manual-1",
                "ejected": True, "reinserted_drive_id": "manual-0"}
    except Exception:
        page.screenshot(path=str(output / "fresh-restore-failure.png"))
        raise
    finally:
        context.close()


def check_media_restore(browser, url, output):
    output.mkdir(parents=True, exist_ok=True)
    rom, first, second = create_media_fixtures(output)
    original_digests = {path: sha256(path.read_bytes()).digest() for path in (rom, first, second)}
    saved_path = output / "browser.json"
    context = player_context(browser)
    errors = []
    page = start_page(context, url, errors)
    try:
        setup_files(page, rom, (first, second))
        updates = page.evaluate("workerReport.outputs")
        click(page, "Start")
        wait_generation(page, 2)
        page.wait_for_function("updates => workerReport.outputs > updates + 3",
                               arg=updates, timeout=30000)
        page.keyboard.press("F5")
        page.wait_for_timeout(400)
        saved = export_state(page, saved_path)
        check_standard_encoding(page, saved)
        check_source(saved, rom, second, "manual-1")
        assert disk_reference(saved)["modified_tracks"] == {}
        assert saved["state"]["cck"] > 0
        assert saved["state"]["cpu"]["d"][0] == 0x2A
        frame = decode_base64(saved["state"]["denise"]["frame_builder"]["buffer"])
        assert pack("<I", 0xFF1199EE) in frame
        check_restored_state(export_state(page, output / "browser-still-paused.json"), saved)
        assert not errors and not page.evaluate("workerReport.failures"), errors
    except Exception:
        page.screenshot(path=str(output / "paused-export-failure.png"))
        raise
    finally:
        context.close()

    result = check_rejected_imports(browser, url, rom, second, saved_path, output)
    result["fresh_page"] = check_fresh_restore(browser, url, rom, first, second, saved_path, output)
    for path, digest in original_digests.items():
        assert sha256(path.read_bytes()).digest() == digest, f"Media changed at {path}"
    return result


def check_native_exchange(browser, url, native_path, output):
    original = native_path.with_name("browser.json")
    rom, first, second = (native_path.with_name(name) for name in ("test.rom", "first.adf", "second.adf"))
    browser_save = json.loads(original.read_text(encoding="utf-8"))
    native_save = json.loads(native_path.read_text(encoding="utf-8"))
    check_restored_state(native_save, browser_save)
    check_source(native_save, rom, second, "manual-1")
    return check_fresh_restore(browser, url, rom, first, second, native_path, output)


def check_archive_imports(browser, url, rom, output):
    """Boot the same ADF from three formats and verify its original content."""
    expected = bytes([0x33]) * (880 * 1024)
    zipped = BytesIO()
    with ZipFile(zipped, "w", compression=ZIP_DEFLATED) as archive:
        archive.writestr("disks/Archive Fill.adf", expected)
        archive.writestr("readme.txt", "Ignore this member")
    seven_z = Path(__file__).parent / "fixtures/disk-import/single-disk.7z"
    results = {}
    for filename, contents in (
        ("Archive Fill.adf", expected),
        ("Browser ZIP.zip", zipped.getvalue()),
        ("Browser 7z.7z", seven_z.read_bytes()),
    ):
        context = browser.new_context(viewport=VIEWPORT, device_scale_factor=1, accept_downloads=True)
        page = context.new_page()
        errors = []
        page.on("pageerror", lambda error: errors.append(str(error)))
        page.add_init_script("""
            window.showOpenFilePicker = undefined;
            const OriginalWorker = Worker;
            globalThis.archiveWorker = {outputs: 0, failures: []};
            window.Worker = class extends OriginalWorker {
                constructor(url, options) {
                    super(url, options);
                    this.addEventListener("message", event => {
                        if (event.data.type === "output") archiveWorker.outputs++;
                        if (event.data.type === "failed") archiveWorker.failures.push(event.data.message);
                    });
                }
            };
        """)
        try:
            page.goto(url)
            page.wait_for_function("globalThis.archiveWorker.outputs > 0", timeout=30000)
            page.wait_for_timeout(500)

            def click(control):
                x, y = CONTROL_POINTS[control]
                page.mouse.move(x, y)
                page.wait_for_timeout(120)
                page.mouse.click(x, y, delay=80)
                page.wait_for_timeout(100)

            with page.expect_file_chooser() as chooser:
                click("Select Kickstart")
            chooser.value.set_files(rom)
            page.wait_for_timeout(400)
            with page.expect_file_chooser() as chooser:
                click("Select disk")
            assert chooser.value.is_multiple()
            assert chooser.value.element.get_attribute("accept") == ".adf,.zip,.7z"
            chooser.value.set_files({
                "name": filename,
                "mimeType": "application/octet-stream",
                "buffer": contents,
            })
            page.wait_for_timeout(400)
            updates = page.evaluate("archiveWorker.outputs")
            click("Start")
            page.wait_for_function("updates => archiveWorker.outputs > updates + 3", arg=updates, timeout=30000)
            with page.expect_download(timeout=30000) as download:
                click("Game Save")
            path = output / (filename + ".json")
            download.value.save_as(path)
            snapshot = json.loads(path.read_text(encoding="utf-8"))
            assert snapshot["header"]["association"]["mounted"] == ["manual-0", None, None, None]
            assert snapshot["state"]["cck"] > 0
            drive = snapshot["state"]["floppy"]["drives"][0]
            assert drive["disk_inserted"]
            reference = drive["track_data"]
            assert reference["sha256"] == sha256(expected).hexdigest()
            assert reference["modified_tracks"] == {}
            results[filename] = reference["sha256"]
            assert not errors, errors
            assert not page.evaluate("archiveWorker.failures")
        except Exception:
            page.screenshot(path=str(output / (filename + "-failure.png")))
            raise
        finally:
            context.close()
    assert len(set(results.values())) == 1, results
    return results



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--browser-executable", required=True)
    parser.add_argument("--rom", type=Path)
    parser.add_argument("--disk", type=Path)
    parser.add_argument("--dist", type=Path, default=Path(".agent/tmp/web-dist"))
    parser.add_argument("--output", type=Path, default=Path(".agent/tmp/browser-player"))
    parser.add_argument("--native-state", type=Path,
                        help="Native re-export of media/browser.json and its adjacent fixtures")
    args = parser.parse_args()
    if not args.native_state and (args.rom is None or args.disk is None):
        parser.error("--rom and --disk are required unless --native-state is supplied")
    args.output.mkdir(parents=True, exist_ok=True)
    with serve_distribution(args.dist) as url, sync_playwright() as playwright:
        browser = playwright.chromium.launch(executable_path=args.browser_executable, headless=True,
            args=["--enable-webgl", "--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"])
        try:
            if args.native_state:
                result = {"native_exchange": check_native_exchange(browser, url, args.native_state, args.output)}
            else:
                result = check_workflow(browser, url, args.rom, args.disk, args.output)
                result["archive_media_digests"] = check_archive_imports(browser, url, args.rom, args.output)
                result["media_restore"] = check_media_restore(browser, url, args.output / "media")
            (args.output / "result.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
            print(json.dumps(result, indent=2))
        finally:
            browser.close()


if __name__ == "__main__":
    main()
