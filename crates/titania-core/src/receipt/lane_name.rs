use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::ReceiptError;

/// Stable lane name stored in a [`super::ReceiptEnvelope`].
///
/// Empty names and NUL bytes are rejected so JSON receipts remain stable,
/// human-readable, and safe to pass through line-oriented tooling.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LaneName(String);

impl LaneName {
    /// Construct a validated lane name.
    ///
    /// # Errors
    /// - [`ReceiptError::EmptyLaneName`] if `name` is empty.
    /// - [`ReceiptError::InvalidLaneName`] if `name` contains a NUL byte.
    pub fn new(name: impl Into<String>) -> Result<Self, ReceiptError> {
        let name = name.into();
        check_lane_name_not_empty(&name)?;
        check_lane_name_no_nul(&name)?;
        Ok(Self(name))
    }

    /// Borrow the validated lane name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Check a lane name is not empty.
///
/// # Errors
/// Returns [`ReceiptError::EmptyLaneName`] when `name` is empty.
fn check_lane_name_not_empty(name: &str) -> Result<(), ReceiptError> {
    (!name.is_empty()).then_some(()).ok_or(ReceiptError::EmptyLaneName)
}

/// Check a lane name contains no NUL bytes.
///
/// # Errors
/// Returns [`ReceiptError::InvalidLaneName`] when `name` contains NUL.
fn check_lane_name_no_nul(name: &str) -> Result<(), ReceiptError> {
    (!name.as_bytes().contains(&b'\0')).then_some(()).ok_or(ReceiptError::InvalidLaneName)
}

impl fmt::Display for LaneName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for LaneName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LaneName({})", self.0)
    }
}

impl Serialize for LaneName {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for LaneName {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let name = <std::borrow::Cow<'_, str> as Deserialize>::deserialize(de)?;
        Self::new(name.into_owned()).map_err(serde::de::Error::custom)
    }
}
