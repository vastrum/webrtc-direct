#[derive(Clone, Copy, Debug)]
pub enum DtlsSetup {
    Active,
    Passive,
    Actpass,
}

impl fmt::Display for DtlsSetup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Active => "active",
            Self::Passive => "passive",
            Self::Actpass => "actpass",
        })
    }
}
enum SdpLine {
    IceUfrag(String),
    IcePwd(String),
    Other(String),
}

pub struct ParsedSdp {
    lines: Vec<SdpLine>,
}

impl ParsedSdp {
    pub fn parse(sdp: &str) -> Self {
        let mut lines = Vec::new();
        for line in sdp.split('\n') {
            let line = line.strip_suffix('\r').unwrap_or(line);
            if line.is_empty() {
                continue;
            }

            if let Some(val) = line.strip_prefix("a=ice-ufrag:") {
                lines.push(SdpLine::IceUfrag(val.to_owned()));
            } else if let Some(val) = line.strip_prefix("a=ice-pwd:") {
                lines.push(SdpLine::IcePwd(val.to_owned()));
            } else {
                lines.push(SdpLine::Other(line.to_owned()));
            }
        }
        return Self { lines };
    }

    pub fn set_ice_credentials(&mut self, ufrag: Ufrag) -> usize {
        let val = ufrag.to_string();
        let mut count = 0;
        for line in &mut self.lines {
            match line {
                SdpLine::IceUfrag(v) | SdpLine::IcePwd(v) => {
                    *v = val.clone();
                    count += 1;
                }
                SdpLine::Other(_) => {}
            }
        }
        return count;
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        for line in &self.lines {
            match line {
                SdpLine::IceUfrag(v) => {
                    out.push_str("a=ice-ufrag:");
                    out.push_str(v);
                }
                SdpLine::IcePwd(v) => {
                    out.push_str("a=ice-pwd:");
                    out.push_str(v);
                }
                SdpLine::Other(s) => out.push_str(s),
            }
            out.push_str("\r\n");
        }
        return out;
    }
}

pub fn render(
    ip: IpAddr,
    port: u16,
    ufrag: Ufrag,
    fingerprint: &str,
    setup: DtlsSetup,
    ice_lite: bool,
) -> String {
    let v = if ip.is_ipv4() { "IP4" } else { "IP6" };
    let ice_lite_attr = if ice_lite { "a=ice-lite\r\n" } else { "" };
    let candidate = format!(
        "a=candidate:1467250027 1 UDP 1467250027 {ip} {port} typ host\r\n\
         a=end-of-candidates\r\n"
    );
    format!(
        "v=0\r\n\
         o=- 0 0 IN {v} {ip}\r\n\
         s=-\r\n\
         t=0 0\r\n\
         {ice_lite_attr}\
         m=application {port} UDP/DTLS/SCTP webrtc-datachannel\r\n\
         c=IN {v} {ip}\r\n\
         a=mid:0\r\n\
         a=ice-options:ice2\r\n\
         a=ice-ufrag:{ufrag}\r\n\
         a=ice-pwd:{ufrag}\r\n\
         a=fingerprint:sha-256 {fingerprint}\r\n\
         a=setup:{setup}\r\n\
         a=sctp-port:5000\r\n\
         a=max-message-size:{MAX_MESSAGE_SIZE}\r\n\
         {candidate}"
    )
}

use crate::limits::MAX_MESSAGE_SIZE;
use crate::ufrag::Ufrag;
use std::fmt;
use std::net::IpAddr;

#[cfg(test)]
mod tests {
    use super::*;

    const BROWSER_OFFER: &str = "\
        v=0\r\n\
        o=- 123 2 IN IP4 127.0.0.1\r\n\
        s=-\r\n\
        t=0 0\r\n\
        m=application 9 UDP/DTLS/SCTP webrtc-datachannel\r\n\
        c=IN IP4 0.0.0.0\r\n\
        a=mid:0\r\n\
        a=ice-ufrag:oldufrag\r\n\
        a=ice-pwd:oldpwd\r\n\
        a=fingerprint:sha-256 AA:BB:CC\r\n\
        a=setup:actpass\r\n\
        a=sctp-port:5000\r\n";

    #[test]
    fn round_trip_preserves_lines() {
        let sdp = ParsedSdp::parse(BROWSER_OFFER);
        let rendered = sdp.render();
        assert_eq!(rendered, BROWSER_OFFER);
    }

    #[test]
    fn set_ice_credentials_replaces_both() {
        let mut sdp = ParsedSdp::parse(BROWSER_OFFER);
        let ufrag = Ufrag::random();
        let ufrag_str = ufrag.to_string();
        let count = sdp.set_ice_credentials(ufrag);
        assert_eq!(count, 2);
        let rendered = sdp.render();
        assert!(rendered.contains(&format!("a=ice-ufrag:{ufrag_str}\r\n")));
        assert!(rendered.contains(&format!("a=ice-pwd:{ufrag_str}\r\n")));
        assert!(!rendered.contains("oldufrag"));
        assert!(!rendered.contains("oldpwd"));
    }

    #[test]
    fn parse_handles_lf_only() {
        let lf_sdp = "v=0\na=ice-ufrag:frag\na=ice-pwd:pw\na=setup:active\n";
        let sdp = ParsedSdp::parse(lf_sdp);
        let rendered = sdp.render();
        assert!(rendered.contains("a=ice-ufrag:frag\r\n"));
        assert!(rendered.contains("a=setup:active\r\n"));
    }

    #[test]
    fn set_ice_credentials_returns_zero_when_missing() {
        let mut sdp = ParsedSdp::parse("v=0\r\ns=-\r\n");
        let ufrag = Ufrag::random();
        assert_eq!(sdp.set_ice_credentials(ufrag), 0);
    }
}
