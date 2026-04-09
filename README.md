# webrtc-direct


Crate for connecting a Browser to a Server directly using WebRTC without requiring a TLS cert / domain name from the Server or requiring intermediary TURN servers.  

[![crates.io](https://img.shields.io/crates/v/webrtc-direct-server)](https://crates.io/crates/webrtc-direct-server)

## Why

Traditional WebRTC requires signaling servers and TURN relays. This is centralized infrastructure which creates single points of failure. 

Using webrtc-direct a browser can directly connect to any server that publishes its IP and fingerprint. This makes it practical to build decentralized P2P networks where browsers talk directly to nodes without depending on external services.

## How it works


Based on [libp2p WebRTC Direct](https://github.com/libp2p/specs/blob/master/webrtc/webrtc-direct.md)

1. The server generates a DTLS key and derives a SHA-256 fingerprint from its certificate
2. The fingerprint is shared with the client (embedded in HTML, fetched from an endpoint, hardcoded)
3. The browser constructs an SDP offer locally, generates the SDP answer itself using the server IP + fingerprint, and initiates a direct DTLS connection
4. The server uses ICE lite (no candidate gathering) and accepts the connection on a single UDP port
5. A negotiated DataChannel (ID 0) opens without any channel negotiation signaling needed


## Crates

| Crate | Description |
|-------|-------------|
| [`webrtc-direct-server`](https://crates.io/crates/webrtc-direct-server) | WebRTC server |
| [`webrtc-direct-client`](https://crates.io/crates/webrtc-direct-client) | Browser WASM client |
| [`webrtc-direct-protocol`](https://crates.io/crates/webrtc-direct-protocol) | Shared protocol |

## Quick start

### Server (native Rust)

```rust
use webrtc_direct_server::{DtlsKey, WebRtcServer};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let key = DtlsKey::from_rng();
    let cert = key.to_dtls_cert();
    let fingerprint = key.fingerprint();

    println!("fingerprint: {}", fingerprint.to_hex());

    let addr: SocketAddr = ([0, 0, 0, 0], 3478).into();
    let mut server = WebRtcServer::bind(addr, cert).await.unwrap();

    while let Some((conn, peer_addr)) = server.accept().await {
        tokio::spawn(async move {
            let (mut reader, writer) = conn.split();
            while let Some(msg) = reader.recv().await {
                let _ = writer.send(&msg).await; // echo
            }
        });
    }
}
```

### Client (browser WASM)

```rust
use webrtc_direct_client::{WebRtcClient, FramedClient, Fingerprint};
use std::net::SocketAddr;

// server_fingerprint_hex: obtained from the server (for example fetched from an HTTP endpoint or embedded inside the HTML)
let addr: SocketAddr = ([127, 0, 0, 1], 3478).into();
let fingerprint = Fingerprint::from_hex(server_fingerprint_hex);

let raw = WebRtcClient::connect(addr, fingerprint).await.unwrap();
let client = FramedClient::new(raw);

client.send(b"hello").unwrap();
let response = client.recv().await.unwrap();
```

## Full working example

See [`example/`](example/) for a complete runnable echo demo with server, WASM client, and browser frontend. To run:

```bash
cd example && make run
```

## Requirements

- Rust 1.85+ (edition 2024)
- For the client: `wasm-pack`, `wasm32-unknown-unknown` target

## License

MIT
