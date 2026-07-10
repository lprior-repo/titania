//! External Rust project on disk. A validated, absolute, UTF-8 directory
//! path containing a `Cargo.toml` file.
//!
//! Distinct from [`crate::WorkspacePath`]: `WorkspacePath` is a validated
//! relative path *string* used for human-readable output and stable
//! cross-platform hashing of finding locations. `TargetProject` is a real
//! filesystem path representing the project being judged — discovered
//! from CWD by the shell layer's [`crate::discover::select_target_root`]
//! pure selector, paired with shell-supplied ancestor observations.
//!
//! Invariants enforced by construction:
//! - Absolute.
//! - Valid UTF-8.
//! - The path's directory metadata and manifest metadata are supplied by
//!   the caller (the shell layer reads them from the filesystem).
//!
//! Construction is total: every public constructor returns a `Result`;
//! there is no public API that produces an invalid value.

use core::fmt;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    discover::{ManifestKind, TargetObservation},
    error::TargetProjectError,
};

/// A validated absolute UTF-8 path to a directory containing a
/// `Cargo.toml` file.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TargetProject(Utf8PathBuf);

impl TargetProject {
    /// Validate a [`TargetProject`] from a shell-supplied filesystem
    /// observation.
    ///
    /// Pure: the caller passes the directory and manifest kind it
    /// observed for the ancestor root. This is the constructor the
    /// shell's `discover_target` uses.
    ///
    /// # Errors
    /// - [`TargetProjectError::NotUtf8`] if the underlying path bytes are
    ///   not UTF-8 (cannot happen when the observation's `root` was built
    ///   from a UTF-8 string, but the contract preserves the previous
    ///   error variant for surface compatibility).
    /// - [`TargetProjectError::Empty`] if the path is empty.
    /// - [`TargetProjectError::NonAbsolute`] if the path is relative.
    /// - [`TargetProjectError::NotFound`] if the directory does not exist.
    /// - [`TargetProjectError::NotADirectory`] if the directory path is
    ///   not a directory.
    /// - [`TargetProjectError::NoCargoToml`] if `Cargo.toml` is absent.
    /// - [`TargetProjectError::CargoTomlNotFile`] if `Cargo.toml` exists
    ///   but is not a file.
    pub fn try_from_observation(
        observation: &TargetObservation,
    ) -> Result<Self, TargetProjectError> {
        Self::try_from_validated_path(
            &observation.root,
            observation.root_is_dir,
            observation.manifest,
        )
    }

    /// Validate a [`TargetProject`] from caller-supplied absolute path and
    /// observed manifest kind.
    ///
    /// Pure constructor used by the shell layer (via
    /// [`Self::try_from_observation`]) and by tests that want to exercise
    /// the validation rules without touching the filesystem.
    ///
    /// # Errors
    /// - [`TargetProjectError::Empty`] if the path is empty.
    /// - [`TargetProjectError::NonAbsolute`] if the path is relative.
    /// - [`TargetProjectError::NotFound`] if `root_is_dir` is `false`.
    /// - [`TargetProjectError::CargoTomlNotFile`] if `manifest` is
    ///   `Directory`.
    /// - [`TargetProjectError::NoCargoToml`] if `manifest` is `Missing`.
    pub fn try_from_validated_path(
        path: &Utf8Path,
        root_is_dir: bool,
        manifest: ManifestKind,
    ) -> Result<Self, TargetProjectError> {
        validate_not_empty(path)?;
        validate_absolute(path)?;
        ensure_root_directory(root_is_dir)?;
        match manifest {
            ManifestKind::File => Ok(Self(path.to_owned())),
            ManifestKind::Directory => Err(TargetProjectError::CargoTomlNotFile),
            ManifestKind::Missing => Err(TargetProjectError::NoCargoToml),
        }
    }

    /// Validate a [`TargetProject`] from a UTF-8 path string at a
    /// boundary that has no filesystem access (typically JSON
    /// deserialization).
    ///
    /// Pure: only path-shape invariants are enforced. Existence and
    /// manifest presence are not checked; the shell layer does those
    /// checks via [`Self::try_from_observation`].
    ///
    /// # Errors
    /// - [`TargetProjectError::NotUtf8`] if the path bytes are not UTF-8
    ///   (the input is already a `Utf8Path`, so this is a defensive
    ///   error path; in practice it cannot trigger here).
    /// - [`TargetProjectError::Empty`] if the path is empty.
    /// - [`TargetProjectError::NonAbsolute`] if the path is relative.
    pub fn try_from_path_string(path: &Utf8Path) -> Result<Self, TargetProjectError> {
        validate_not_empty(path)?;
        validate_absolute(path)?;
        Ok(Self(path.to_owned()))
    }

    /// Borrow the underlying path as a [`Utf8Path`].
    #[must_use]
    pub fn as_path(&self) -> &Utf8Path {
        &self.0
    }

    /// Path to the manifest: `{root}/Cargo.toml`.
    #[must_use]
    pub fn manifest_path(&self) -> Utf8PathBuf {
        self.0.join("Cargo.toml")
    }

    /// Borrow the underlying path as a [`std::path::Path`].
    #[must_use]
    pub fn as_std_path(&self) -> &std::path::Path {
        self.0.as_std_path()
    }
}

/// Reject an empty path.
///
/// # Errors
/// Returns [`TargetProjectError::Empty`] when `path` is empty.
fn validate_not_empty(path: &Utf8Path) -> Result<(), TargetProjectError> {
    if path.as_str().is_empty() { Err(TargetProjectError::Empty) } else { Ok(()) }
}

/// Reject a relative path.
///
/// # Errors
/// Returns [`TargetProjectError::NonAbsolute`] when `path` is relative.
fn validate_absolute(path: &Utf8Path) -> Result<(), TargetProjectError> {
    if path.is_absolute() { Ok(()) } else { Err(TargetProjectError::NonAbsolute(path.to_string())) }
}

/// Reject an observation whose root is not a directory.
///
/// # Errors
/// Returns [`TargetProjectError::NotFound`] when `root_is_dir` is `false`.
fn ensure_root_directory(root_is_dir: bool) -> Result<(), TargetProjectError> {
    root_is_dir.then_some(()).ok_or(TargetProjectError::NotFound)
}

impl fmt::Display for TargetProject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl fmt::Debug for TargetProject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TargetProject").field(&self.0.as_str()).finish()
    }
}

impl AsRef<Utf8Path> for TargetProject {
    fn as_ref(&self) -> &Utf8Path {
        &self.0
    }
}

impl Serialize for TargetProject {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self.0.as_str())
    }
}

impl<'de> Deserialize<'de> for TargetProject {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = <std::borrow::Cow<'_, str> as Deserialize>::deserialize(de)?;
        let p = Utf8PathBuf::from(s.into_owned());
        Self::try_from_path_string(p.as_path()).map_err(serde::de::Error::custom)
    }
}
