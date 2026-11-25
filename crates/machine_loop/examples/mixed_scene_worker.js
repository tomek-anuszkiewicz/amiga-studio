import init, {run_mixed_scene} from './machine_loop.js';

try {
    if (!crossOriginIsolated) throw new Error('Missing cross-origin isolation');
    await init();
    postMessage({report: JSON.parse(run_mixed_scene())});
} catch (error) {
    postMessage({error: String(error)});
}
