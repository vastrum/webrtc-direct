pub struct WebRtcServer {
    accept_rx: mpsc::Receiver<(DataStream, SocketAddr)>,
}
impl WebRtcServer {
    pub async fn bind(listen_addr: SocketAddr, dtls_cert: DtlsCert) -> eyre::Result<Self> {
        let listener_sock = create_udp_listener(listen_addr)?;

        let (accept_tx, accept_rx) = mpsc::channel(limits::ACCEPT_CHANNEL_CAPACITY);

        tokio::spawn(listener::run(listener_sock, dtls_cert, accept_tx));

        Ok(Self { accept_rx })
    }

    pub async fn accept(&mut self) -> Option<(DataStream, SocketAddr)> {
        self.accept_rx.recv().await
    }
}

use crate::listener;
use crate::stream::DataStream;
use crate::utils::socket::create_udp_listener;
use std::net::SocketAddr;
use str0m::config::DtlsCert;
use tokio::sync::mpsc;
use webrtc_direct_protocol::limits;
