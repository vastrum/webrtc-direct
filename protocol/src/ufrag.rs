#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct Ufrag([u8; 32]);

impl Ufrag {
    pub fn random() -> Self {
        let mut bytes = [0u8; 32];
        rand::fill(&mut bytes);
        Self(bytes)
    }

    /// Parse from ICE wire format (64 hex chars).
    pub fn from_ice_str(s: &str) -> Option<Self> {
        let bytes: [u8; 32] = hex::decode(s).ok()?.try_into().ok()?;
        Some(Self(bytes))
    }

    /// Extract a ufrag from a STUN USERNAME attribute value (`"local:remote"`).
    pub fn from_stun_username(username: &str, local: bool) -> Option<Self> {
        let part = if local { username.split(':').next() } else { username.split(':').next_back() };
        part.and_then(Self::from_ice_str)
    }
}

impl fmt::Display for Ufrag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

use std::fmt;
