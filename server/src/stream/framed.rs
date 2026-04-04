#[derive(Clone)]
pub struct FramedWriter {
    write_tx: mpsc::Sender<Vec<u8>>,
}

impl FramedWriter {
    pub fn new(write_tx: mpsc::Sender<Vec<u8>>) -> Self {
        Self { write_tx }
    }

    pub async fn send(&self, data: &[u8]) -> eyre::Result<()> {
        let wire = encode_framed(data);
        self.write_tx.send(wire).await?;
        Ok(())
    }
}

pub struct FramedReader {
    inner: DataStream,
}

impl FramedReader {
    pub fn new(inner: DataStream) -> Self {
        Self { inner }
    }

    pub async fn recv(&mut self) -> Option<Vec<u8>> {
        let mut hdr = [0u8; 4];
        self.inner.read_exact(&mut hdr).await?;
        let len = u32::from_be_bytes(hdr) as usize;
        if len == 0 || len > limits::MAX_MESSAGE_SIZE {
            return None;
        }
        let mut payload = vec![0u8; len];
        self.inner.read_exact(&mut payload).await?;
        decode_framed(&payload)
    }
}

use super::DataStream;
use tokio::sync::mpsc;
use webrtc_direct_protocol::{decode_framed, encode_framed, limits};
