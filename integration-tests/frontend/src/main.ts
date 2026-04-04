import { run_stress_tests } from '../wasm/pkg';

try {
    const resp = await fetch('/webrtc-info');
    const info: { port: number; fingerprint: string } = await resp.json();
    console.log('WebRTC info:', JSON.stringify(info));

    await run_stress_tests(info.port, info.fingerprint);

    console.log('\nAll stress tests passed.');
    window.parent.postMessage({ type: 'test-result', status: 'success' }, '*');
} catch (err) {
    console.error('\nTest failed:', err);
    window.parent.postMessage({ type: 'test-result', status: 'failed' }, '*');
}
