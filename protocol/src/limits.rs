use std::time::Duration;

pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024; //16mb

pub const MAX_CHUNK_PAYLOAD: usize = 60 * 1024; //60kb

pub const RECEIVE_MTU_BUFFER: usize = 8 * 1024; //8mb

pub const MAX_WEBRTC_CONNECTIONS: usize = 4096;

pub const MAX_CONNS_PER_IP: usize = 20;

pub const ACCEPT_CHANNEL_CAPACITY: usize = 64;

pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
