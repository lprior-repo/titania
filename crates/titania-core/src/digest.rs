use core::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::DigestError;

const DIGEST_HEX_LEN: usize = 64;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Digest(String);

impl Digest {
    #[must_use]
    pub fn from_bytes(data: &[u8]) -> Self {
        Self(blake3::hash(data).to_hex().to_string())
    }

    pub fn from_hex(hex: &str) -> Result<Self, DigestError> {
        let bytes = hex.as_bytes();
        check_digest_len(bytes)?;
        check_digest_hex(bytes)?;
        Ok(Self(hex.to_owned()))
    }

    #[must_use]
    pub fn as_hex(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Digest({})", self.0)
    }
}

impl FromStr for Digest {
    type Err = DigestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s)
    }
}

impl Serialize for Digest {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = <std::borrow::Cow<'_, str> as Deserialize>::deserialize(de)?;
        Self::from_hex(&s).map_err(serde::de::Error::custom)
    }
}

fn check_digest_len(bytes: &[u8]) -> Result<(), DigestError> {
    (bytes.len() == DIGEST_HEX_LEN).then_some(()).ok_or(DigestError::WrongLength(bytes.len()))
}

fn check_digest_hex(bytes: &[u8]) -> Result<(), DigestError> {
    bytes
        .iter()
        .position(|&b| !is_lower_hex(b))
        .map_or(Ok(()), |index| Err(DigestError::NonHexChar(index)))
}

#[must_use]
const fn is_lower_hex(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'a'..=b'f')
}
