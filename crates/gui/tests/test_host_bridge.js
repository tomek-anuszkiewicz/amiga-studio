// Deterministic DOM-adapter regressions; run with node --test.
const assert = require('node:assert/strict');
const {readFile} = require('node:fs/promises');
const {test} = require('node:test');
const {join} = require('node:path');

test('document keyboard focus and mouse release leave gamepad polling independent', async () => {
    class Surface {
        listeners = new Map();
        addEventListener(type, callback) { this.listeners.set(type, [...(this.listeners.get(type) || []), callback]); }
        removeEventListener(type, callback) { this.listeners.set(type, this.listeners.get(type).filter(item => item !== callback)); }
        fire(type, event = {}) {
            for (const callback of this.listeners.get(type) || []) {
                callback(event);
                if (event.stopped) break;
            }
        }
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
        let backendReady = true;
        let keyOwner = true;
        const worker = bridge.start_machine_worker({}, {buffer: new SharedArrayBuffer(16)}, () => {}, () => {},
            (...event) => {
                if (event[0] === 'focus') {
                    events.push(event);
                    return true;
                }
                if (event[0] === 'key') {
                    events.push(event);
                    return keyOwner && event[4] !== 0;
                }
                if (!backendReady) return false;
                events.push(event);
                return true;
            });
        const poll = () => channel.port1.onmessage({data: token});
        let widgetKeys = 0;
        window.addEventListener('keydown', () => widgetKeys++);
        window.addEventListener('keyup', () => widgetKeys++);
        const key = (type, code, owned) => {
            const previousWidgets = widgetKeys;
            const event = {
                code, key: code, prevented: false, stopped: false,
                preventDefault() { this.prevented = true; },
                stopImmediatePropagation() { this.stopped = true; },
            };
            window.fire(type, event);
            assert.equal(events.at(-1)[0], 'key');
            const edge = JSON.parse(events.at(-1)[1]);
            assert.equal(edge.physical, code);
            assert.equal(edge.pressed, type === 'keydown');
            assert.equal(event.prevented, owned, code + ' browser default');
            assert.equal(widgetKeys, previousWidgets + Number(!owned), code + ' widget input');
        };
        const bothEdges = (code, owned = true) => {
            key('keydown', code, owned);
            key('keyup', code, owned);
        };

        bridge.set_host_input_enabled(false, true, 6, 1n);
        poll();
        assert.equal(JSON.parse(events.at(-1)[1]).focused, true);
        assert.equal(document.pointerLockElement, undefined, 'Gamepads must not lock the pointer');
        assert.ok(timerDelay <= 6, 'Gamepads without capture retain active polling cadence');
        document.activeElement = document.body = {};
        bothEdges('KeyA');
        bothEdges('ArrowUp');
        keyOwner = false;
        bothEdges('KeyB', false);
        keyOwner = true;

        bridge.set_host_input_enabled(true, true, 6, 1n);
        assert.equal(document.pointerLockElement, canvas);
        for (const code of ['KeyR', 'Escape', 'F2', 'F12', 'Tab', 'Space']) bothEdges(code);
        events.length = 0;
        backendReady = false;
        poll();
        canvas.fire('pointermove', {movementX: 20, movementY: 5});
        canvas.fire('pointerdown', {button: 1, buttons: 4, preventDefault() {}});
        assert.equal(document.pointerLockElement, null);
        assert.equal(bridge.take_pointer_capture_loss(), true);
        assert.equal(bridge.take_pointer_capture_loss(), false);
        backendReady = true;
        bridge.set_host_input_enabled(false, true, 6, 1n);
        assert.ok(events.some(event => event[0] === 'pads'), 'Contended controller sample survives release');
        assert.ok(events.some(event => event[0] === 'mouse-release'));
        assert.ok(!events.some(event => event[0] === 'motion' || event[0] === 'mouse'));
        bothEdges('KeyA');
        poll();
        assert.equal(events.at(-1)[0], 'pads');

        bridge.set_host_input_enabled(true, true, 6, 1n);
        document.exitPointerLock();
        assert.equal(bridge.take_pointer_capture_loss(), true);
        bothEdges('ArrowUp');
        bridge.set_host_input_enabled(false, true, 6, 1n);
        canvas.requestPointerLock = () => Promise.reject(new Error('Pointer lock rejected'));
        bridge.set_host_input_enabled(true, true, 6, 1n);
        await Promise.resolve();
        assert.equal(bridge.take_pointer_capture_loss(), true);
        bothEdges('Space');
        document.fire('pointerlockerror');
        assert.equal(bridge.take_pointer_capture_loss(), true);
        bothEdges('KeyR');

        key('keydown', 'KeyA', true);
        backendReady = false;
        focused = false;
        window.fire('blur');
        assert.equal(events.at(-1)[0], 'focus');
        assert.equal(events.at(-1)[4], 0);
        key('keyup', 'KeyA', false);
        assert.equal(bridge.host_release_pending(), true);
        document.exitPointerLock();
        focused = true;
        window.fire('focus');
        backendReady = true;
        poll();
        assert.ok(events.some(event => event[0] === 'release'), 'Pointer-lock loss must preserve a contended full release');
        assert.equal(bridge.host_release_pending(), false);
        bothEdges('KeyA');
        key('keydown', 'KeyA', true);
        document.hidden = true;
        document.fire('visibilitychange');
        assert.equal(events.at(-1)[0], 'release');
        key('keyup', 'KeyA', false);
        document.hidden = false;
        document.fire('visibilitychange');
        bothEdges('KeyA');

        poll();
        const admittedPad = JSON.parse(events.at(-1)[1]).devices[0];
        assert.deepEqual(admittedPad.axes, [0, 0]);
        assert.deepEqual(admittedPad.buttons, [false]);
        backendReady = false;
        controller.axes[0] = -1;
        controller.buttons[0].pressed = true;
        poll();
        assert.equal(bridge.host_release_pending(), false);
        const beforeHeldViewSwitch = events.length;
        backendReady = true;
        const heldAtSwitch = bridge.discard_buffered_host_controls();
        assert.ok(heldAtSwitch, 'View release must observe a held pad even when its callback was contended');
        assert.deepEqual(JSON.parse(heldAtSwitch), [{...admittedPad, axes: [-1, 0], buttons: [true]}]);
        assert.equal(events.length, beforeHeldViewSwitch, 'View release must not call the backend while its mailbox is held');
        poll();
        assert.deepEqual(events.slice(beforeHeldViewSwitch).map(event => event[0]), ['pads']);

        controller.axes[0] = 0;
        controller.buttons[0].pressed = false;
        const neutralAtSwitch = bridge.discard_buffered_host_controls();
        assert.deepEqual(JSON.parse(neutralAtSwitch), [admittedPad],
            'View release must see neutral even before the next poll');

        backendReady = false;
        focused = false;
        window.fire('blur');
        focused = true;
        window.fire('focus');
        poll();
        controller.buttons[0].pressed = true;
        poll();
        const beforeViewSwitch = events.length;
        bridge.discard_buffered_host_controls();
        assert.equal(bridge.host_release_pending(), true);
        backendReady = true;
        poll();
        assert.deepEqual(events.slice(beforeViewSwitch).map(event => event[0]), ['release', 'pads']);
        assert.equal(JSON.parse(events.at(-1)[1]).devices[0].buttons[0], true);
        assert.equal(bridge.host_release_pending(), false);

        bridge.set_host_input_enabled(false, false, 6, 1n);
        document.pointerLockElement = canvas;
        document.fire('pointerlockchange');
        assert.equal(document.pointerLockElement, null, 'A late lock completion cannot capture the mouse again');
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
