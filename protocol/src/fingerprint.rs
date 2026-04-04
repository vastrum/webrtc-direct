#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct Fingerprint([u8; 32]);

impl Serialize for Fingerprint {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Fingerprint {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let hex = String::deserialize(d)?;
        Ok(Self::from_hex(hex))
    }
}

impl Fingerprint {
    pub fn from_hex(hex: impl AsRef<str>) -> Self {
        let bytes: [u8; 32] = hex::decode(hex.as_ref())
            .unwrap()
            .try_into()
            .unwrap();
        Self(bytes)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Option<Self> {
        let bytes: [u8; 32] = bytes.try_into().ok()?;
        Some(Self(bytes))
    }

    pub fn from_sdp(sdp: &str) -> Self {
        let hex: String = sdp.replace(':', "").to_lowercase();
        Self::from_hex(hex)
    }

    pub fn to_sdp(&self) -> String {
        let upper = hex::encode_upper(self.0);
        upper.as_bytes().chunks(2).map(|c| str::from_utf8(c).unwrap()).collect::<Vec<_>>().join(":")
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
