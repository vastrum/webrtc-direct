import { connect, send_echo } from '../../client-wasm/pkg';

const connectBtn = document.getElementById('connect-btn') as HTMLButtonElement;
const sendBtn = document.getElementById('send-btn') as HTMLButtonElement;
const input = document.getElementById('msg-input') as HTMLInputElement;
const log = document.getElementById('log')!;

function appendLog(text: string) {
    log.textContent += text + '\n';
    log.scrollTop = log.scrollHeight;
}

connectBtn.addEventListener('click', async () => {
    try {
        connectBtn.textContent = 'Connecting...';
        connectBtn.disabled = true;
        const resp = await fetch('/webrtc-info');
        const info: { port: number; fingerprint: string } = await resp.json();
        appendLog(`Connecting to port ${info.port}...`);
        await connect(info.port, info.fingerprint);
        appendLog('Connected!');
        sendBtn.disabled = false;
        input.disabled = false;
        input.focus();
    } catch (err) {
        appendLog(`Connect failed: ${err}`);
        connectBtn.textContent = 'Connect';
        connectBtn.disabled = false;
    }
});

sendBtn.addEventListener('click', send);
input.addEventListener('keydown', (e) => { if (e.key === 'Enter') send(); });

async function send() {
    const msg = input.value.trim();
    if (!msg) return;
    appendLog(`> ${msg}`);
    input.value = '';
    try {
        const echo = await send_echo(msg);
        appendLog(`< ${echo}`);
    } catch (err) {
        appendLog(`Error: ${err}`);
    }
}
