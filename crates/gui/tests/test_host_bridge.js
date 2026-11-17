// Deterministic DOM-adapter regressions; run with node --test.
const assert = require('node:assert/strict');
const {readFile} = require('node:fs/promises');
const {test} = require('node:test');
const {join} = require('node:path');

test('browser capture release and independent gamepad polling', async () => {
    class Surface {
        listeners = new Map();
        addEventListener(type, callback) { this.listeners.set(type, [...(this.listeners.get(type) || []), callback]); }
        removeEventListener(type, callback) { this.listeners.set(type, this.listeners.get(type).filter(item => item !== callback)); }
        fire(type, event = {}) { for (const callback of this.listeners.get(type) || []) callback(event); }
    }
    const canvas = new Surface();
    const document = new Surface();
    const window = new Surface();
    let focused = true;
    let token;
    let channel;
    let timerDelay;
    document.baseURI = 'https://example.invalid/';
    document.hidden = false;
    document.hasFocus = () => focused;
    document.getElementById = () => canvas;
    document.exitPointerLock = () => { document.pointerLockElement = null; document.fire('pointerlockchange'); };
    canvas.requestPointerLock = () => { document.pointerLockElement = canvas; };
    const controller = {index: 0, id: 'pad', connected: true, axes: [0, 0], buttons: [{pressed: false}]};
    const saved = new Map();
    const globals = {
        document, window, crossOriginIsolated: true,
        navigator: {getGamepads: () => [controller]},
        Worker: class {postMessage() {} terminate() {}},
        MessageChannel: class {
            constructor() {
                this.port1 = {close() {}};
                this.port2 = {postMessage(value) {token = value;}, close() {}};
                channel = this;
            }
        },
        setTimeout: (_callback, delay) => {timerDelay = delay; return 1;},
        clearTimeout() {},
    };
    for (const [name, value] of Object.entries(globals)) {
        saved.set(name, Object.getOwnPropertyDescriptor(globalThis, name));
        Object.defineProperty(globalThis, name, {value, configurable: true, writable: true});
    }
    try {
        const source = await readFile(join(__dirname, '../host_bridge.js'), 'utf8');
        const bridge = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);
        const events = [];
        let accepting = true;
        const worker = bridge.start_machine_worker({}, {buffer: new SharedArrayBuffer(16)}, () => {}, () => {},
            (...event) => {if (!accepting) return false; events.push(event); return true;});
        const poll = () => channel.port1.onmessage({data: token});
        bridge.set_host_input_enabled(false, true, 6, 1n);
        poll();
        assert.equal(JSON.parse(events.at(-1)[1]).focused, true);
        assert.equal(document.pointerLockElement, undefined, 'Gamepads must not lock the pointer');
        assert.ok(timerDelay <= 6, 'Gamepads without capture retain active polling cadence');
        bridge.set_host_input_enabled(true, true, 6, 1n);
        assert.equal(document.pointerLockElement, canvas);
        events.length = 0;
        let prevented = false;
        window.fire('keydown', {code: 'KeyR', ctrlKey: true, preventDefault() {prevented = true;}});
        assert.equal(events.at(-1)[0], 'keydown');
        assert.equal(events.at(-1)[1], 'KeyR');
        assert.ok(prevented, 'Ctrl+R reaches the guest without browser reload');
        for (const code of ['Escape', 'F2']) {
            window.fire('keydown', {code, preventDefault() {}});
            assert.equal(events.at(-1)[1], code);
        }
        const before = events.length;
        window.fire('keydown', {code: 'F12', preventDefault() {}});
        assert.equal(events.length, before, 'F12 remains a frontend key');
        accepting = false;
        window.fire('keydown', {code: 'KeyA', preventDefault() {}});
        canvas.fire('pointerdown', {button: 1, buttons: 4, preventDefault() {}});
        assert.equal(document.pointerLockElement, null);
        assert.equal(bridge.take_pointer_capture_loss(), true);
        assert.equal(bridge.take_pointer_capture_loss(), false);
        accepting = true;
        bridge.set_host_input_enabled(false, true, 6, 1n);
        assert.ok(events.some(event => event[0] === 'capture-release'));
        assert.ok(!events.some(event => event[1] === 'KeyA'), 'Capture release discards lock-contended keyboard backlog');
        assert.ok(!events.some(event => event[0] === 'mouse'), 'Middle mouse never reaches the guest');
        bridge.set_host_input_enabled(false, true, 6, 1n);
        poll();
        assert.equal(events.at(-1)[0], 'pads');
        focused = false;
        window.fire('blur');
        assert.equal(events.at(-1)[0], 'release');
        poll();
        assert.equal(JSON.parse(events.at(-1)[1]).focused, false);
        focused = true;
        bridge.set_host_input_enabled(true, true, 6, 1n);
        events.length = 0;
        accepting = false;
        focused = false;
        window.fire('blur');
        document.exitPointerLock();
        accepting = true;
        poll();
        assert.ok(events.some(event => event[0] === 'release'), 'Pointer-lock loss must not downgrade a pending full focus release');
        assert.ok(!events.some(event => event[0] === 'capture-release'));
        bridge.set_host_input_enabled(false, false, 6, 1n);
        poll();
        assert.ok(timerDelay > 6, 'Inactive gamepads use discovery cadence');
        worker.onmessage({data: {type: 'stopped'}});
    } finally {
        for (const [name, descriptor] of saved) {
            if (descriptor) Object.defineProperty(globalThis, name, descriptor);
            else delete globalThis[name];
        }
    }
});
