pub struct FramedClient {
    inner: WebRtcClient,
    frame_buf: RefCell<FrameBuf>,
}

impl FramedClient {
    pub fn new(client: WebRtcClient) -> Self {
        Self { inner: client, frame_buf: RefCell::new(FrameBuf::new()) }
    }

    pub async fn recv(&self) -> Option<Vec<u8>> {
        loop {
            let raw = self.inner.recv_raw().await?;
            if let Some(msg) = self.frame_buf.borrow_mut().push(&raw) {
                return Some(msg);
            }
        }
    }

    pub fn send(&self, data: &[u8]) -> Result<(), WebRtcError> {
        let wire = encode_framed(data);
        self.inner.write_all(&wire)
    }
}

use crate::{WebRtcClient, WebRtcError};
use std::cell::RefCell;
use webrtc_direct_protocol::{FrameBuf, encode_framed};
