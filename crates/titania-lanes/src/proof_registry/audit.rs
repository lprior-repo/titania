use super::{
    classify::{DiscoveredSource, SourceClass, classify_source},
    registry::{REQUIRED_PRODUCTION_OBLIGATIONS, Registry, RegistryAuthority, RegistryEntry},
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuditFailure {
    #[error("registered proof obligation missing: {obligation}")]
    RegisteredMissing { obligation: String },
    #[error("registered proof obligation unreadable: {obligation}")]
    RegisteredUnreadable { obligation: String },
    #[error("registered proof obligation is proof-free: {obligation}")]
    RegisteredProofFree { obligation: String },
    #[error("registered production obligation is smoke-only: {obligation}")]
    RegisteredSmokeAsProduction { obligation: String },
    #[error("required production obligation removed from registry: {obligation}")]
    RequiredObligationRemoved { obligation: String },
    #[error("unregistered production proof requires disposition: {path}")]
    UnregisteredProductionProof { path: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReport {
    passed: Vec<String>,
    failures: Vec<AuditFailure>,
}

impl AuditReport {
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.failures.is_empty()
    }

    #[must_use]
    pub fn passed(&self) -> &[String] {
        &self.passed
    }

    #[must_use]
    pub fn failures(&self) -> &[AuditFailure] {
        &self.failures
    }
}

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

fn check_required_rows(registry: &Registry) -> Vec<AuditFailure> {
    REQUIRED_PRODUCTION_OBLIGATIONS
        .iter()
        .filter(|required| !registry_registers_production(registry, required))
        .map(|required| AuditFailure::RequiredObligationRemoved {
            obligation: (*required).to_owned(),
        })
        .collect()
}

fn registry_registers_production(registry: &Registry, required: &str) -> bool {
    registry
        .targets
        .iter()
        .any(|entry| entry.path == required && entry.authority == RegistryAuthority::Production)
}

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

fn registry_registers_any(registry: &Registry, path: &str) -> bool {
    registry.targets.iter().any(|entry| entry.path == path)
}

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

fn ok_ref(outcome: &Result<String, AuditFailure>) -> Option<String> {
    outcome.as_ref().ok().cloned()
}

fn err_val(outcome: Result<String, AuditFailure>) -> Option<AuditFailure> {
    outcome.err()
}
