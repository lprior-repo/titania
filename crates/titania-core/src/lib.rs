//! Pure domain types for titania-check. Zero IO, zero async, zero unsafe.
//!
//! Each public type has a smart constructor that returns a `Result`. Once
//! constructed, all invariants are type-enforced: there is no way to produce
//! an invalid value of these types without going through the constructor.
//!
//! See `crates/titania-core/src/*.rs` for the primitive definitions and
//! `crates/titania-core/tests/*.rs` for the property- and behavior-tests.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::string_slice)]
#![deny(clippy::get_unwrap)]
#![deny(clippy::arithmetic_side_effects)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::as_conversions)]
#![forbid(unsafe_code)]

mod artifact;
mod diagnostic;
mod digest;
mod discover;
mod error;
mod failure;
mod finding;
mod gate_scope;
#[cfg(kani)]
mod kani;
mod kani_inventory;
mod lane;
mod mutants_baseline;
mod mutants_outcomes;
mod outcome;
mod proof_id;
mod receipt;
mod report;
mod rule_id;
mod target_project;
mod text_range;
mod v1_receipt;
mod workspace_path;

pub use artifact::{ArtifactOutcome, ArtifactVariant, LaneArtifact};
pub use diagnostic::{DiagnosticSeverity, InputDiagnostic, PolicyDiagnostic};
pub use digest::Digest;
pub use discover::{
    ManifestKind, ManifestObservation, ManifestStatus, TargetObservation, classify_manifest,
    select_target_observation, select_target_root,
};
pub use error::{
    ArtifactError, CoreError, DigestError, FailureError, FindingError, GateScopeError,
    KaniHarnessIdError, KaniInventoryError, LaneError, LocationError, MutantIdError,
    MutantOperatorError, MutantsBaselineError, MutantsOutcomesError, OutcomeError,
    PathSegmentError, PerLaneScopeError, ReceiptError, ReportError, RuleIdError,
    TargetProjectError, TextRangeError, WorkspacePathError,
};
pub use failure::{LaneFailure, ProcessTermination};
pub use finding::{
    CatalogRow, Finding, FindingEffect, Location, RepairHint, RepairHintClass, catalog_rows,
};
pub use gate_scope::GateScope;
pub use kani_inventory::{
    KANI_INVENTORY_MAX_HARNESSES, KaniHarnessListing, KaniInventory, canonical_harness_id,
};
pub use lane::Lane;
pub use mutants_baseline::{
    ACCEPTED_BY_RULE_FAMILY, MUTANTS_BASELINE_SCHEMA_VERSION, MutantBaselineEntry, MutantsBaseline,
};
pub use mutants_outcomes::{
    MUTANTS_OUTCOMES_MAX_ENTRIES, MUTANTS_RECORDS_MAX_ENTRIES, MutantOutcomeEntry, MutantRecord,
    MutantScenarioData, MutantsOutcomes, MutantsRecords, OutcomeScenario, OutcomeSummary,
    RawFunction, RawPoint, RawSpan, relative_mutant_path,
};
pub use outcome::{CommandEvidence, LaneEvidence, LaneOutcome, SkipReason};
pub use proof_id::{
    KANI_HARNESS_ID_MAX_LEN, KaniHarnessId, MUTANT_PATH_MAX_LEN, MUTANT_PKG_MAX_LEN, MutantId,
    MutantOperator, ToolKind,
};
pub use receipt::{
    LaneDigest, LaneName, RECEIPT_ENVELOPE_SCHEMA_VERSION, ReceiptDigests, ReceiptEnvelope,
    ReceiptLaneExit, ReceiptPeriod, RecordedTargetRoot,
};
pub use report::{PerLaneEntry, RejectKind, Report, ReportKind};
pub use rule_id::RuleId;
pub use target_project::TargetProject;
pub use text_range::TextRange;
/// Re-export as `QualityReceipt` for compatibility with the v1-spec naming.
pub use v1_receipt::QualityReceiptV1 as QualityReceipt;
pub use v1_receipt::{LaneReceipt, QualityReceiptV1};
pub use workspace_path::WorkspacePath;
