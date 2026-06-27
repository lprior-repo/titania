//! A content-addressed digest used throughout Xtask for source, policy, and evidence.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A BLAKE3 digest with algorithm tag and lowercase hex encoding.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Digest {
    /// The hash algorithm used.
    pub algorithm: DigestAlgorithm,
    /// Lowercase hex string (64 chars for BLAKE3).
    pub hex: String,
}

/// Supported digest algorithms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DigestAlgorithm {
    /// BLAKE3 — the default for all Xtask digests.
    Blake3,
}

impl Digest {
    /// Compute a BLAKE3 digest from raw bytes.
    #[must_use]
    pub fn from_bytes(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self {
            algorithm: DigestAlgorithm::Blake3,
            hex: hash.to_hex().to_string(),
        }
    }

    /// Compute a BLAKE3 digest from a UTF-8 string.
    #[must_use]
    pub fn from_text(data: &str) -> Self {
        Self::from_bytes(data.as_bytes())
    }

    /// Compute a BLAKE3 digest from a serializable value using canonical JSON.
    ///
    /// # Errors
    /// Returns an error if the value cannot be serialized to JSON.
    pub fn from_serializable<T: Serialize>(value: &T) -> Result<Self, serde_json::Error> {
        let json = serde_json::to_string(value)?;
        Ok(Self::from_text(&json))
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algorithm, self.hex)
    }
}

impl fmt::Display for DigestAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Blake3 => write!(f, "blake3"),
        }
    }
}
