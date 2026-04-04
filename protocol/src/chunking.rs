use crate::limits::MAX_CHUNK_PAYLOAD;

pub fn split_chunks(data: &[u8]) -> impl Iterator<Item = &[u8]> {
    data.chunks(MAX_CHUNK_PAYLOAD)
}