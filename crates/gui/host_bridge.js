// Host notifications carry no machine state or image pixels.
export function start_machine_worker(module, memory, notify, finished, input) {
    if (!crossOriginIsolated || !(memory.buffer instanceof SharedArrayBuffer)) {
        throw new Error('The machine worker requires cross-origin isolation and shared Wasm memory');
    }
    const worker = new Worker(new URL('machine_worker.js', document.baseURI), {type: 'module'});
    const stopInput = startInput(input);
    const complete = (success, message = '') => {
        worker.onmessage = null;
        worker.onerror = null;
        worker.terminate();
        stopInput();
        finished(success, message);
    };
    worker.onmessage = event => {
        if (event.data.type === 'output') notify();
        else if (event.data.type === 'stopped') complete(true);
        else if (event.data.type === 'failed') {
            console.error(event.data.message);
            complete(false, event.data.message);
        }
    };
    worker.onerror = event => {
        console.error(event.message);
        complete(false, event.message);
    };
    try {
        worker.postMessage({module, memory});
    } catch (error) {
        worker.terminate();
        throw error;
    }
    return worker;
}

let inputEnabled = false;
let resetInput;
let captureLost = false;
export function take_pointer_capture_loss() { const lost = captureLost; captureLost = false; return lost; }
let updateInputTimer;
let pollMilliseconds = 0;
let inputGeneration = 0n;
export function set_host_input_enabled(enabled, interval_ms, generation) {
    const changed = inputEnabled !== enabled || inputGeneration !== generation;
    if (changed) resetInput?.();
    if (changed && enabled) {
        const canvas = document.getElementById('amiga_canvas');
        try {
            const request = canvas.requestPointerLock();
            request?.catch(() => { captureLost = true; resetInput?.(); });
        } catch { captureLost = true; }
    } else if (!enabled && document.pointerLockElement) document.exitPointerLock();
    inputEnabled = enabled;
    pollMilliseconds = interval_ms;
    inputGeneration = generation;
    if (changed) updateInputTimer?.();
}

function startInput(collect) {
    const MAX_EVENTS = 1024;
    const controllerConnections = new Map();
    const canvas = document.getElementById('amiga_canvas');
    const listeners = [];
    let pending = [];

    let buttons = 0;
    const keys = new Set();
    let retry;
    let timer;
    let schedule = 0;
    let nextPoll = 0;
    const sampling = new MessageChannel();
    const active = () => inputEnabled && document.hasFocus() && !document.hidden && document.pointerLockElement === canvas;
    const flush = () => {
        clearTimeout(retry);
        while (pending.length && collect(...pending[0])) pending.shift();
        if (pending.length) retry = setTimeout(flush, 1);
    };
    const push = event => {
        if (pending.length === MAX_EVENTS || event[0] === 'release') pending = [['release', '', 0, 0, 0, inputGeneration]];
        else pending.push([...event, inputGeneration]);
        flush();
    };
    const listen = (target, type, callback, capture = false) => {
        target.addEventListener(type, callback, capture);
        listeners.push(() => target.removeEventListener(type, callback, capture));
    };
    const reserved = new Set(['F11', 'F12']);
    for (const type of ['keydown', 'keyup']) listen(window, type, event => {
        if (!active() || reserved.has(event.code) || event.repeat) return;
        if (type === 'keydown') keys.add(event.code);
        else keys.delete(event.code);
        // Application command shortcuts retain their existing meaning.
        if (type === 'keydown' && (event.ctrlKey || event.metaKey) && ['KeyR', 'KeyO', 'KeyS', 'KeyL', 'KeyT', 'Equal', 'Minus', 'Digit0'].includes(event.code)) return;
        if (type === 'keyup' && (event.code === 'ControlLeft' || event.code === 'ControlRight')
            && (keys.has('ControlLeft') || keys.has('ControlRight'))) return;
        push([type, event.code, 0, 0, 0]);
        event.preventDefault();
    }, true);
    listen(canvas, 'pointermove', event => {
        if (active()) push(['motion', '', event.movementX, event.movementY, 0]);
    });
    for (const type of ['pointerdown', 'pointerup']) listen(canvas, type, event => {
        if (!active()) return;
        buttons = event.buttons;
        push(['mouse', '', 0, 0, buttons]);
    });
    const release = () => {
        buttons = 0;
        keys.clear();

        push(['release', '', 0, 0, 0]);
    };
    resetInput = release;
    listen(window, 'blur', () => { release(); updateInputTimer(); });
    listen(window, 'focus', () => updateInputTimer());
    listen(document, 'visibilitychange', () => { if (document.hidden) release(); updateInputTimer(); });
    listen(document, 'pointerlockchange', () => { if (document.pointerLockElement !== canvas) { captureLost = true; release(); } });
    listen(document, 'pointerlockerror', () => { captureLost = true; release(); });
    listen(canvas, 'contextmenu', event => { if (active()) event.preventDefault(); });
    // Sampling runs on its own host timer, independently of requestAnimationFrame.
    const poll = () => {
        const pads = Array.from(navigator.getGamepads?.() || []).filter(pad => pad?.connected);
        const samples = pads.map(pad => {
            let connection = controllerConnections.get(pad.index);
            if (!connection || connection.id !== pad.id) {
                connection = {id: pad.id, token: crypto.randomUUID()};
                controllerConnections.set(pad.index, connection);
            }
            return {identity: `${connection.token}:${pad.index}:${pad.id}`, name: pad.id, axes: Array.from(pad.axes), buttons: pad.buttons.map(button => button.pressed)};
        });
        push(['pads', JSON.stringify({devices: samples, focused: document.hasFocus() && !document.hidden}), 0, 0, Number(active())]);
    };
    listen(window, 'gamepadconnected', event => {
        controllerConnections.set(event.gamepad.index, {id: event.gamepad.id, token: crypto.randomUUID()});
        poll();
    });
    listen(window, 'gamepaddisconnected', event => {
        controllerConnections.delete(event.gamepad.index);
        release();
        poll();
    });
    updateInputTimer = () => {
        clearTimeout(timer);
        schedule++;
        if (!document.hidden) {
            nextPoll = performance.now();
            sampling.port2.postMessage(schedule);
        }
    };
    sampling.port1.onmessage = event => {
        if (event.data !== schedule || document.hidden) return;
        poll();
        nextPoll = Math.max(nextPoll + (active() ? pollMilliseconds : 250), performance.now());
        const token = schedule;
        // Keep timer nesting independent of the sampling task.
        timer = setTimeout(() => sampling.port2.postMessage(token), Math.max(0, Math.ceil(nextPoll - performance.now())));
    };
    updateInputTimer();
    return () => { clearTimeout(timer); clearTimeout(retry); sampling.port1.close(); sampling.port2.close(); listeners.forEach(remove => remove()); resetInput = undefined; updateInputTimer = undefined; };
}

export function host_now() {
    return performance.now() / 1000;
}

export function notify_gui() {
    postMessage({type: 'output'});
}

// Called by the panic hook before panic_abort traps. Worker-local JS storage
// survives the trap and needs no Rust owner, mailbox, stack or TLS cleanup.
export function report_machine_failure(message) {
    if (typeof document === 'undefined') {
        globalThis.machineFailureMessage ??= message;
    } else {
        console.error(message);
    }
}
