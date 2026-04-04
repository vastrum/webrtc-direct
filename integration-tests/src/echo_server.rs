use std::net::SocketAddr;
use webrtc_direct_server::{DtlsKey, WebRtcServer};

pub async fn start_echo_server() -> (u16, String) {
    let dtls_key = DtlsKey::from_rng();
    let dtls_cert = dtls_key.to_dtls_cert();
    let fingerprint = DtlsKey::cert_fingerprint(&dtls_cert).to_hex();
    let port = 3478;
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let mut server = WebRtcServer::bind(addr, dtls_cert).await.expect("Failed to bind echo server");

    tokio::spawn(async move {
        while let Some((conn, addr)) = server.accept().await {
            tokio::spawn(echo_handler(conn, addr));
        }
    });

    (port, fingerprint)
}

pub async fn echo_handler(conn: webrtc_direct_server::DataStream, _addr: SocketAddr) {
    let (mut reader, writer) = conn.split();
    while let Some(request) = reader.recv().await {
        let response = if let Ok(s) = str::from_utf8(&request) {
            if let Some(size_str) = s.strip_prefix("big:") {
                let size: usize = size_str.parse().unwrap_or(300_000);
                vec![b'X'; size]
            } else {
                request
            }
        } else {
            request
        };

        let _ = writer.send(&response).await;
    }
}
