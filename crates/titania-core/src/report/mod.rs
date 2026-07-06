//! The aggregated report from a titania-check run.
//!
//! A `Report` is the final output: either a pass (with a receipt), a reject
//! (with findings and failures), or an error (policy or input diagnostics).
//!
//! File layout:
//! - `mod.rs` — public types (`PerLaneEntry`, `Report`, `ReportKind`,
//!   `RejectKind`), private `ReportInner`, and free-function validators
//!   shared by both constructors and wire deserialization.
//! - `wire.rs` — `serde::Deserialize` path for `Report` via the private
//!   `ReportWire` mirror.
//! - `accessors.rs` — `impl Report` accessor methods (`is_pass`,
//!   `code_findings`, `kind`, …).

use serde::{Deserialize, Serialize};

use crate::{
    diagnostic::{InputDiagnostic, PolicyDiagnostic},
    failure::LaneFailure,
    finding::Finding,
    lane::Lane,
    outcome::LaneOutcome,
    v1_receipt::QualityReceiptV1 as QualityReceipt,
};

mod accessors;
mod validators;
mod wire;

/// A single per-lane entry: lane name plus its outcome.
///
/// Serialized as `{"lane": "Fmt", "outcome": {"variant": "clean", ...}}`.
///
/// Constructed via [`PerLaneEntry::new`] — direct field access is forbidden
/// to prevent illegal state construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PerLaneEntry(PerLaneEntryInner);

impl PerLaneEntry {
    /// Construct a new per-lane entry.
    #[must_use]
    pub const fn new(lane: Lane, outcome: LaneOutcome) -> Self {
        Self(PerLaneEntryInner { lane, outcome })
    }

    /// Lane identifier.
    #[must_use]
    pub const fn lane(&self) -> &Lane {
        &self.0.lane
    }

    /// Lane outcome.
    #[must_use]
    pub const fn outcome(&self) -> &LaneOutcome {
        &self.0.outcome
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
struct PerLaneEntryInner {
    lane: Lane,
    outcome: LaneOutcome,
}

impl<'de> Deserialize<'de> for PerLaneEntry {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        PerLaneEntryInner::deserialize(de).map(Self)
    }
}

/// Classification of a [`Report::reject`] by which collections are populated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectKind {
    /// Only `code_findings` is non-empty; `gate_failures` is empty.
    CodeOnly,
    /// Only `gate_failures` is non-empty; `code_findings` is empty.
    GateOnly,
    /// Both collections are non-empty.
    Mixed,
}

/// Aggregated report from a titania-check run.
///
/// A `Report` is the final output: either a pass (with a receipt), a reject
/// (with findings and failures), or an error (policy or input diagnostics).
///
/// Constructed via [`Report::pass`], [`Report::reject`], [`Report::policy_error`],
/// or [`Report::input_error`] — direct construction of inner variants is
/// forbidden to prevent illegal state construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Report(ReportInner);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
enum ReportInner {
    /// All lanes passed.
    Pass {
        /// Receipt summarizing the successful run.
        receipt: QualityReceipt,
        /// Per-lane outcomes that justify the pass.
        per_lane: Box<[PerLaneEntry]>,
    },
    /// One or more lanes rejected or failed.
    ///
    /// INVARIANT: at least one of `code_findings` or `gate_failures` is
    /// non-empty. A `Reject` with both empty is a bug — should be `Pass`.
    Reject {
        /// Findings that caused one or more code lanes to reject.
        code_findings: Box<[Finding]>,
        /// Gate failures that prevented a clean verdict.
        gate_failures: Box<[LaneFailure]>,
        /// Per-lane outcomes observed during the rejected run.
        per_lane: Box<[PerLaneEntry]>,
    },
    /// Policy configuration error.
    PolicyError {
        /// Policy diagnostics explaining why configuration loading failed.
        diagnostics: Box<[PolicyDiagnostic]>,
    },
    /// Input or argument error.
    InputError {
        /// Input diagnostics explaining why invocation validation failed.
        diagnostics: Box<[InputDiagnostic]>,
    },
}

/// Discriminator for the four report states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    /// All lanes passed.
    Pass,
    /// One or more lanes rejected or failed.
    Reject,
    /// Policy configuration error.
    PolicyError,
    /// Input or argument error.
    InputError,
}
