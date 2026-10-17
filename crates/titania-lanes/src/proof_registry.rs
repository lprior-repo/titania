//! Authoritative proof-registry completeness audit for `.rs` and `.vr` sources.
//!
//! The inventory audits registered proof sources against the required
//! obligation registry in [`PROOF_OBLIGATIONS_PATH`]. A registered missing,
//! unreadable, proof-free, or smoke-only production obligation fails; a
//! removed required row fails; a discovered unregistered production proof
//! requires disposition. This is coverage integrity only — never proof of
//! executable correctness.
//!
//! Fail-closed rules live in the submodules: unknown registry fields are
//! rejected, proof words inside comments or strings never count, an explicit
//! smoke tag wins over proof tokens, and over-stripping fails loud.

/// Registry-completeness audit outcome and named failures.
pub mod audit;
/// Proof-source content classification by tokens, never by extension.
pub mod classify;
/// Registry-document model and fail-closed parser.
pub mod registry;
/// Single-pass comment/string-skipping scanner.
pub mod scanner;
/// Live-workspace inventory behind the pure audit.
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
