pub async fn run(
    listener_sock: UdpSocket,
    dtls_cert: DtlsCert,
    accept_tx: mpsc::Sender<(DataStream, SocketAddr)>,
) {
    let listen_addr = listener_sock.local_addr().unwrap();
    let mut buf = vec![0u8; RECEIVE_MTU_BUFFER];
    let mut conn_limiter = ConnectionLimiter::new();

    loop {
        let Ok((n, addr)) = listener_sock.recv_from(&mut buf).await else {
            break;
        };

        let Some(guard) = conn_limiter.allow(addr.ip()) else {
            continue;
        };

        //create a udp socket for each connection instead of demuxing on single udp socket
        //the listener udp socket only handles new incoming connections
        let Ok(sock) = create_connected_udp_socket(listen_addr, addr) else {
            continue;
        };

        let data = &buf[..n];
        let dtls_cert = dtls_cert.clone();
        let accept_tx = accept_tx.clone();
        let initial_stun = data.to_vec();

        tokio::spawn(async move {
            let Ok(result) = Handshaker::start(sock, addr, &dtls_cert, &initial_stun).await else {
                return;
            };

            let stream = Connection::run(result, guard);
            let _ = accept_tx.try_send((stream, addr));
        });
    }
}

use crate::connection::Connection;
use crate::handshake::Handshaker;
use crate::stream::DataStream;
use crate::utils::connection_limit::ConnectionLimiter;
use crate::utils::socket::create_connected_udp_socket;
use std::net::SocketAddr;
use str0m::config::DtlsCert;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use webrtc_direct_protocol::limits::RECEIVE_MTU_BUFFER;
