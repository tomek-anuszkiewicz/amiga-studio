import init, {start} from './gui.js';

try {
    if (!crossOriginIsolated || typeof SharedArrayBuffer !== 'function') {
        throw new Error('Shared-memory startup requires cross-origin isolation and SharedArrayBuffer');
    }
    const wasm = await init();
    if (!(wasm.memory.buffer instanceof SharedArrayBuffer)) {
        throw new Error('Build the browser application with shared Wasm memory');
    }
    await start();
} catch (error) {
    console.error(error);
    const message = document.createElement('p');
    message.textContent = `Unable to start Amiga Studio: ${error.message || error}`;
    message.style.cssText = 'color: white; padding: 1em; font-family: sans-serif';
    document.body.replaceChildren(message);
}
