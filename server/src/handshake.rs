pub struct Handshaker {
    rtc: Rtc,
    sock: UdpSocket,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
}
impl Handshaker {
    pub async fn start(
        sock: UdpSocket,
        remote_addr: SocketAddr,
        dtls_cert: &DtlsCert,
        initial_stun: &[u8],
    ) -> eyre::Result<HandshakeResult> {
        let Some(remote_ufrag) = stun::remote_ufrag(initial_stun) else {
            return Err(eyre::eyre!("could not parse ufrag"));
        };

        let local_addr = sock.local_addr().unwrap();
        let rtc = Self::create_rtc(dtls_cert, &remote_ufrag, local_addr);
        let handshaker = Handshaker { rtc, sock, local_addr, remote_addr };
        return tokio::time::timeout(HANDSHAKE_TIMEOUT, handshaker.run(initial_stun)).await?;
    }

    async fn run(mut self, initial_stun: &[u8]) -> eyre::Result<HandshakeResult> {
        self.feed_rtc(initial_stun)?;

        let mut buf = vec![0u8; RECEIVE_MTU_BUFFER];
        let mut channel_id = None;

        loop {
            let next_poll = self.process_rtc(&mut channel_id).await?;

            if let Some(cid) = channel_id {
                return Ok(HandshakeResult {
                    rtc: self.rtc,
                    sock: self.sock,
                    local_addr: self.local_addr,
                    remote_addr: self.remote_addr,
                    channel_id: cid,
                });
            }

            tokio::select! { biased;
                result = self.sock.recv(&mut buf) => {
                    let n = result?;
                    self.feed_rtc(&buf[..n])?;
                }
                _ = tokio::time::sleep_until(next_poll) => {
                    self.tick_rtc()?;
                }
            }
        }
    }

    fn feed_rtc(&mut self, data: &[u8]) -> eyre::Result<()> {
        let Ok(contents) = data.try_into() else { return Ok(()) };
        let receive = Receive {
            proto: Protocol::Udp,
            source: self.remote_addr,
            destination: self.local_addr,
            contents,
        };
        // str0m processing can overflow tokios default 2 MB stack
        stacker::maybe_grow(4 * 1024 * 1024, 4 * 1024 * 1024, || {
            self.rtc.handle_input(Input::Receive(Instant::now(), receive))
        })?;
        Ok(())
    }

    fn poll_rtc(&mut self) -> eyre::Result<Output> {
        Ok(stacker::maybe_grow(4 * 1024 * 1024, 4 * 1024 * 1024, || self.rtc.poll_output())?)
    }

    fn tick_rtc(&mut self) -> eyre::Result<()> {
        Ok(stacker::maybe_grow(4 * 1024 * 1024, 4 * 1024 * 1024, || {
            self.rtc.handle_input(Input::Timeout(Instant::now()))
        })?)
    }

    async fn process_rtc(
        &mut self,
        channel_id: &mut Option<ChannelId>,
    ) -> eyre::Result<tokio::time::Instant> {
        loop {
            let output = self.poll_rtc()?;
            match output {
                Output::Timeout(deadline) => return Ok(deadline.into()),
                Output::Transmit(packet) => {
                    let _ = self.sock.send(&packet.contents).await;
                }
                Output::Event(event) => {
                    Self::handle_event(channel_id, event)?;
                }
            }
        }
    }

    fn handle_event(channel_id: &mut Option<ChannelId>, event: Event) -> eyre::Result<()> {
        let res = match event {
            Event::ChannelOpen(cid, _) => {
                *channel_id = Some(cid);
                Ok(())
            }
            Event::IceConnectionStateChange(IceConnectionState::Disconnected) => {
                bail!("ice disconnected")
            }
            _ => Ok(()),
        };
        return res;
    }

    /*
    https://libp2p.io/guides/browser-connectivity/#webrtc
    https://libp2p.io/blog/rust-libp2p-browser-webrtc/
    https://github.com/libp2p/rust-libp2p/tree/master/examples/browser-webrtc
    https://github.com/libp2p/specs/blob/master/webrtc/webrtc-direct.md
     */
    fn create_rtc(dtls_cert: &DtlsCert, remote_ufrag: &Ufrag, local_addr: SocketAddr) -> Rtc {
        let ice_creds =
            IceCreds { ufrag: remote_ufrag.to_string(), pass: remote_ufrag.to_string() };

        let mut rtc = RtcConfig::new()
            .set_ice_lite(true)
            .set_dtls_cert(dtls_cert.clone())
            .set_fingerprint_verification(false)
            .set_local_ice_credentials(ice_creds.clone())
            .build(Instant::now());

        {
            let mut api = rtc.direct_api();
            api.set_remote_ice_credentials(ice_creds);
            api.set_ice_controlling(false);
            let dummy_fp: Str0mFingerprint = "sha-256 FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF:FF".parse().unwrap();
            api.set_remote_fingerprint(dummy_fp);
            api.start_dtls(false).unwrap();
            api.start_sctp(false);
        }

        let candidate = Candidate::host(local_addr, "udp").unwrap();
        rtc.add_local_candidate(candidate);

        {
            let mut api = rtc.direct_api();
            api.create_data_channel(str0m::channel::ChannelConfig {
                negotiated: Some(0),
                ..Default::default()
            });
        }

        return rtc;
    }
}

pub struct HandshakeResult {
    pub rtc: Rtc,
    pub sock: UdpSocket,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub channel_id: ChannelId,
}

use crate::stun;
use eyre::bail;
use std::net::SocketAddr;
use std::time::Instant;
use str0m::channel::ChannelId;
use str0m::config::{DtlsCert, Fingerprint as Str0mFingerprint};
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, Event, IceConnectionState, IceCreds, Input, Output, Rtc, RtcConfig};
use tokio::net::UdpSocket;
use webrtc_direct_protocol::Ufrag;
use webrtc_direct_protocol::limits::{HANDSHAKE_TIMEOUT, RECEIVE_MTU_BUFFER};
