//! Registry-completeness audit: pure coverage integrity, never proof of
//! executable correctness.
//!
//! A registered missing, unreadable, proof-free, or smoke-only production
//! obligation fails; a removed required row fails; a discovered unregistered
//! production proof requires disposition.

use super::{
    classify::{DiscoveredSource, SourceClass, classify_source},
    registry::{REQUIRED_PRODUCTION_OBLIGATIONS, Registry, RegistryAuthority, RegistryEntry},
};

/// One named registry-completeness failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuditFailure {
    /// A registered obligation file is absent.
    #[error("registered proof obligation missing: {obligation}")]
    RegisteredMissing {
        /// Stable obligation identity.
        obligation: String,
    },
    /// A registered obligation file exists but cannot be read.
    #[error("registered proof obligation unreadable: {obligation}")]
    RegisteredUnreadable {
        /// Stable obligation identity.
        obligation: String,
    },
    /// A registered obligation file carries no proof tokens.
    #[error("registered proof obligation is proof-free: {obligation}")]
    RegisteredProofFree {
        /// Stable obligation identity.
        obligation: String,
    },
    /// A smoke-only file is registered as a production obligation.
    #[error("registered production obligation is smoke-only: {obligation}")]
    RegisteredSmokeAsProduction {
        /// Stable obligation identity.
        obligation: String,
    },
    /// A required obligation row was removed from the registry.
    #[error("required production obligation removed from registry: {obligation}")]
    RequiredObligationRemoved {
        /// Stable obligation identity.
        obligation: String,
    },
    /// A production proof exists outside the registry without disposition.
    #[error("unregistered production proof requires disposition: {path}")]
    UnregisteredProductionProof {
        /// Discovered workspace-relative path.
        path: String,
    },
}

/// Registry-completeness audit outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReport {
    passed: Vec<String>,
    failures: Vec<AuditFailure>,
}

impl AuditReport {
    /// `true` when every obligation passed and nothing needs disposition.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.failures.is_empty()
    }

    /// Workspace-relative paths that passed, in registry order.
    #[must_use]
    pub fn passed(&self) -> &[String] {
        &self.passed
    }

    /// Named failures in registry, required-coverage, then discovery order.
    #[must_use]
    pub fn failures(&self) -> &[AuditFailure] {
        &self.failures
    }
}

/// Audit registered sources against required coverage and discoveries.
///
/// `read_file` resolves workspace-relative paths so behavior tests run
/// without touching the filesystem; `discovered` lists candidate `.rs`/`.vr`
/// sources found by the caller.
#[must_use]
pub fn audit(
    read_file: impl Fn(&str) -> Result<String, super::workspace::ReadError>,
    registry: &Registry,
    discovered: &[DiscoveredSource],
) -> AuditReport {
    let outcomes: Vec<Result<String, AuditFailure>> =
        registry.targets.iter().map(|entry| check_registered_entry(&read_file, entry)).collect();
    let passed: Vec<String> = outcomes.iter().filter_map(ok_ref).collect();
    let mut failures: Vec<AuditFailure> = outcomes.into_iter().filter_map(err_val).collect();
    failures.extend(check_required_rows(registry));
    failures.extend(check_discovered_sources(&read_file, registry, discovered));
    AuditReport { passed, failures }
}

/// Check one registered entry against its file content.
///
/// # Errors
/// Returns the named [`AuditFailure`] when the file is missing, unreadable,
/// proof-free, or smoke-only while registered as production.
fn check_registered_entry(
    read_file: impl Fn(&str) -> Result<String, super::workspace::ReadError>,
    entry: &RegistryEntry,
) -> Result<String, AuditFailure> {
    use super::workspace::ReadErrorKind as Kind;
    match read_file(entry.path.as_str()) {
        Err(error) if error.kind == Kind::NotFound => {
            Err(AuditFailure::RegisteredMissing { obligation: entry.path.clone() })
        }
        Err(_) => Err(AuditFailure::RegisteredUnreadable { obligation: entry.path.clone() }),
        Ok(content) => check_entry_content(entry, &content),
    }
}

/// Check fetched content against the entry authority.
///
/// # Errors
/// Returns the named [`AuditFailure`] for proof-free content and for
/// smoke-only content registered as production.
fn check_entry_content(entry: &RegistryEntry, content: &str) -> Result<String, AuditFailure> {
    match classify_source(content) {
        SourceClass::ProofFree => {
            Err(AuditFailure::RegisteredProofFree { obligation: entry.path.clone() })
        }
        SourceClass::SmokeOnly if entry.authority == RegistryAuthority::Production => {
            Err(AuditFailure::RegisteredSmokeAsProduction { obligation: entry.path.clone() })
        }
        SourceClass::GenuineProof | SourceClass::SmokeOnly => Ok(entry.path.clone()),
    }
}

/// Fail every required production obligation missing from the registry.
fn check_required_rows(registry: &Registry) -> Vec<AuditFailure> {
    REQUIRED_PRODUCTION_OBLIGATIONS
        .iter()
        .filter(|required| !registry_registers_production(registry, required))
        .map(|required| AuditFailure::RequiredObligationRemoved {
            obligation: (*required).to_owned(),
        })
        .collect()
}

/// `true` when the registry keeps the required path as production.
fn registry_registers_production(registry: &Registry, required: &str) -> bool {
    registry
        .targets
        .iter()
        .any(|entry| entry.path == required && entry.authority == RegistryAuthority::Production)
}

/// Fail discovered unregistered production proofs that need disposition.
fn check_discovered_sources(
    read_file: impl Fn(&str) -> Result<String, super::workspace::ReadError>,
    registry: &Registry,
    discovered: &[DiscoveredSource],
) -> Vec<AuditFailure> {
    discovered
        .iter()
        .filter(|source| source.production)
        .filter(|source| !registry_registers_any(registry, &source.path))
        .filter_map(|source| undiscovered_proof_failure(&read_file, source))
        .collect()
}

/// `true` when any registry row already accounts for the path.
fn registry_registers_any(registry: &Registry, path: &str) -> bool {
    registry.targets.iter().any(|entry| entry.path == path)
}

/// Fail when a discovered file carries genuine proof; ignore anything else.
fn undiscovered_proof_failure(
    read_file: impl Fn(&str) -> Result<String, super::workspace::ReadError>,
    source: &DiscoveredSource,
) -> Option<AuditFailure> {
    match read_file(source.path.as_str()) {
        Ok(content) if classify_source(&content) == SourceClass::GenuineProof => {
            Some(AuditFailure::UnregisteredProductionProof { path: source.path.clone() })
        }
        Ok(_) | Err(_) => None,
    }
}

/// Borrow the success path of an entry outcome.
fn ok_ref(outcome: &Result<String, AuditFailure>) -> Option<String> {
    outcome.as_ref().ok().cloned()
}

/// Move the failure out of an entry outcome.
fn err_val(outcome: Result<String, AuditFailure>) -> Option<AuditFailure> {
    outcome.err()
}
