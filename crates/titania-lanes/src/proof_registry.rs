pub mod audit;
pub mod classify;
pub mod registry;
pub mod scanner;
pub mod workspace;

pub use audit::{AuditFailure, AuditReport, audit};
pub use classify::{DiscoveredSource, SourceClass, classify_source};
pub use registry::{
    PROOF_OBLIGATIONS_PATH, REQUIRED_PRODUCTION_OBLIGATIONS, Registry, RegistryAuthority,
    RegistryEntry, RegistryError, parse_registry,
};
pub use workspace::{
    PROOF_SOURCE_ROOT, ReadError, ReadErrorKind, WorkspaceAuditError, audit_workspace,
};
