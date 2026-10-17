//! Trust/binding audit: source linkage versus production behavior verification.
//!
//! A `#[path]` import plus an `assume_specification` bridge is source
//! linkage, not proof of the target body. This module classifies each proof
//! obligation independently:
//!
//! - [`BindingVerdict::VerifiedProduction`] — a passing verifier run over the
//!   real executable body at the current production revision. Names its
//!   implementation binding and trusted base.
//! - [`BindingVerdict::SourceLinked`] — bound or explicitly waived linkage
//!   with no body proof. Allowed, but never production behavior verification.
//! - [`BindingVerdict::Blocked`] — the obligation fails closed
//!   ([`BlockReason`]): smoke fixtures, unapproved trust markers, missing
//!   bindings, mirror drift, vacuous proofs, or failed evidence runs.
//!
//! The audit is pure (`Data` + `Calculations` only): callers supply typed
//! obligations, scanned file contents, evidence records, waivers, and the
//! current production revision. It performs no I/O, runs no verifier, and
//! treats absent tools or evidence as a block, never a pass.

pub mod audit;
pub mod classify;
pub mod decide;
pub mod report;
pub mod scan;

pub use audit::{AuditInputs, AuditResult, Obligation, VerifierEvidence, Waiver, audit_all};
pub use classify::{BindingVerdict, BlockReason, TrustMarker};
pub use report::finding_for_blocked;
pub use scan::{FileSignals, ScannedFile, scan_proof_content};
