//! Live-workspace inventory: filesystem reads behind the pure audit.
//!
//! Reads [`PROOF_OBLIGATIONS_PATH`], lists `.rs`/`.vr` candidates under
//! [`PROOF_SOURCE_ROOT`], and audits with pure [`audit`](crate::proof_registry::audit::audit).

use std::path::Path;

use super::{
    audit::audit,
    classify::DiscoveredSource,
    registry::{PROOF_OBLIGATIONS_PATH, parse_registry},
};
use thiserror::Error;

/// Workspace-relative root scanned for unregistered proof sources.
pub const PROOF_SOURCE_ROOT: &str = "verification/verus";

/// File-read failure for one workspace-relative path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadError {
    /// Workspace-relative path that could not be read.
    pub path: String,
    /// Failure kind.
    pub kind: ReadErrorKind,
}

/// File-read failure kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadErrorKind {
    /// The file does not exist.
    NotFound,
    /// The file exists but could not be read.
    Io,
}

/// Filesystem error for a live workspace inventory.
///
/// # Errors
/// Returns [`WorkspaceAuditError`] when the registry cannot be read or
/// parsed, or when the proof source root cannot be listed.
#[derive(Debug, Error)]
pub enum WorkspaceAuditError {
    /// The registry file cannot be read; carries the I/O detail.
    #[error("cannot read proof-obligation registry: {0}")]
    RegistryUnreadable(String),
    /// The registry file does not match the registry schema.
    #[error(transparent)]
    RegistryInvalid(#[from] super::registry::RegistryError),
    /// The proof source root cannot be opened; carries the I/O detail.
    #[error("cannot open proof source root: {0}")]
    SourceRootUnreadable(String),
    /// The proof source root listing failed partway; carries the I/O detail.
    #[error("cannot list proof source root: {0}")]
    SourceListingFailed(String),
}

/// Run the inventory against a live workspace root.
///
/// Reads [`PROOF_OBLIGATIONS_PATH`], lists `.rs`/`.vr` candidates under
/// [`PROOF_SOURCE_ROOT`], and audits with pure [`audit`].
///
/// # Errors
/// Returns [`WorkspaceAuditError`] when the registry cannot be read or
/// parsed, or when the proof source root cannot be listed.
pub fn audit_workspace(root: &Path) -> Result<super::audit::AuditReport, WorkspaceAuditError> {
    let text = std::fs::read_to_string(root.join(PROOF_OBLIGATIONS_PATH))
        .map_err(|error| WorkspaceAuditError::RegistryUnreadable(error.to_string()))?;
    let registry = parse_registry(&text)?;
    let discovered = discover_sources(root)?;
    Ok(audit(|path| read_workspace_file(root, path), &registry, &discovered))
}

/// Read one workspace-relative file with a typed failure kind.
///
/// # Errors
/// Returns [`ReadError`] when the file is missing or cannot be read.
fn read_workspace_file(root: &Path, path: &str) -> Result<String, ReadError> {
    std::fs::read_to_string(root.join(path))
        .map_err(|error| ReadError { path: path.to_owned(), kind: read_error_kind(&error) })
}

/// Map an I/O error to its typed read-failure kind.
fn read_error_kind(error: &std::io::Error) -> ReadErrorKind {
    match error.kind() {
        std::io::ErrorKind::NotFound => ReadErrorKind::NotFound,
        _ => ReadErrorKind::Io,
    }
}

/// List `.rs`/`.vr` candidates directly under the proof source root.
///
/// # Errors
/// Returns [`WorkspaceAuditError`] when the source root cannot be opened or
/// its listing fails partway.
fn discover_sources(root: &Path) -> Result<Vec<DiscoveredSource>, WorkspaceAuditError> {
    let entries = std::fs::read_dir(root.join(PROOF_SOURCE_ROOT))
        .map_err(|error| WorkspaceAuditError::SourceRootUnreadable(error.to_string()))?
        .collect::<Result<Vec<std::fs::DirEntry>, std::io::Error>>()
        .map_err(|error| WorkspaceAuditError::SourceListingFailed(error.to_string()))?;
    Ok(entries.iter().filter_map(entry_source).collect())
}

/// Map one directory entry to a candidate source, if it qualifies.
fn entry_source(entry: &std::fs::DirEntry) -> Option<DiscoveredSource> {
    if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
        return None;
    }
    if !is_proof_candidate(entry) {
        return None;
    }
    let name = entry.file_name().to_string_lossy().into_owned();
    Some(DiscoveredSource { path: format!("{PROOF_SOURCE_ROOT}/{name}"), production: true })
}

/// `true` for `.rs` and `.vr` files, matched case-insensitively.
fn is_proof_candidate(entry: &std::fs::DirEntry) -> bool {
    entry.path().extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("rs") || extension.eq_ignore_ascii_case("vr")
    })
}
