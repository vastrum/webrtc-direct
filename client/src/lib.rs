//! WebRTC Direct client for the browser (WASM).
//!
//! Connects to a [`webrtc-direct-server`] using the server's socket address
//! and DTLS fingerprint, no signaling server required.
//!
//! # Usage
//!
//! ```rust,ignore
//! use webrtc_direct_client::{WebRtcClient, FramedClient, Fingerprint};
//! use std::net::SocketAddr;
//!
//! let addr: SocketAddr = ([127, 0, 0, 1], 3478).into();
//! let fp = Fingerprint::from_hex(hex_string);
//!
//! let raw = WebRtcClient::connect(addr, fp).await.unwrap();
//! let client = FramedClient::new(raw);
//!
//! client.send(b"hello").unwrap();
//! let response = client.recv().await.unwrap();
//! ```
//!
//! [`WebRtcClient`] gives raw DataChannel access. [`FramedClient`] wraps it
//! with length-prefixed LZ4-compressed framing, matching the server's
//! [`FramedReader`]/[`FramedWriter`].

pub mod client;
pub mod error;
pub mod framed;
pub mod sdp;
pub use client::WebRtcClient;
pub use error::WebRtcError;
pub use framed::FramedClient;
pub use webrtc_direct_protocol::{Fingerprint, Ufrag};
