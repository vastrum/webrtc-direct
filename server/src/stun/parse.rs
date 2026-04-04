use bytecodec::DecodeExt;
use stun_codec::rfc5389::{attributes::Username, Attribute};
use stun_codec::MessageDecoder;
use webrtc_direct_protocol::Ufrag;

pub fn remote_ufrag(buf: &[u8]) -> Option<Ufrag> {
    let mut decoder = MessageDecoder::<Attribute>::new();
    let msg = decoder.decode_from_bytes(buf).ok()?.ok()?;
    let username: &Username = msg.get_attribute()?;
    Ufrag::from_stun_username(username.name(), false)
}
