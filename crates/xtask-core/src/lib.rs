//! Core domain types for Xtask quality gate.
//!
//! This crate defines the report model, finding types, lane outcomes,
//! quality receipt, and all shared domain types used across Xtask.

pub mod digest;
pub mod finding;
pub mod lane;
pub mod location;
pub mod receipt;
pub mod report;

pub use digest::Digest;
pub use finding::{Finding, FindingEffect, RepairHint, RuleId, TextRange};
pub use lane::{
    CommandEvidence, GateScope, Lane, LaneEvidence, LaneFailure, LaneOutcome, LaneReceipt,
    ProcessTermination, SkipReason,
};
pub use location::{Location, WorkspacePath};
pub use receipt::QualityReceipt;
pub use report::{RejectKind, Report};
