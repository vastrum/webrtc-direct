pub struct DataStream {
    incoming_rx: mpsc::Receiver<Vec<u8>>,
    write_tx: mpsc::Sender<Vec<u8>>,
    buf: Vec<u8>,
}

impl DataStream {
    pub fn new(incoming_rx: mpsc::Receiver<Vec<u8>>, write_tx: mpsc::Sender<Vec<u8>>) -> Self {
        Self { incoming_rx, write_tx, buf: Vec::new() }
    }

    pub fn split(self) -> (FramedReader, FramedWriter) {
        let writer = FramedWriter::new(self.write_tx.clone());
        let reader = FramedReader::new(self);
        (reader, writer)
    }

    pub async fn read_exact(&mut self, out: &mut [u8]) -> Option<()> {
        let mut filled = 0;
        if !self.buf.is_empty() {
            let n = self.buf.len().min(out.len());
            out[..n].copy_from_slice(&self.buf[..n]);
            self.buf.drain(..n);
            filled = n;
        }
        while filled < out.len() {
            let data = self.incoming_rx.recv().await?;
            if data.is_empty() {
                return None;
            }
            let need = out.len() - filled;
            let take = data.len().min(need);
            out[filled..filled + take].copy_from_slice(&data[..take]);
            filled += take;
            if take < data.len() {
                self.buf.extend_from_slice(&data[take..]);
            }
        }
        return Some(());
    }
}

use super::framed::{FramedReader, FramedWriter};
use tokio::sync::mpsc;
