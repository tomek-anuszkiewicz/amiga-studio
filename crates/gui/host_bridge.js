// Host notifications carry no machine state or image pixels.
export function start_machine_worker(module, memory, notify, finished, input) {
    if (!crossOriginIsolated || !(memory.buffer instanceof SharedArrayBuffer)) {
        throw new Error('The machine worker requires cross-origin isolation and shared Wasm memory');
    }
    const worker = new Worker(new URL('machine_worker.js', document.baseURI), {type: 'module'});
    const stopInput = startInput(input);
    const complete = success => {
        worker.onmessage = null;
        worker.onerror = null;
        worker.terminate();
        stopInput();
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

let inputEnabled = false;
let resetInput;
let updateInputTimer;
let pollMilliseconds = 0;
let inputGeneration = 0n;
export function set_host_input_enabled(enabled, interval_ms, generation) {
    const changed = inputEnabled !== enabled || inputGeneration !== generation;
    if (changed) resetInput?.();
    inputEnabled = enabled;
    pollMilliseconds = interval_ms;
    inputGeneration = generation;
    if (changed) updateInputTimer?.();
}

function startInput(collect) {
    const MAX_EVENTS = 1024;
    const AXIS_THRESHOLD = 0.5;
    const canvas = document.getElementById('amiga_canvas');
    const listeners = [];
    let pending = [];
    let lastPad = 0;
    let buttons = 0;
    const keys = new Set();
    let retry;
    let timer;
    let schedule = 0;
    let nextPoll = 0;
    const sampling = new MessageChannel();
    const active = () => inputEnabled && document.hasFocus() && !document.hidden;
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
    const reserved = new Set(['F2', 'F5', 'F6', 'F9', 'F10', 'F11', 'F12', 'Escape']);
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
        lastPad = 0;
        push(['release', '', 0, 0, 0]);
    };
    resetInput = release;
    listen(window, 'blur', () => { release(); updateInputTimer(); });
    listen(window, 'focus', () => updateInputTimer());
    listen(document, 'visibilitychange', () => { if (document.hidden) release(); updateInputTimer(); });
    listen(canvas, 'pointerleave', release);
    listen(canvas, 'contextmenu', event => { if (active()) event.preventDefault(); });
    // Sampling runs on its own host timer, independently of requestAnimationFrame.
    const poll = () => {
        if (!active()) return;
        const pads = navigator.getGamepads?.() || [];
        const pad = Array.from(pads).find(pad => pad?.connected && pad.mapping === 'standard');
        let state = 0;
        if (pad) {
            const pressed = index => Boolean(pad.buttons[index]?.pressed);
            const x = pad.axes[0] || 0, y = pad.axes[1] || 0;
            state = Number(y < -AXIS_THRESHOLD || pressed(12))
                | Number(y > AXIS_THRESHOLD || pressed(13)) << 1
                | Number(x < -AXIS_THRESHOLD || pressed(14)) << 2
                | Number(x > AXIS_THRESHOLD || pressed(15)) << 3
                | Number(pressed(0)) << 4 | Number(pressed(1)) << 5;
        }
        if (state !== lastPad) {
            push(['joystick', '', 0, 0, state]);
            lastPad = state;
        }
    };
    updateInputTimer = () => {
        clearTimeout(timer);
        schedule++;
        if (active()) {
            nextPoll = performance.now();
            sampling.port2.postMessage(schedule);
        }
    };
    sampling.port1.onmessage = event => {
        if (event.data !== schedule || !active()) return;
        poll();
        nextPoll = Math.max(nextPoll + pollMilliseconds, performance.now());
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
