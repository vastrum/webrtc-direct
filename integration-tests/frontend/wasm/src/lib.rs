struct EchoTransport {
    inner: Rc<FramedClient>,
    pending: Rc<RefCell<VecDeque<oneshot::Sender<Vec<u8>>>>>,
}

impl EchoTransport {
    async fn connect(port: u16, fingerprint: Fingerprint) -> Result<Self, String> {
        let addr: SocketAddr = ([127, 0, 0, 1], port).into();

        let pending: Rc<RefCell<VecDeque<oneshot::Sender<Vec<u8>>>>> =
            Rc::new(RefCell::new(VecDeque::new()));

        let raw =
            WebRtcClient::connect(addr, fingerprint).await.map_err(|e| format!("connect: {e}"))?;
        let client = Rc::new(FramedClient::new(raw));

        spawn_echo_reader(client.clone(), pending.clone());

        Ok(Self { inner: client, pending })
    }

    /// Send data and return a receiver for the echo response.
    fn send(&self, data: &[u8]) -> Result<oneshot::Receiver<Vec<u8>>, String> {
        let (tx, rx) = oneshot::channel();
        self.pending.borrow_mut().push_back(tx);
        self.inner.send(data).map_err(|e| format!("send: {e}"))?;
        Ok(rx)
    }

    /// Send data and wait for the echo response.
    async fn echo(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        let rx = self.send(data)?;
        rx.await.map_err(|_| "echo: channel closed".to_string())
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub async fn run_stress_tests(port: u16, fingerprint: String) -> Result<(), JsError> {
    log("=== WebRTC Direct Stress Tests ===");
    let fp = Fingerprint::from_hex(fingerprint);

    log("[1/9] test_roundtrip_latency");
    test_roundtrip_latency(port, fp).await.map_err(to_js)?;

    log("[2/9] test_large_messages");
    test_large_messages(port, fp).await.map_err(to_js)?;

    log("[3/9] test_graduated_sizes");
    test_graduated_sizes(port, fp).await.map_err(to_js)?;

    log("[4/9] test_rapid_fire_500");
    test_rapid_fire_500(port, fp).await.map_err(to_js)?;

    log("[5/9] test_random_data_integrity");
    test_random_data_integrity(port, fp).await.map_err(to_js)?;

    log("[6/9] test_sustained_throughput");
    test_sustained_throughput(port, fp).await.map_err(to_js)?;

    log("[7/9] test_reconnect_storm");
    test_reconnect_storm(port, fp).await.map_err(to_js)?;

    log("[8/9] test_max_message_near_limit");
    test_max_message_near_limit(port, fp).await.map_err(to_js)?;

    log("[9/9] test_concurrent_sends");
    test_concurrent_sends(port, fp).await.map_err(to_js)?;

    log("=== All 9 stress tests passed! ===");
    Ok(())
}

// ---------------------------------------------------------------------------
// Individual tests
// ---------------------------------------------------------------------------

/// Measure round-trip latency: first message (cold) and subsequent messages (warm).
async fn test_roundtrip_latency(port: u16, fp: Fingerprint) -> Result<(), String> {
    let perf = web_sys::window().unwrap().performance().unwrap();

    // --- Connection timing ---
    let t0 = perf.now();
    let t = EchoTransport::connect(port, fp).await?;
    let connect_ms = perf.now() - t0;
    log(&format!("  [latency] connect: {connect_ms:.1}ms"));

    // --- First message (cold start - SCTP slow start, etc.) ---
    let t0 = perf.now();
    let resp = t.echo(b"ping-cold").await?;
    let cold_ms = perf.now() - t0;
    if resp != b"ping-cold" {
        return Err("cold ping mismatch".into());
    }
    log(&format!("  [latency] first msg (cold): {cold_ms:.1}ms"));

    // --- Warm-up: 5 messages to stabilize ---
    for i in 0..5 {
        let msg = format!("warmup-{i}");
        t.echo(msg.as_bytes()).await?;
    }

    // --- Measure 50 sequential small round-trips ---
    let mut latencies = Vec::with_capacity(50);
    for i in 0..50 {
        let msg = format!("lat-{i}");
        let t0 = perf.now();
        let resp = t.echo(msg.as_bytes()).await?;
        let elapsed = perf.now() - t0;
        if resp != msg.as_bytes() {
            return Err(format!("latency ping {i} mismatch"));
        }
        latencies.push(elapsed);
    }

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let min = latencies[0];
    let max = latencies[latencies.len() - 1];
    let median = latencies[latencies.len() / 2];
    let avg: f64 = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let p95 = latencies[(latencies.len() as f64 * 0.95) as usize];
    let p99 = latencies[(latencies.len() as f64 * 0.99) as usize];

    log(&format!(
        "  [latency] 50 small msgs: min={min:.1}ms avg={avg:.1}ms median={median:.1}ms p95={p95:.1}ms p99={p99:.1}ms max={max:.1}ms"
    ));

    // --- Individual latencies for analysis ---
    let first_10: Vec<String> = latencies.iter().take(10).map(|l| format!("{l:.1}")).collect();
    log(&format!("  [latency] sorted first 10: [{}]", first_10.join(", ")));

    // --- Measure 20 round-trips with 1KB payload ---
    let data_1k = vec![0xABu8; 1024];
    let mut latencies_1k = Vec::with_capacity(20);
    for _ in 0..20 {
        let t0 = perf.now();
        let resp = t.echo(&data_1k).await?;
        let elapsed = perf.now() - t0;
        if resp.len() != 1024 {
            return Err("1KB echo size mismatch".into());
        }
        latencies_1k.push(elapsed);
    }
    latencies_1k.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg_1k: f64 = latencies_1k.iter().sum::<f64>() / latencies_1k.len() as f64;
    log(&format!(
        "  [latency] 20 x 1KB msgs: min={:.1}ms avg={avg_1k:.1}ms max={:.1}ms",
        latencies_1k[0],
        latencies_1k.last().unwrap()
    ));

    // --- Measure 10 round-trips with 64KB payload (just over chunk boundary) ---
    let data_64k = vec![0xCDu8; 65536];
    let mut latencies_64k = Vec::with_capacity(10);
    for _ in 0..10 {
        let t0 = perf.now();
        let resp = t.echo(&data_64k).await?;
        let elapsed = perf.now() - t0;
        if resp.len() != 65536 {
            return Err("64KB echo size mismatch".into());
        }
        latencies_64k.push(elapsed);
    }
    latencies_64k.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg_64k: f64 = latencies_64k.iter().sum::<f64>() / latencies_64k.len() as f64;
    log(&format!(
        "  [latency] 10 x 64KB msgs: min={:.1}ms avg={avg_64k:.1}ms max={:.1}ms",
        latencies_64k[0],
        latencies_64k.last().unwrap()
    ));

    // --- Measure round-trip for 1.4MB payload (matches real page response size) ---
    let data_1_4m = vec![0xEFu8; 1_400_000];
    let t0 = perf.now();
    let resp = t.echo(&data_1_4m).await?;
    let elapsed_1_4m = perf.now() - t0;
    if resp.len() != 1_400_000 {
        return Err("1.4MB echo size mismatch".into());
    }
    log(&format!("  [latency] 1.4MB msg (warm): {elapsed_1_4m:.1}ms"));

    // --- Fresh connection + immediate 1.4MB (simulates first page load) ---
    let t0 = perf.now();
    let fresh = EchoTransport::connect(port, fp).await?;
    let connect_ms = perf.now() - t0;

    let t0 = perf.now();
    let resp = fresh.echo(&data_1_4m).await?;
    let cold_1_4m = perf.now() - t0;
    if resp.len() != 1_400_000 {
        return Err("cold 1.4MB echo size mismatch".into());
    }
    log(&format!(
        "  [latency] fresh connect: {connect_ms:.1}ms + 1.4MB cold: {cold_1_4m:.1}ms = {:.1}ms total",
        connect_ms + cold_1_4m
    ));

    log("  latency test OK");
    Ok(())
}

/// Echo 1MB, 5MB, 10MB payloads via big:SIZE, verify byte-for-byte.
async fn test_large_messages(port: u16, fp: Fingerprint) -> Result<(), String> {
    let t = EchoTransport::connect(port, fp).await?;

    for &size in &[1_000_000, 5_000_000, 10_000_000] {
        log(&format!("  echo big:{size}..."));
        let cmd = format!("big:{size}");
        let response = t.echo(cmd.as_bytes()).await?;
        assert_eq_len(response.len(), size, &format!("big:{size}"))?;
        assert_all_bytes(&response, b'X', &format!("big:{size}"))?;
        log(&format!("  big:{size} OK"));
    }
    Ok(())
}

/// Echo at chunk-boundary sizes: 60KB, 61KB (just over boundary), 128KB, 500KB, 1MB.
async fn test_graduated_sizes(port: u16, fp: Fingerprint) -> Result<(), String> {
    let t = EchoTransport::connect(port, fp).await?;

    // MAX_CHUNK_PAYLOAD = 61440 (60 KB)
    for &size in &[61_440, 61_441, 131_072, 500_000, 1_000_000] {
        log(&format!("  echo {size} bytes..."));
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let response = t.echo(&data).await?;
        assert_eq_len(response.len(), size, &format!("graduated {size}"))?;
        if response != data {
            return Err(format!("graduated {size}: data mismatch"));
        }
        log(&format!("  {size} bytes OK"));
    }
    Ok(())
}

/// Fire 500 small messages sequentially as fast as possible.
async fn test_rapid_fire_500(port: u16, fp: Fingerprint) -> Result<(), String> {
    let t = EchoTransport::connect(port, fp).await?;

    for i in 0u32..500 {
        let msg = format!("ping-{i}");
        let response = t.echo(msg.as_bytes()).await?;
        if response != msg.as_bytes() {
            return Err(format!("rapid fire mismatch at {i}"));
        }
        if i % 100 == 0 {
            log(&format!("  rapid fire: {i}/500"));
        }
    }
    log("  rapid fire: 500/500");
    Ok(())
}

/// Send 2MB of seeded random data, verify echo matches exactly.
async fn test_random_data_integrity(port: u16, fp: Fingerprint) -> Result<(), String> {
    let t = EchoTransport::connect(port, fp).await?;

    let size = 2_000_000;
    log(&format!("  sending {size} bytes of seeded random data..."));
    let data = seeded_random_bytes(size, 42);
    let response = t.echo(&data).await?;
    assert_eq_len(response.len(), size, "random data")?;
    if response != data {
        for (i, (a, b)) in data.iter().zip(response.iter()).enumerate() {
            if a != b {
                return Err(format!("random data mismatch at byte {i}: expected {a}, got {b}"));
            }
        }
        return Err("random data: length matches but data differs".into());
    }
    log("  random data integrity OK");
    Ok(())
}

/// 50 sequential 500KB round-trips (~25MB total). Tests for memory leaks / GC pressure.
async fn test_sustained_throughput(port: u16, fp: Fingerprint) -> Result<(), String> {
    let t = EchoTransport::connect(port, fp).await?;

    let size = 500_000;
    let rounds = 50;
    log(&format!("  {rounds} rounds of {size} bytes (~{}MB total)...", size * rounds / 1_000_000));

    for i in 0..rounds {
        let cmd = format!("big:{size}");
        let response = t.echo(cmd.as_bytes()).await?;
        assert_eq_len(response.len(), size, &format!("sustained round {i}"))?;
        if i % 10 == 0 {
            log(&format!("  sustained: {i}/{rounds}"));
        }
    }
    log(&format!("  sustained: {rounds}/{rounds}"));
    Ok(())
}

/// Connect → echo → drop → reconnect 10 times. Tests server connection cleanup.
async fn test_reconnect_storm(port: u16, fp: Fingerprint) -> Result<(), String> {
    for i in 0..10 {
        log(&format!("  reconnect {i}/10..."));
        let t = EchoTransport::connect(port, fp).await?;
        let msg = format!("reconnect-{i}");
        let response = t.echo(msg.as_bytes()).await?;
        if response != msg.as_bytes() {
            return Err(format!("reconnect {i}: mismatch"));
        }
        drop(t);
        // Small delay to let server clean up the previous connection
        gloo_timers::future::TimeoutFuture::new(100).await;
    }
    log("  reconnect storm OK");
    Ok(())
}

/// Echo 15MB (near chunk-count limit with ~245 chunks). Tests large allocation + chunk assembly.
async fn test_max_message_near_limit(port: u16, fp: Fingerprint) -> Result<(), String> {
    let t = EchoTransport::connect(port, fp).await?;

    let size = 15_000_000;
    log(&format!("  echo big:{size} (15MB)..."));
    let cmd = format!("big:{size}");
    let response = t.echo(cmd.as_bytes()).await?;
    assert_eq_len(response.len(), size, "max message")?;
    assert_all_bytes(&response, b'X', "max message")?;
    log("  15MB OK");
    Ok(())
}

/// Send 5 large messages (200KB each) before reading any responses.
/// Tests interleaved chunk assembly and backpressure handling.
async fn test_concurrent_sends(port: u16, fp: Fingerprint) -> Result<(), String> {
    let t = EchoTransport::connect(port, fp).await?;

    let size = 200_000;
    let count = 5;
    log(&format!("  sending {count} x {size} bytes concurrently..."));

    let mut receivers = Vec::new();
    let mut expected = Vec::new();

    for i in 0..count {
        let data: Vec<u8> = (0..size).map(|j| ((i * size + j) % 256) as u8).collect();
        let rx = t.send(&data)?;
        receivers.push(rx);
        expected.push(data);
    }

    for (i, rx) in receivers.into_iter().enumerate() {
        let response = rx.await.map_err(|_| format!("concurrent {i}: channel closed"))?;
        assert_eq_len(response.len(), size, &format!("concurrent {i}"))?;
        if response != expected[i] {
            return Err(format!("concurrent {i}: data mismatch"));
        }
        log(&format!("  concurrent {}/{count} verified", i + 1));
    }
    log("  concurrent sends OK");
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn spawn_echo_reader(
    client: Rc<FramedClient>,
    pending: Rc<RefCell<VecDeque<oneshot::Sender<Vec<u8>>>>>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        while let Some(msg) = client.recv().await {
            if let Some(tx) = pending.borrow_mut().pop_front() {
                let _ = tx.send(msg);
            }
        }
    });
}

fn log(msg: &str) {
    web_sys::console::log_1(&msg.into());
}

fn to_js(e: String) -> JsError {
    JsError::new(&e)
}

fn assert_eq_len(got: usize, expected: usize, ctx: &str) -> Result<(), String> {
    if got != expected {
        Err(format!("{ctx}: expected {expected} bytes, got {got}"))
    } else {
        Ok(())
    }
}

fn assert_all_bytes(data: &[u8], expected: u8, ctx: &str) -> Result<(), String> {
    if let Some(pos) = data.iter().position(|&b| b != expected) {
        Err(format!("{ctx}: byte at {pos} is {} (expected {expected})", data[pos]))
    } else {
        Ok(())
    }
}

/// Simple xorshift64 PRNG for reproducible test data.
fn seeded_random_bytes(len: usize, seed: u64) -> Vec<u8> {
    let mut state = seed;
    let mut bytes = Vec::with_capacity(len);
    for _ in 0..len {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        bytes.push(state as u8);
    }
    bytes
}

// ---------------------------------------------------------------------------
// Imports (project convention: at the bottom)
// ---------------------------------------------------------------------------

use std::cell::RefCell;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::rc::Rc;

use futures::channel::oneshot;
use wasm_bindgen::prelude::*;

use webrtc_direct_client::{Fingerprint, FramedClient, WebRtcClient};
