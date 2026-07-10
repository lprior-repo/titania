//! Discover the target Rust project from a starting directory.
//!
//! Pure selection over caller-provided observations. The action layer walks
//! ancestor directories, reads Cargo manifests, and gathers typed metadata;
//! the selectors below choose the nearest workspace root (falling back to
//! the nearest non-workspace package manifest) without performing any
//! filesystem I/O of their own.

use camino::Utf8PathBuf;

use crate::error::TargetProjectError;

/// Manifest classification produced by parsing a Cargo.toml document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestStatus {
    /// Manifest declares an explicit `[workspace]` table.
    Workspace,
    /// Manifest declares an explicit `[package]` table.
    Package,
    /// Manifest parsed but has neither `[workspace]` nor `[package]`.
    Other,
    /// Manifest could not be parsed as TOML.
    Malformed,
}

/// Observed kind of a candidate manifest path on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestKind {
    /// Manifest exists and is a regular file.
    File,
    /// Manifest path exists but is a directory.
    Directory,
    /// Manifest path is absent.
    Missing,
}

/// One ancestor's manifest, already read and classified by the shell layer.
///
/// The shell produces these by walking the ancestor chain, attempting to
/// read each `Cargo.toml`, and classifying the result via
/// [`classify_manifest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestObservation {
    /// Absolute path of the ancestor directory whose `Cargo.toml` was read.
    pub root: Utf8PathBuf,
    /// Absolute path of the `Cargo.toml` file itself.
    pub manifest_path: Utf8PathBuf,
    /// Classification of the parsed manifest text.
    pub status: ManifestStatus,
}

/// One ancestor's filesystem metadata, gathered by the shell layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetObservation {
    /// Absolute path of the ancestor directory.
    pub root: Utf8PathBuf,
    /// Whether the ancestor directory exists and is a directory.
    pub root_is_dir: bool,
    /// Observed kind of `Cargo.toml` at this ancestor.
    pub manifest: ManifestKind,
}

impl TargetObservation {
    /// Construct an observation with the supplied metadata.
    #[must_use]
    pub const fn new(root: Utf8PathBuf, root_is_dir: bool, manifest: ManifestKind) -> Self {
        Self { root, root_is_dir, manifest }
    }
}

/// Select the nearest workspace root, falling back to the nearest package
/// or malformed manifest root.
///
/// Inputs must be ordered from nearest ancestor (typically the supplied
/// `cwd`) to the filesystem root.
///
/// # Errors
/// - [`TargetProjectError::NoCargoToml`] when no manifest was observed.
/// - [`TargetProjectError::MalformedCargoToml`] when the selected manifest
///   is malformed.
pub fn select_target_root(
    _observations: &[TargetObservation],
    manifests: &[ManifestObservation],
) -> Result<Utf8PathBuf, TargetProjectError> {
    if let Some(workspace) = manifests.iter().find(|m| m.status == ManifestStatus::Workspace) {
        return Ok(workspace.root.clone());
    }
    manifests
        .iter()
        .find(|m| matches!(m.status, ManifestStatus::Package | ManifestStatus::Malformed))
        .map_or(Err(TargetProjectError::NoCargoToml), selected_root_from_manifest)
}

/// Borrow the [`TargetObservation`] corresponding to the selected root.
///
/// # Errors
/// - [`TargetProjectError::NoCargoToml`] when no manifest was observed.
/// - [`TargetProjectError::MalformedCargoToml`] when the selected manifest
///   is malformed.
pub fn select_target_observation<'a>(
    observations: &'a [TargetObservation],
    manifests: &[ManifestObservation],
) -> Result<&'a TargetObservation, TargetProjectError> {
    let selected_root = select_target_root(observations, manifests)?;
    observations.iter().find(|o| o.root == selected_root).ok_or(TargetProjectError::NoCargoToml)
}

/// Resolve the selected manifest into its target root path.
///
/// # Errors
/// - [`TargetProjectError::MalformedCargoToml`] when the selected manifest
///   is malformed.
/// - [`TargetProjectError::NoCargoToml`] when the selected manifest is the
///   `Other` kind (parses cleanly but has no workspace or package table).
fn selected_root_from_manifest(
    manifest: &ManifestObservation,
) -> Result<Utf8PathBuf, TargetProjectError> {
    match manifest.status {
        ManifestStatus::Workspace | ManifestStatus::Package => Ok(manifest.root.clone()),
        ManifestStatus::Malformed => {
            Err(TargetProjectError::MalformedCargoToml { path: manifest.manifest_path.to_string() })
        }
        ManifestStatus::Other => Err(TargetProjectError::NoCargoToml),
    }
}

/// Classify a Cargo.toml document's text into a [`ManifestStatus`].
///
/// Exposed so the shell can reuse the exact same parsing rules without
/// duplicating them.
#[must_use]
pub fn classify_manifest(toml_text: &str) -> ManifestStatus {
    match toml_text.parse::<toml_edit::DocumentMut>() {
        Ok(doc) if has_explicit_table(&doc, "workspace") => ManifestStatus::Workspace,
        Ok(doc) if has_explicit_table(&doc, "package") => ManifestStatus::Package,
        Ok(_) => ManifestStatus::Other,
        Err(_) => ManifestStatus::Malformed,
    }
}

/// Returns `true` if the given Cargo.toml document has an explicit table.
///
/// TOML parsing prevents comments, strings, arrays, and implicit parent
/// tables such as `[workspace.metadata]` from being treated as roots.
fn has_explicit_table(doc: &toml_edit::DocumentMut, name: &str) -> bool {
    doc.get(name).and_then(toml_edit::Item::as_table).is_some_and(|table| !table.is_implicit())
}
