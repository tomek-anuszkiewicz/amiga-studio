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

let mouseEnabled = false;
let gamepadsEnabled = false;
let resetInput;
let releaseMouse;
let awaitingFullRelease;
let discardBufferedControls;
let captureLost = false;
let lockRequest = 0;
export function take_pointer_capture_loss() { const lost = captureLost; captureLost = false; return lost; }
export function host_release_pending() { return awaitingFullRelease?.() ?? false; }
export function discard_buffered_host_controls() { return discardBufferedControls?.() ?? '[]'; }
export function host_window_active() { return document.hasFocus() && !document.hidden; }
let updateInputTimer;
let pollMilliseconds = 0;
let inputGeneration = 0n;
export function set_host_input_enabled(mouse, gamepads, interval_ms, generation) {
    const generationChanged = inputGeneration !== generation;
    const mouseChanged = mouseEnabled !== mouse;
    const controllersChanged = gamepadsEnabled !== gamepads;
    inputGeneration = generation;
    if (generationChanged) resetInput?.();
    mouseEnabled = mouse;
    gamepadsEnabled = gamepads;
    pollMilliseconds = interval_ms;
    if (mouseChanged || generationChanged) {
        const canvas = document.getElementById('amiga_canvas');
        const attempt = ++lockRequest;
        if (mouse) {
            const failed = () => {
                if (attempt !== lockRequest || !mouseEnabled) return;
                captureLost = true;
                releaseMouse?.();
            };
            try {
                canvas.requestPointerLock()?.catch(failed);
            } catch { failed(); }
        } else {
            releaseMouse?.();
            if (document.pointerLockElement === canvas) document.exitPointerLock();
        }
    }
    if (mouseChanged || generationChanged || controllersChanged) updateInputTimer?.();
}

function startInput(collect) {
    const MAX_EVENTS = 1024;
    const controllerConnections = new Map();
    const canvas = document.getElementById('amiga_canvas');
    const listeners = [];
    let pending = [];
    awaitingFullRelease = () => pending.some(event => event[0] === 'release');
    discardBufferedControls = () => {
        pending = pending.filter(event => event[0] === 'release');
        return JSON.stringify(host_window_active() ? sampleGamepads() : []);
    };

    let retry;
    let timer;
    let schedule = 0;
    let nextPoll = 0;
    const sampling = new MessageChannel();
    const active = () => mouseEnabled && host_window_active() && document.pointerLockElement === canvas;
    const flush = () => {
        clearTimeout(retry);
        while (pending.length && collect(...pending[0])) pending.shift();
        if (pending.length) retry = setTimeout(flush, 1);
    };
    const push = event => {
        if (event[0] === 'mouse-release') {
            pending = pending.filter(queued => queued[0] !== 'motion' && queued[0] !== 'mouse');
            if (!pending.some(queued => queued[0] === 'release' || queued[0] === 'mouse-release')) {
                if (pending.length === MAX_EVENTS) pending = [['release', '', 0, 0, 0, inputGeneration]];
                else pending.push([...event, inputGeneration]);
            }
        } else if (pending.length === MAX_EVENTS || event[0] === 'release') {
            pending = [['release', '', 0, 0, 0, inputGeneration]];
        } else pending.push([...event, inputGeneration]);
        flush();
    };
    const listen = (target, type, callback, capture = false) => {
        target.addEventListener(type, callback, capture);
        listeners.push(() => target.removeEventListener(type, callback, capture));
    };
    // Rust's shared keyboard policy chooses GUI commands or guest input.
    for (const type of ['keydown', 'keyup']) listen(window, type, event => {
        const key = JSON.stringify({physical: event.code, logical: event.key || '',
            pressed: type === 'keydown', repeat: !!event.repeat, synthetic: false});
        if (collect('key', key, 0, 0, Number(host_window_active()), inputGeneration)) {
            event.preventDefault();
            // Eframe otherwise turns this DOM event into a widget key event.
            event.stopImmediatePropagation();
        }
    }, true);
    listen(canvas, 'pointerdown', event => {
        if (event.button === 0 && host_window_active()) {
            const rect = canvas.getBoundingClientRect();
            collect('primary-press', '', Math.round(event.clientX - rect.left),
                Math.round(event.clientY - rect.top), 0, inputGeneration);
        }
    }, true);
    listen(canvas, 'pointermove', event => {
        if (active()) push(['motion', '', event.movementX, event.movementY, 0]);
    });
    for (const type of ['pointerdown', 'pointerup']) listen(canvas, type, event => {
        if (!active()) return;
        if (event.button === 1) {
            if (type === 'pointerdown') {
                captureLost = true;
                releaseMouse();
                document.exitPointerLock();
            }
            event.preventDefault();
            return;
        }
        push(['mouse', '', 0, 0, event.buttons & 3]);
    });
    releaseMouse = () => push(['mouse-release', '', 0, 0, 0]);
    resetInput = () => {
        pending = [];
        captureLost = false;
        clearTimeout(retry);
    };
    const focusChanged = focused => {
        collect('focus', '', 0, 0, Number(focused), inputGeneration);
        if (!focused) {
            push(['release', '', 0, 0, 0]);
            if (document.pointerLockElement === canvas) document.exitPointerLock();
        }
        updateInputTimer();
    };
    listen(window, 'blur', () => focusChanged(false));
    listen(window, 'focus', () => focusChanged(host_window_active()));
    listen(document, 'visibilitychange', () => focusChanged(host_window_active()));
    const losePointerLock = () => {
        if (!mouseEnabled) return;
        captureLost = true;
        releaseMouse();
    };
    listen(document, 'pointerlockchange', () => {
        if (document.pointerLockElement !== canvas) losePointerLock();
        else if (!mouseEnabled) document.exitPointerLock();
    });
    listen(document, 'pointerlockerror', losePointerLock);
    listen(canvas, 'contextmenu', event => { if (active()) event.preventDefault(); });
    const sampleGamepads = () => {
        const pads = Array.from(navigator.getGamepads?.() || []).filter(pad => pad?.connected);
        return pads.map(pad => {
            let connection = controllerConnections.get(pad.index);
            if (!connection || connection.id !== pad.id) {
                connection = {id: pad.id, token: crypto.randomUUID()};
                controllerConnections.set(pad.index, connection);
            }
            return {identity: `${connection.token}:${pad.index}:${pad.id}`, name: pad.id, axes: Array.from(pad.axes), buttons: pad.buttons.map(button => button.pressed)};
        });
    };
    // Sampling runs on its own host timer, independently of requestAnimationFrame.
    const poll = () => {
        push(['pads', JSON.stringify({devices: sampleGamepads(), focused: host_window_active()}), 0, 0, 0]);
    };
    listen(window, 'gamepadconnected', event => {
        controllerConnections.set(event.gamepad.index, {id: event.gamepad.id, token: crypto.randomUUID()});
        poll();
    });
    listen(window, 'gamepaddisconnected', event => {
        controllerConnections.delete(event.gamepad.index);
        push(['release', '', 0, 0, 0]);
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
        nextPoll = Math.max(nextPoll + ((active() || gamepadsEnabled) ? pollMilliseconds : 250), performance.now());
        const token = schedule;
        // Keep timer nesting independent of the sampling task.
        timer = setTimeout(() => sampling.port2.postMessage(token), Math.max(0, Math.ceil(nextPoll - performance.now())));
    };
    updateInputTimer();
    return () => { clearTimeout(timer); clearTimeout(retry); sampling.port1.close(); sampling.port2.close(); listeners.forEach(remove => remove()); resetInput = undefined; releaseMouse = undefined; awaitingFullRelease = undefined; discardBufferedControls = undefined; updateInputTimer = undefined; };
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
