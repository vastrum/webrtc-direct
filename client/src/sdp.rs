pub fn munge_offer(sdp: &str, ufrag: Ufrag) -> RtcSessionDescriptionInit {
    let mut parsed = ParsedSdp::parse(sdp);
    let replaced = parsed.set_ice_credentials(ufrag);
    assert_eq!(replaced, 2, "SDP offer missing ice-ufrag or ice-pwd");

    let description = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    description.set_sdp(&parsed.render());
    return description;
}

pub fn server_answer(
    addr: SocketAddr,
    fingerprint: Fingerprint,
    ufrag: Ufrag,
) -> RtcSessionDescriptionInit {
    let fp_sdp = fingerprint.to_sdp();
    let sdp = webrtc_direct_protocol::render(
        addr.ip(),
        addr.port(),
        ufrag,
        &fp_sdp,
        DtlsSetup::Passive,
        false,
    );
    let answer = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    answer.set_sdp(&sdp);
    return answer;
}

use std::net::SocketAddr;
use web_sys::{RtcSdpType, RtcSessionDescriptionInit};
use webrtc_direct_protocol::{DtlsSetup, Fingerprint, ParsedSdp, Ufrag};
