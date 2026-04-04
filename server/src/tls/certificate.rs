#[derive(Clone, Copy, Debug, BorshSerialize, BorshDeserialize)]
pub struct DtlsKey {
    key: [u8; 32],
}

impl DtlsKey {
    pub fn from_rng() -> Self {
        let mut key = [0u8; 32];
        rand::fill(&mut key);
        Self { key }
    }

    pub fn from_seed(seed: u64) -> Self {
        let mut key = [0u8; 32];
        key[24..32].copy_from_slice(&seed.to_be_bytes());
        Self { key }
    }

    pub fn from_bytes(key: [u8; 32]) -> Self {
        Self { key }
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.key
    }

    pub fn try_from_string(value: String) -> Option<Self> {
        let bytes = hex::decode(value).ok()?;
        let key: [u8; 32] = bytes.try_into().ok()?;
        Some(Self { key })
    }

    pub fn to_dtls_cert(&self) -> DtlsCert {
        let signing_key = SigningKey::from_bytes((&self.key).into()).unwrap();
        let validity = Validity {
            not_before: Time::UtcTime(
                UtcTime::from_unix_duration(std::time::Duration::ZERO).unwrap(),
            ),
            not_after: Time::INFINITY,
        };
        let pub_key = SubjectPublicKeyInfoOwned::from_key(*signing_key.verifying_key()).unwrap();

        let cert = CertificateBuilder::new(
            Profile::Leaf {
                issuer: Name::default(),
                enable_key_agreement: true,
                enable_key_encipherment: false,
            },
            SerialNumber::from(1u32),
            validity,
            Name::default(),
            pub_key,
            &signing_key,
        )
        .unwrap()
        .build::<p256::ecdsa::DerSignature>()
        .unwrap();

        DtlsCert {
            certificate: cert.to_der().unwrap(),
            private_key: signing_key.to_pkcs8_der().unwrap().as_bytes().to_vec(),
        }
    }

    pub fn fingerprint(&self) -> Fingerprint {
        Self::cert_fingerprint(&self.to_dtls_cert())
    }

    pub fn cert_fingerprint(cert: &DtlsCert) -> Fingerprint {
        let hash: [u8; 32] = Sha256::digest(&cert.certificate).into();
        Fingerprint::from_bytes(hash.to_vec()).unwrap()
    }
}

impl std::fmt::Display for DtlsKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use str0m::RtcConfig;

    #[test]
    fn to_dtls_cert_is_deterministic() {
        let key = DtlsKey::from_bytes([7u8; 32]);
        let cert_a = key.to_dtls_cert();
        let cert_b = key.to_dtls_cert();
        assert_eq!(cert_a.certificate, cert_b.certificate);
        assert_eq!(cert_a.private_key, cert_b.private_key);

        let fp_a = DtlsKey::cert_fingerprint(&cert_a);
        let fp_b = DtlsKey::cert_fingerprint(&cert_b);
        assert_eq!(fp_a, fp_b);
    }

    #[test]
    fn different_keys_different_fingerprints() {
        let cert_a = DtlsKey::from_bytes([1u8; 32]).to_dtls_cert();
        let cert_b = DtlsKey::from_bytes([2u8; 32]).to_dtls_cert();
        assert_ne!(DtlsKey::cert_fingerprint(&cert_a), DtlsKey::cert_fingerprint(&cert_b),);
    }

    #[test]
    fn fingerprint_matches_str0m() {
        let key = DtlsKey::from_bytes([42u8; 32]);
        let cert = key.to_dtls_cert();
        let our_fp = DtlsKey::cert_fingerprint(&cert);

        let mut rtc = RtcConfig::new().set_dtls_cert(cert).build(Instant::now());
        let api = rtc.direct_api();
        let str0m_fp = api.local_dtls_fingerprint();
        let str0m_fp = Fingerprint::from_bytes(str0m_fp.bytes.clone()).unwrap();

        assert_eq!(our_fp, str0m_fp);
    }
}

use borsh::{BorshDeserialize, BorshSerialize};
use p256::ecdsa::SigningKey;
use p256::pkcs8::EncodePrivateKey;
use sha2::{Digest, Sha256};
use str0m::config::DtlsCert;
use webrtc_direct_protocol::Fingerprint;
use x509_cert::builder::{Builder, CertificateBuilder, Profile};
use x509_cert::der::Encode;
use x509_cert::der::asn1::UtcTime;
use x509_cert::name::Name;
use x509_cert::serial_number::SerialNumber;
use x509_cert::spki::SubjectPublicKeyInfoOwned;
use x509_cert::time::{Time, Validity};
