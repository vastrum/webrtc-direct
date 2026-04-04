//! Shared wire protocol for WebRTC Direct connections.
//!
//! Provides the framing, chunking, compression, and SDP types used by both
//! the native server and the browser WASM client.
//!
//! # Wire format
//!
//! Messages are length-prefixed and LZ4-compressed:
//!
//! ```text
//! Frame = len:u32be ++ lz4_compress(payload)
//! ```
//!
//! Large frames are split into 60 KB DataChannel chunks via [`split_chunks`].
//!
//! # Limits
//!
//! - Max message: 16 MB ([`MAX_MESSAGE_SIZE`])
//! - Max chunk: 60 KB ([`MAX_CHUNK_PAYLOAD`])

pub mod chunking;
pub mod fingerprint;
pub mod framing;
pub mod limits;
pub mod sdp;
pub mod ufrag;

pub use chunking::split_chunks;
pub use fingerprint::Fingerprint;
pub use framing::{FrameBuf, decode_framed, encode_framed};
pub use limits::{MAX_CHUNK_PAYLOAD, MAX_MESSAGE_SIZE};
pub use sdp::{DtlsSetup, ParsedSdp, render};
pub use ufrag::Ufrag;
