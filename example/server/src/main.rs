use axum::{Json, Router, response::Html, routing::get};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use webrtc_direct_server::{DtlsKey, WebRtcServer};

const WEBRTC_PORT: u16 = 3478;
const HTTP_PORT: u16 = 8080;

#[tokio::main]
async fn main() {
    let key = DtlsKey::from_rng();
    let fingerprint = key.fingerprint().to_hex();
    let cert = key.to_dtls_cert();

    // WebRTC echo server
    let addr: SocketAddr = ([0, 0, 0, 0], WEBRTC_PORT).into();
    let mut server = WebRtcServer::bind(addr, cert).await.unwrap();
    println!("WebRTC echo server on {addr}");
    println!("Fingerprint: {fingerprint}");

    tokio::spawn(async move {
        while let Some((conn, peer_addr)) = server.accept().await {
            println!("New connection from {peer_addr}");
            tokio::spawn(async move {
                let (mut reader, writer) = conn.split();
                while let Some(msg) = reader.recv().await {
                    let _ = writer.send(&msg).await;
                }
                println!("Disconnected: {peer_addr}");
            });
        }
    });

    // HTTP server
    let html = include_str!("../../frontend/dist/index.html");
    let info = serde_json::json!({ "port": WEBRTC_PORT, "fingerprint": fingerprint });

    let app = Router::new()
        .route("/", get(Html(html)))
        .route("/webrtc-info", get(Json(info)));

    let listener = TcpListener::bind(("0.0.0.0", HTTP_PORT)).await.unwrap();
    println!("Open http://127.0.0.1:{HTTP_PORT}");
    axum::serve(listener, app).await.unwrap();
}
