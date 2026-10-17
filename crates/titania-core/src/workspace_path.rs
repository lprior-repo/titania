use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use unicode_normalization::UnicodeNormalization;

use crate::error::WorkspacePathError;

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WorkspacePath(String);

impl WorkspacePath {
    pub fn new(s: &str) -> Result<Self, WorkspacePathError> {
        validate_workspace_path(s)?;
        Ok(Self(s.nfc().collect()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn segment_count(&self) -> usize {
        self.0.split('/').filter(|s| !s.is_empty()).count()
    }

    #[must_use]
    pub fn starts_with_segment(&self, segment: &str) -> bool {
        self.0.split('/').next().is_some_and(|first| first == segment)
    }
}

fn validate_workspace_path(s: &str) -> Result<(), WorkspacePathError> {
    check_non_empty(s)?;
    check_no_drive_absolute(s)?;
    check_no_backslash(s)?;
    check_no_unc(s)?;
    check_relative(s)?;
    check_no_null(s)?;
    check_no_control_bytes(s)?;
    check_no_dotdot(s)?;
    Ok(())
}

const fn check_non_empty(s: &str) -> Result<(), WorkspacePathError> {
    if s.is_empty() {
        return Err(WorkspacePathError::Empty);
    }
    Ok(())
}

fn check_no_drive_absolute(s: &str) -> Result<(), WorkspacePathError> {
    let bytes = s.as_bytes();
    let Some((first, rest)) = bytes.split_first() else {
        return Ok(());
    };
    if !first.is_ascii_alphabetic() || rest.first() != Some(&b':') {
        return Ok(());
    }
    let prefix = s.get(..2).map_or_else(|| format!("{first}:"), ToOwned::to_owned);
    Err(WorkspacePathError::DriveAbsolute(prefix))
}

fn check_relative(s: &str) -> Result<(), WorkspacePathError> {
    if s.starts_with('/') {
        return Err(WorkspacePathError::LeadingSlash);
    }
    Ok(())
}

fn check_no_backslash(s: &str) -> Result<(), WorkspacePathError> {
    if s.as_bytes().contains(&b'\\') {
        return Err(WorkspacePathError::ContainsBackslash);
    }
    Ok(())
}

fn check_no_unc(s: &str) -> Result<(), WorkspacePathError> {
    if s.starts_with("//") {
        return Err(WorkspacePathError::UncForm);
    }
    Ok(())
}

fn check_no_null(s: &str) -> Result<(), WorkspacePathError> {
    if s.as_bytes().contains(&0) {
        return Err(WorkspacePathError::ContainsNull);
    }
    Ok(())
}

fn check_no_control_bytes(s: &str) -> Result<(), WorkspacePathError> {
    match s.as_bytes().iter().find(|&&b| b < 0x20) {
        Some(&b) => Err(WorkspacePathError::ControlByte(b)),
        None => Ok(()),
    }
}

fn check_no_dotdot(s: &str) -> Result<(), WorkspacePathError> {
    if s.split('/').any(|seg| seg == "..") {
        return Err(WorkspacePathError::ContainsDotDot);
    }
    Ok(())
}

impl fmt::Display for WorkspacePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for WorkspacePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WorkspacePath({})", self.0)
    }
}

impl AsRef<str> for WorkspacePath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Serialize for WorkspacePath {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for WorkspacePath {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = <std::borrow::Cow<'_, str> as Deserialize>::deserialize(de)?;
        Self::new(&s).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<&str> for WorkspacePath {
    type Error = WorkspacePathError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::new(s)
    }
}
