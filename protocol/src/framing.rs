use crate::limits::MAX_MESSAGE_SIZE;

pub fn encode_framed(data: &[u8]) -> Vec<u8> {
    let compressed = lz4_flex::compress_prepend_size(data);
    let mut wire = Vec::with_capacity(4 + compressed.len());
    wire.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
    wire.extend_from_slice(&compressed);
    wire
}

pub fn decode_framed(data: &[u8]) -> Option<Vec<u8>> {
    lz4_flex::decompress_size_prepended(data).ok()
}

pub struct FrameBuf {
    buf: Vec<u8>,
}

impl FrameBuf {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn push(&mut self, data: &[u8]) -> Option<Vec<u8>> {
        self.buf.extend_from_slice(data);
        if self.buf.len() < 4 {
            return None;
        }
        let len = u32::from_be_bytes(self.buf[0..4].try_into().unwrap()) as usize;
        if len == 0 || len > MAX_MESSAGE_SIZE {
            self.buf.clear();
            return None;
        }
        if self.buf.len() < 4 + len {
            return None;
        }
        let payload = self.buf[4..4 + len].to_vec();
        self.buf.drain(..4 + len);
        decode_framed(&payload)
    }
}