use std::path::Path;

use super::{
    audit::audit,
    classify::DiscoveredSource,
    registry::{PROOF_OBLIGATIONS_PATH, parse_registry},
};
use thiserror::Error;

pub const PROOF_SOURCE_ROOT: &str = "verification/verus";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadError {
    pub path: String,
    pub kind: ReadErrorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadErrorKind {
    NotFound,
    Io,
}

#[derive(Debug, Error)]
pub enum WorkspaceAuditError {
    #[error("cannot read proof-obligation registry: {0}")]
    RegistryUnreadable(String),
    #[error(transparent)]
    RegistryInvalid(#[from] super::registry::RegistryError),
    #[error("cannot open proof source root: {0}")]
    SourceRootUnreadable(String),
    #[error("cannot list proof source root: {0}")]
    SourceListingFailed(String),
}

pub fn audit_workspace(root: &Path) -> Result<super::audit::AuditReport, WorkspaceAuditError> {
    let text = std::fs::read_to_string(root.join(PROOF_OBLIGATIONS_PATH))
        .map_err(|error| WorkspaceAuditError::RegistryUnreadable(error.to_string()))?;
    let registry = parse_registry(&text)?;
    let discovered = discover_sources(root)?;
    Ok(audit(|path| read_workspace_file(root, path), &registry, &discovered))
}

fn read_workspace_file(root: &Path, path: &str) -> Result<String, ReadError> {
    std::fs::read_to_string(root.join(path))
        .map_err(|error| ReadError { path: path.to_owned(), kind: read_error_kind(&error) })
}

fn read_error_kind(error: &std::io::Error) -> ReadErrorKind {
    match error.kind() {
        std::io::ErrorKind::NotFound => ReadErrorKind::NotFound,
        _ => ReadErrorKind::Io,
    }
}

fn discover_sources(root: &Path) -> Result<Vec<DiscoveredSource>, WorkspaceAuditError> {
    let entries = std::fs::read_dir(root.join(PROOF_SOURCE_ROOT))
        .map_err(|error| WorkspaceAuditError::SourceRootUnreadable(error.to_string()))?
        .collect::<Result<Vec<std::fs::DirEntry>, std::io::Error>>()
        .map_err(|error| WorkspaceAuditError::SourceListingFailed(error.to_string()))?;
    Ok(entries.iter().filter_map(entry_source).collect())
}

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

fn is_proof_candidate(entry: &std::fs::DirEntry) -> bool {
    entry.path().extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("rs") || extension.eq_ignore_ascii_case("vr")
    })
}
