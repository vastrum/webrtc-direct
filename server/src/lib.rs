//! WebRTC Direct server for native Rust (Tokio).
//!
//! Accepts browser WebRTC DataChannel connections on a single UDP socket
//! without any signaling server. The browser connects using the server's
//! address and DTLS fingerprint.
//!
//! # Usage
//!
//! ```rust,no_run
//! use webrtc_direct_server::{DtlsKey, WebRtcServer};
//! use std::net::SocketAddr;
//!
//! # async fn example() {
//! let key = DtlsKey::from_rng();
//! let cert = key.to_dtls_cert();
//! let addr: SocketAddr = ([0, 0, 0, 0], 3478).into();
//! let mut server = WebRtcServer::bind(addr, cert).await.unwrap();
//!
//! while let Some((stream, peer)) = server.accept().await {
//!     let (mut reader, writer) = stream.split();
//!     // reader.recv() / writer.send() for framed messages
//! }
//! # }
//! ```

mod connection;
mod handshake;
mod listener;
mod server;
mod stream;
mod stun;
mod tls;
mod utils;

pub use server::WebRtcServer;
pub use stream::DataStream;
pub use stream::FramedReader;
pub use stream::FramedWriter;
pub use tls::DtlsKey;
pub use utils::socket::local_network_ip;
