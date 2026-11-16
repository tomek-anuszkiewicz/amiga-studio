"""Exercise manual WASM setup and real file save/import in Chromium.

Build with tools/web.ps1, then provide local ROM/ADF files and an installed
Chromium executable. The test serves only the built distribution on loopback.
File selection exercises the ordinary HTML input fallback; physical controls
and File System Access permission dialogs remain interactive checks.
"""
import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
from threading import Thread

from playwright.sync_api import sync_playwright


class IsolatedFiles(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        super().end_headers()

    def log_message(self, *_args):
        pass


def check_workflow(browser, url, rom, disk, output):
    context = browser.new_context(viewport={"width": 1280, "height": 800}, accept_downloads=True)
    page = context.new_page()
    errors = []
    page.on("pageerror", lambda error: errors.append(str(error)))

    def observe_generation(route):
        response = route.fetch()
        source = response.text()
        marker = "export function set_host_input_enabled(enabled, interval_ms, generation) {"
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
    stage = "startup"
    try:
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
            page.mouse.click(55, 100)
        chooser.value.set_files(rom)
        page.wait_for_timeout(400)
        stage = "disk selection"
        with page.expect_file_chooser() as chooser:
            page.mouse.click(211, 121)
        chooser.value.set_files(disk)
        page.wait_for_timeout(400)
        page.screenshot(path=str(output / "ready.png"))
        stage = "manual start"
        page.mouse.click(24, 202)
        page.wait_for_function("globalThis.observedGeneration === 2", timeout=30000)
        page.wait_for_timeout(600)
        page.screenshot(path=str(output / "game.png"))
        stage = "save export"
        with page.expect_download(timeout=30000) as download:
            page.mouse.click(1084, 223)
        saved_path = output / "exported.json"
        download.value.save_as(saved_path)
        saved = json.loads(saved_path.read_text(encoding="utf-8"))
        assert saved["header"]["version"] == 10
        assert saved["header"]["timestamp"] > 0
        assert saved["header"]["screenshot_png"]
        assert saved["state"]["floppy"]["drives"][0]["disk_inserted"]
        assert saved["state"]["cck"] > 0
        stage = "state import"
        with page.expect_file_chooser() as chooser:
            page.mouse.click(1124, 223)
        chooser.value.set_files(saved_path)
        page.wait_for_function("globalThis.observedGeneration === 3", timeout=30000)
        page.wait_for_timeout(300)
        with page.expect_download(timeout=30000) as download:
            page.mouse.click(1084, 223)
        restored_path = output / "restored.json"
        download.value.save_as(restored_path)
        restored = json.loads(restored_path.read_text(encoding="utf-8"))
        assert restored["state"]["cck"] == saved["state"]["cck"], "Import booted or advanced the paused restored machine"
        assert restored["state"]["cpu"] == saved["state"]["cpu"]
        page.screenshot(path=str(output / "restored.png"))
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
                "schema": saved["header"]["version"], "errors": errors}
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
