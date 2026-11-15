"""Check real shared-memory worker startup and idle notifications in Chromium.

Serve the application with tools/web.ps1, then run:
python crates/gui/tests/browser_worker_runtime.py --browser-executable <chromium-path>
Requires the existing Playwright Python package. No browser is installed by this test.
"""

import argparse
import json
from pathlib import Path

from playwright.sync_api import sync_playwright


def check_startup(browser, url, delayed):
    context = browser.new_context(viewport={"width": 1280, "height": 800})
    if delayed:
        def delay_initialization(route):
            response = route.fetch()
            source = response.text()
            marker = "wasm = await init("
            assert source.count(marker) == 1, "Worker initialization boundary changed"
            source = source.replace(marker,
                "await new Promise(resolve => setTimeout(resolve, 80));\n        " + marker)
            route.fulfill(response=response, body=source)
        context.route("**/machine_worker.js", delay_initialization)
    page = context.new_page()
    errors, console = [], []
    page.on("pageerror", lambda error: errors.append(str(error)))
    page.on("console", lambda message: console.append({"type": message.type, "text": message.text}))
    page.add_init_script("""
        const OriginalWorker = Worker;
        window.workerReport = {created:0, outputs:0, failures:[], shared:false};
        window.Worker = class extends OriginalWorker {
            constructor(url, options) {
                super(url, options);
                workerReport.created++;
                this.addEventListener('message', event => {
                    if (event.data.type === 'output') workerReport.outputs++;
                    if (event.data.type === 'failed') workerReport.failures.push(event.data.message);
                });
            }
            postMessage(message, ...transfer) {
                if (message?.memory) workerReport.shared = message.memory.buffer instanceof SharedArrayBuffer;
                return super.postMessage(message, ...transfer);
            }
        };
    """)
    page.goto(url)
    page.wait_for_function("workerReport.outputs > 0 || workerReport.failures.length > 0", timeout=30000)
    initial = page.evaluate("workerReport")
    assert not initial["failures"], initial
    assert initial["created"] == 1 and initial["shared"], initial
    page.wait_for_timeout(300)
    before = page.evaluate("workerReport.outputs")
    page.wait_for_timeout(300)
    after = page.evaluate("workerReport.outputs")
    assert before == after, "Paused worker repeatedly notified the GUI"
    exported_probe_hooks = page.evaluate("""async () => {
        const module = await WebAssembly.compileStreaming(fetch('gui_bg.wasm'));
        return WebAssembly.Module.exports(module).filter(item => item.name.toLowerCase().includes('runtimeprobe'));
    }""")
    assert not exported_probe_hooks, "Production artifact contains diagnostic exports"
    assert not errors, errors
    assert not any(message["type"] == "error" for message in console), console
    result = {"delayed_initialization": delayed, "initial": initial, "idle_outputs": after,
              "page_errors": errors, "console": console}
    context.close()
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--browser-executable", required=True)
    parser.add_argument("--url", default="http://127.0.0.1:8080/")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(executable_path=args.browser_executable, headless=True,
            args=["--enable-webgl", "--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"])
        results = {"browser": browser.version,
                   "cases": [check_startup(browser, args.url, delayed) for delayed in (False, True)]}
        browser.close()
    encoded = json.dumps(results, indent=2)
    if args.output:
        args.output.write_text(encoded, encoding="utf-8")
    print(encoded)


if __name__ == "__main__":
    main()
