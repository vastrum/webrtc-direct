pub struct Connection {
    rtc: Rtc,
    sock: UdpSocket,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    write_rx: mpsc::Receiver<Vec<u8>>,
    incoming_tx: mpsc::Sender<Vec<u8>>,
    channel_id: ChannelId,
    pending_writes: VecDeque<Vec<u8>>,
}

impl Connection {
    pub fn run(hs_result: HandshakeResult, conn_guard: ConnGuard) -> DataStream {
        let (write_tx, write_rx) = mpsc::channel(4);
        let (incoming_tx, incoming_rx) = mpsc::channel(64);
        let stream = DataStream::new(incoming_rx, write_tx);

        let mut conn = Connection {
            rtc: hs_result.rtc,
            sock: hs_result.sock,
            local_addr: hs_result.local_addr,
            remote_addr: hs_result.remote_addr,
            write_rx,
            incoming_tx,
            channel_id: hs_result.channel_id,
            pending_writes: Default::default(),
        };

        tokio::spawn(async move {
            let _guard = conn_guard;
            let _ = conn.data_loop().await;
            conn.rtc.disconnect();
        });

        stream
    }

    async fn data_loop(&mut self) -> eyre::Result<()> {
        let mut buf = vec![0u8; RECEIVE_MTU_BUFFER];

        loop {
            self.flush_pending_writes();
            let next_poll = self.process_rtc().await?;

            tokio::select! { biased;
                result = self.sock.recv(&mut buf) => {
                    let n = result?;
                    self.feed_rtc(&buf[..n])?;
                }
                cmd = self.write_rx.recv() => {
                    let Some(data) = cmd else { break };
                    self.handle_write(data);
                    while let Ok(data) = self.write_rx.try_recv() {
                        self.handle_write(data);
                    }
                }
                _ = time::sleep_until(next_poll) => {
                    self.tick_rtc()?;
                }
            }
        }

        Ok(())
    }

    async fn process_rtc(&mut self) -> eyre::Result<time::Instant> {
        loop {
            let output = self.poll_rtc()?;
            match output {
                Output::Timeout(deadline) => return Ok(deadline.into()),
                Output::Transmit(packet) => {
                    let _ = self.sock.send(&packet.contents).await;
                }
                Output::Event(event) => self.handle_event(event)?,
            }
        }
    }

    fn handle_event(&mut self, event: Event) -> eyre::Result<()> {
        match event {
            Event::ChannelData(data) => {
                let _ = self.incoming_tx.try_send(data.data);
                Ok(())
            }
            Event::IceConnectionStateChange(IceConnectionState::Disconnected) => {
                bail!("ice disconnected")
            }
            _ => Ok(()),
        }
    }

    fn poll_rtc(&mut self) -> eyre::Result<Output> {
        Ok(stacker::maybe_grow(4 * 1024 * 1024, 4 * 1024 * 1024, || self.rtc.poll_output())?)
    }

    fn tick_rtc(&mut self) -> eyre::Result<()> {
        Ok(stacker::maybe_grow(4 * 1024 * 1024, 4 * 1024 * 1024, || {
            self.rtc.handle_input(Input::Timeout(Instant::now()))
        })?)
    }

    fn feed_rtc(&mut self, data: &[u8]) -> eyre::Result<()> {
        let Ok(contents) = data.try_into() else { return Ok(()) };
        let receive = Receive {
            proto: Protocol::Udp,
            source: self.remote_addr,
            destination: self.local_addr,
            contents,
        };
        stacker::maybe_grow(4 * 1024 * 1024, 4 * 1024 * 1024, || {
            self.rtc.handle_input(Input::Receive(Instant::now(), receive))
        })?;
        Ok(())
    }

    fn handle_write(&mut self, data: Vec<u8>) {
        if !self.pending_writes.is_empty() {
            self.pending_writes.extend(split_chunks(&data).map(|c| c.to_vec()));
            return;
        }

        let chunks: Vec<&[u8]> = split_chunks(&data).collect();
        for (i, chunk) in chunks.iter().enumerate() {
            let Some(mut ch) = self.rtc.channel(self.channel_id) else { return };
            match ch.write(true, chunk) {
                Ok(true) => {}
                Ok(false) => {
                    self.pending_writes.extend(chunks[i..].iter().map(|c| c.to_vec()));
                    return;
                }
                Err(_) => return,
            }
        }
    }

    fn flush_pending_writes(&mut self) {
        while let Some(chunk) = self.pending_writes.front() {
            let Some(mut ch) = self.rtc.channel(self.channel_id) else { break };
            match ch.write(true, chunk) {
                Ok(true) => {
                    self.pending_writes.pop_front();
                }
                _ => break,
            }
        }
    }
}

use crate::handshake::HandshakeResult;
use crate::stream::DataStream;
use crate::utils::connection_limit::ConnGuard;
use eyre::bail;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::Instant;
use str0m::channel::ChannelId;
use str0m::net::{Protocol, Receive};
use str0m::{Event, IceConnectionState, Input, Output, Rtc};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::time;
use webrtc_direct_protocol::limits::RECEIVE_MTU_BUFFER;
use webrtc_direct_protocol::split_chunks;
