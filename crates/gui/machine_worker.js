import init, {host_worker_tick} from './gui.js';

let wasm;
let timer;
let schedule = 0;
const immediate = new MessageChannel();
immediate.port1.onmessage = event => {
    if (event.data === schedule) tick();
};
const WAIT_FOR_EVENT = -1;
const STOPPED = -2;

function tick() {
    clearTimeout(timer);
    schedule++;
    try {
        const delay = host_worker_tick();
        if (delay === STOPPED) {
            // Release this instance's stack and TLS after the last Rust call.
            wasm.__wbindgen_thread_destroy();
            postMessage({type: 'stopped'});
            close();
        } else if (delay === 0) {
            // Message tasks avoid the nested-timer minimum delay between slices.
            immediate.port2.postMessage(schedule);
        } else if (delay !== WAIT_FOR_EVENT) {
            timer = setTimeout(tick, delay);
        }
    } catch (error) {
        postMessage({type: 'failed', message: String(error.stack || error)});
        close();
    }
}

onmessage = async event => {
    // Early wakes remain represented in the shared queues and stop flag.
    // The first tick reads them after this one initialization has completed.
    onmessage = () => {};
    try {
        wasm = await init({module_or_path: event.data.module, memory: event.data.memory});
        onmessage = tick;
        tick();
    } catch (error) {
        postMessage({type: 'failed', message: String(error.stack || error)});
        close();
    }
};
