# webrtc-direct

Browser-to-server communication over WebRTC DataChannels, without depending on external TURN/STUN server to start connection.

webrtc-direct connects directly to the server given a DTLS fingerprint and publicly accesible IP endpoint.

This is intended for a browser-wasm Rust client that connects to Rust server. This could be used to implement P2P clients that directly connect to nodes from the browser without introducing dependencies on external TURN/STUN servers.

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

let addr: SocketAddr = ([127, 0, 0, 1], 3478).into();
let fingerprint = Fingerprint::from_hex(server_fingerprint_hex);

let raw = WebRtcClient::connect(addr, fingerprint).await.unwrap();
let client = FramedClient::new(raw);

client.send(b"hello").unwrap();
let response = client.recv().await.unwrap();
```


Based on [libp2p WebRTC Direct](https://github.com/libp2p/specs/blob/master/webrtc/webrtc-direct.md)

## License

MIT
