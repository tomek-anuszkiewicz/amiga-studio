// Host notifications carry no machine state or image pixels.
export function start_machine_worker(module, memory, notify, finished) {
    if (!crossOriginIsolated || !(memory.buffer instanceof SharedArrayBuffer)) {
        throw new Error('The machine worker requires cross-origin isolation and shared Wasm memory');
    }
    const worker = new Worker(new URL('machine_worker.js', document.baseURI), {type: 'module'});
    const complete = success => {
        worker.onmessage = null;
        worker.onerror = null;
        worker.terminate();
        finished(success);
    };
    worker.onmessage = event => {
        if (event.data.type === 'output') notify();
        else if (event.data.type === 'stopped') complete(true);
        else if (event.data.type === 'failed') {
            console.error(event.data.message);
            complete(false);
        }
    };
    worker.onerror = event => {
        console.error(event.message);
        complete(false);
    };
    try {
        worker.postMessage({module, memory});
    } catch (error) {
        worker.terminate();
        throw error;
    }
    return worker;
}

export function host_now() {
    return performance.now() / 1000;
}

export function notify_gui() {
    postMessage({type: 'output'});
}
