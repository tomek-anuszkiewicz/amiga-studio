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


def check_failure_delivery(browser, url, screenshot):
    """Inject at the JS boundary; real Rust invariant trap coverage is separate."""
    diagnostic = "Injected worker invariant: original diagnostic"
    context = browser.new_context(viewport={"width": 1280, "height": 800})

    def fail_first_tick(route):
        response = route.fetch()
        source = response.text()
        marker = "const delay = host_worker_tick();"
        assert source.count(marker) == 1, "Worker execution boundary changed"
        # Model the hook's retained pre-trap message and a subsequent generic
        # Wasm trap. The original diagnostic must win at the existing catch.
        source = source.replace(marker,
            f"globalThis.machineFailureMessage = {json.dumps(diagnostic)};\n"
            "        throw new Error('Generic Wasm trap must not replace the diagnostic');")
        source = source.replace("wasm.__wbindgen_thread_destroy();",
            "postMessage({type: 'unexpected_cleanup'});\n            wasm.__wbindgen_thread_destroy();")
        route.fulfill(response=response, body=source)

    context.route("**/machine_worker.js", fail_first_tick)
    page = context.new_page()
    errors = []
    page.on("pageerror", lambda error: errors.append(str(error)))
    page.add_init_script("""
        const OriginalWorker = Worker;
        window.failureReport = {messages:[], terminations:0};
        window.Worker = class extends OriginalWorker {
            constructor(url, options) {
                super(url, options);
                this.addEventListener('message', event => failureReport.messages.push(event.data));
            }
            terminate() {
                failureReport.terminations++;
                return super.terminate();
            }
        };
    """)
    page.goto(url)
    page.wait_for_function("failureReport.terminations === 1", timeout=30000)
    page.wait_for_timeout(300)
    report = page.evaluate("failureReport")
    assert report["messages"] == [{"type": "failed", "message": diagnostic}], report
    assert not errors, errors
    if screenshot:
        page.screenshot(path=str(screenshot))
    context.close()
    return {"injection_boundary": "JavaScript pre-trap diagnostic storage", "report": report,
            "page_errors": errors}



def check_rust_failure(browser, url, snapshot, screenshot):
    """Load an ordinary state file and execute its forced DMA queue overflow."""
    context = browser.new_context(viewport={"width": 1280, "height": 800})
    def observe_generation(route):
        response = route.fetch()
        source = response.text()
        marker = "export function set_host_input_enabled(mouse, gamepads, interval_ms, generation) {"
        assert source.count(marker) == 1
        source = source.replace(marker, marker + "\n    globalThis.observedGeneration = Number(generation);")
        route.fulfill(response=response, body=source)
    def observe_cleanup(route):
        response = route.fetch()
        source = response.text().replace("wasm.__wbindgen_thread_destroy();",
            "postMessage({type:'unexpected_cleanup'}); wasm.__wbindgen_thread_destroy();")
        route.fulfill(response=response, body=source)
    context.route("**/host_bridge.js", observe_generation)
    context.route("**/machine_worker.js", observe_cleanup)
    page = context.new_page()
    errors = []
    page.on("pageerror", lambda error: errors.append(str(error)))
    state = snapshot.read_bytes()
    context.route("**/forced-invariant.json", lambda route: route.fulfill(body=state, content_type="application/json"))
    page.add_init_script("""
        const OriginalWorker = Worker;
        window.rustFailure = {messages:[], terminations:0};
        window.Worker = class extends OriginalWorker {
            constructor(url, options) {
                super(url, options);
                this.addEventListener('message', event => rustFailure.messages.push(event.data));
            }
            terminate() { rustFailure.terminations++; return super.terminate(); }
        };
    """)
    page.add_init_script("window.showOpenFilePicker = async () => [{getFile: async () => new File([await (await fetch('/forced-invariant.json')).blob()], 'forced-invariant.json', {type:'application/json'})}];")
    page.goto(url)
    page.wait_for_function("globalThis.observedGeneration === 1", timeout=30000)
    page.wait_for_timeout(400)
    # Canvas UI: the ordinary State menu and its Load State action.
    page.mouse.click(60, 15)
    page.wait_for_timeout(150)
    page.mouse.click(125, 64)
    page.wait_for_timeout(1500)
    if screenshot:
        page.screenshot(path=str(screenshot))
    page.wait_for_function("globalThis.observedGeneration === 2", timeout=30000)
    page.mouse.click(130, 15)
    page.wait_for_function("rustFailure.terminations === 1", timeout=30000)
    page.wait_for_timeout(300)
    report = page.evaluate("rustFailure")
    failures = [item for item in report["messages"] if item["type"] == "failed"]
    assert len(failures) == 1, report
    message = failures[0]["message"]
    for expected in ("Mutation queue overflow", "queue=Paula", "CCK=Some(1)",
                     "producer=DMA", "capacity=32", "occupancy=32"):
        assert expected in message, message
    assert not any(item["type"] in ("stopped", "unexpected_cleanup") for item in report["messages"]), report
    count = len(report["messages"])
    # A failed owner must not restart or publish more work after another Run.
    page.mouse.click(130, 15)
    page.wait_for_timeout(300)
    assert page.evaluate("rustFailure.messages.length") == count
    assert page.evaluate("rustFailure.terminations") == 1
    assert not errors, errors
    if screenshot:
        page.screenshot(path=str(screenshot))
    context.close()
    return {"boundary": "real Rust panic hook and Wasm trap after loading a forced DMA state",
            "report": report, "page_errors": errors}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--browser-executable", required=True)
    parser.add_argument("--url", default="http://127.0.0.1:8080/")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--fatal-snapshot", type=Path)
    args = parser.parse_args()
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(executable_path=args.browser_executable, headless=True,
            args=["--enable-webgl", "--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"])
        results = {"browser": browser.version,
                   "cases": [check_startup(browser, args.url, delayed) for delayed in (False, True)],
                   "failure": check_failure_delivery(browser, args.url,
                       args.output.with_suffix(".png") if args.output else None)}
        if args.fatal_snapshot:
            results["rust_failure"] = check_rust_failure(browser, args.url, args.fatal_snapshot,
                args.output.with_name(args.output.stem + "-rust.png") if args.output else None)
        browser.close()
    encoded = json.dumps(results, indent=2)
    if args.output:
        args.output.write_text(encoded, encoding="utf-8")
    print(encoded)


if __name__ == "__main__":
    main()
