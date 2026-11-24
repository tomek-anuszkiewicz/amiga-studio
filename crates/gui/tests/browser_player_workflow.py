"""Exercise manual WASM setup, file save/import and disk swaps in Chromium.

Build with tools/web.ps1, then provide local ROM/ADF files and an installed
Chromium executable. The test serves only the built distribution on loopback.
File selection exercises the ordinary HTML input fallback; physical controls
and File System Access permission dialogs remain interactive checks.
"""
import argparse
from functools import partial
from hashlib import sha256
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
from threading import Thread
from urllib.parse import urlparse

from playwright.sync_api import sync_playwright


class IsolatedFiles(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        super().end_headers()

    def log_message(self, *_args):
        pass


def first_raw_block(snapshot):
    drive = snapshot["state"]["floppy"]["drives"][0]
    return drive["track_data"][0]["Encoded"][0]


def check_workflow(browser, url, rom, disk, output):
    context = browser.new_context(viewport={"width": 1280, "height": 800}, accept_downloads=True)
    page = context.new_page()
    errors = []
    page.on("pageerror", lambda error: errors.append(str(error)))

    def observe_generation(route):
        response = route.fetch()
        source = response.text()
        marker = "export function set_host_input_enabled(mouse, gamepads, interval_ms, generation) {"
        assert source.count(marker) == 1
        route.fulfill(response=response, body=source.replace(marker, marker + "\n    globalThis.observedGeneration = Number(generation);"))

    context.route("**/host_bridge.js", observe_generation)
    page.add_init_script("""
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
    """)
    def click(x, y):
        # Let the canvas consume hover and the pressed state on separate frames.
        page.mouse.move(x, y)
        page.wait_for_timeout(120)
        page.mouse.click(x, y, delay=80)
        page.wait_for_timeout(100)

    def export_state(path):
        with page.expect_download(timeout=30000) as download:
            click(1084, 244)
        download.value.save_as(path)
        return json.loads(path.read_text(encoding="utf-8"))

    def import_state(path, generation):
        with page.expect_file_chooser() as chooser:
            click(1124, 244)
        chooser.value.set_files(path)
        page.wait_for_function(
            "generation => globalThis.observedGeneration === generation",
            arg=generation, timeout=30000,
        )
        page.wait_for_timeout(300)

    def drive_label():
        # Sample the DF0 text, leaving out the activity indicator and hover.
        page.mouse.move(1255, 108)
        return page.screenshot(clip={"x": 1126, "y": 101, "width": 76, "height": 14})

    def select_disk(row, description):
        before = drive_label()
        click(1222, 108)
        click(1150, row)
        for _ in range(20):
            after = drive_label()
            if after != before:
                return after
            page.wait_for_timeout(100)
        raise AssertionError(f"DF0 did not confirm {description}")

    def check_remount(snapshot, disk_id, marker, generation, row):
        # Seed changed raw tracks in a disposable snapshot before making any swap.
        assert first_raw_block(snapshot)["Encoded"] != marker
        first_raw_block(snapshot)["Encoded"] = marker
        snapshot["header"]["association"]["mounted"][0] = disk_id
        fixture = output / f"{disk_id}-fixture.json"
        fixture.write_text(json.dumps(snapshot, separators=(",", ":")), encoding="utf-8")
        import_state(fixture, generation)
        mounted_label = drive_label()
        replacement_label = select_disk(150, "the replacement disk")
        empty_label = select_disk(130, "ejection")
        assert len({mounted_label, replacement_label, empty_label}) == 3
        assert select_disk(row, "reinsertion") == mounted_label
        assert page.evaluate("observedGeneration") == generation, "Disk swap replaced the machine"

        # Observe the tracks only after all disk commands have completed.
        remounted = export_state(output / f"{disk_id}-remounted.json")
        assert remounted["header"]["association"]["mounted"][0] == disk_id
        assert remounted["state"]["floppy"]["drives"][0]["disk_inserted"]
        assert first_raw_block(remounted)["Encoded"] == marker, (
            f"DF0 lost the raw track marker on {disk_id}"
        )

    stage = "startup"
    try:
        with page.expect_response(
            lambda response: urlparse(response.url).path.endswith("/gui_bg.wasm")
        ) as wasm:
            page.goto(url)
        page.wait_for_function("globalThis.observedGeneration === 1", timeout=30000)
        page.wait_for_timeout(800)
        counts = []
        for _ in range(3):
            page.wait_for_timeout(1100)
            counts.append(page.evaluate("workerReport.outputs"))
        assert len(set(counts)) == 1, f"Paused notifications continued without input: {counts}"
        page.screenshot(path=str(output / "setup.png"))
        stage = "ROM selection"
        with page.expect_file_chooser() as chooser:
            click(55, 100)
        chooser.value.set_files(rom)
        page.wait_for_timeout(400)
        stage = "disk selection"
        disk_digest = sha256(disk.read_bytes()).digest()
        with page.expect_file_chooser() as chooser:
            click(211, 121)
        chooser.value.set_files({
            "name": "replacement.adf" if disk.name != "replacement.adf" else "other.adf",
            "mimeType": "application/octet-stream",
            "buffer": bytes([255]) * (880 * 1024),
        })
        page.wait_for_timeout(400)
        with page.expect_file_chooser() as chooser:
            click(211, 121)
        chooser.value.set_files(disk)
        page.wait_for_timeout(400)
        page.screenshot(path=str(output / "ready.png"))
        stage = "manual start"
        click(24, 202)
        page.wait_for_function("globalThis.observedGeneration === 2", timeout=30000)
        page.wait_for_timeout(600)
        page.screenshot(path=str(output / "game.png"))
        stage = "browser toolbar eligibility"
        # The canvas toolbar exposes no DOM or AccessKit nodes. Check the loaded
        # Wasm for a shared control and the native button's application-specific ID.
        application = wasm.value.body()
        assert b"Debug View (F12)" in application
        assert b"amiga-game-screenshot-button" not in application, (
            "Native screenshot button ID found in browser Wasm"
        )
        assert b"amiga-game-cover-button" not in application, (
            "Native cover button ID found in browser Wasm"
        )
        stage = "save export"
        saved_path = output / "exported.json"
        saved = export_state(saved_path)
        assert saved["header"]["version"] == 15
        assert saved["header"]["timestamp"] > 0
        assert saved["header"]["screenshot_png"]
        assert saved["header"]["association"]["mounted"][0] == "manual-1"
        assert saved["state"]["floppy"]["drives"][0]["disk_inserted"]
        assert saved["state"]["cck"] > 0
        stage = "state import"
        import_state(saved_path, 3)
        restored = export_state(output / "restored.json")
        assert restored["state"]["cck"] == saved["state"]["cck"], "Import booted or advanced the paused restored machine"
        assert restored["state"]["cpu"] == saved["state"]["cpu"]
        page.screenshot(path=str(output / "restored.png"))
        stage = "configuration Cancel"
        stored_before_edit = page.evaluate("localStorage.getItem('player_configurations')")
        click(1088, 274)  # Setup; the existing machine remains paused.
        page.wait_for_timeout(300)
        click(145, 10)  # Configurations in the browser navigation bar.
        page.wait_for_timeout(300)
        page.screenshot(path=str(output / "configuration.png"))
        click(140, 138)  # Draft name.
        page.keyboard.press("Control+a")
        page.keyboard.type("Cancelled edit")
        click(70, 789)  # Cancel.
        page.wait_for_timeout(400)
        assert page.evaluate("localStorage.getItem('player_configurations')") == stored_before_edit
        assert page.evaluate("observedGeneration") == 3, "Cancel changed the machine generation"
        stage = "configuration Save and changed-port return"
        click(145, 10)
        page.wait_for_timeout(300)
        click(55, 432)  # Port 2 device dropdown.
        page.wait_for_timeout(200)
        page.screenshot(path=str(output / "port-options.png"))
        click(55, 456)  # Disconnected, the first row in the popup.
        click(23, 789)  # Save, including stopped owner application.
        page.wait_for_function("globalThis.observedGeneration === 4", timeout=30000)
        page.wait_for_timeout(300)
        click(250, 10)  # Return to session.
        page.wait_for_timeout(300)
        changed = export_state(output / "changed-ports.json")
        expected = json.loads(json.dumps(saved["state"]))
        expected["game_ports"]["state"]["port2"] = "None"
        assert changed["state"] == expected, "Stopped controls save changed machine progress or receivers"
        stage = "load older endpoint types with current controls"
        import_state(saved_path, 5)
        current = export_state(output / "current-controls-load.json")
        assert current["state"] == expected, "Import restored historical attachments or changed progress"
        page.screenshot(path=str(output / "current-controls-load.png"))
        stage = "swap and remount changed disk with a source ADF"
        check_remount(current, "manual-1", [0xDE, 0xAD, 0xBE, 0xEF] * 4, 6, 170)
        stage = "swap and remount changed disk with only embedded tracks"
        check_remount(current, "embedded-disk", [0xCA, 0xFE, 0xBA, 0xBE] * 4, 7, 190)
        assert sha256(disk.read_bytes()).digest() == disk_digest, "The original ADF changed"
        stage = "configuration persistence"
        stored = page.evaluate("localStorage.getItem('player_configurations')")
        assert stored and rom.name in stored, "Selected configuration was not persisted before reopening"
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
                "disk_swap_retained_raw_tracks": True, "embedded_disk_reinserted": True,
                "native_screenshot_action_excluded": True,
                "native_cover_action_excluded": True, "errors": errors}
    except Exception:
        page.screenshot(path=str(output / "failure.png"))
        print(f"Failed browser workflow stage: {stage}")
        print(json.dumps({"errors": errors, "worker": page.evaluate("workerReport")}, indent=2))
        raise
    finally:
        context.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--browser-executable", required=True)
    parser.add_argument("--rom", type=Path, required=True)
    parser.add_argument("--disk", type=Path, required=True)
    parser.add_argument("--dist", type=Path, default=Path(".agent/tmp/web-dist"))
    parser.add_argument("--output", type=Path, default=Path(".agent/tmp/browser-player"))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    server = ThreadingHTTPServer(("127.0.0.1", 0), partial(IsolatedFiles, directory=str(args.dist.resolve())))
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(executable_path=args.browser_executable, headless=True,
                args=["--enable-webgl", "--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"])
            try:
                result = check_workflow(browser, f"http://127.0.0.1:{server.server_port}/", args.rom, args.disk, args.output)
                (args.output / "result.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
                print(json.dumps(result, indent=2))
            finally:
                browser.close()
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    main()
