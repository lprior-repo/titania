//! The aggregated report from a titania-check run.
//!
//! A `Report` is the final output: either a pass (with a receipt), a reject
//! (with findings and failures), or an error (policy or input diagnostics).

use serde::{Deserialize, Serialize};

use crate::{
    diagnostic::{InputDiagnostic, PolicyDiagnostic},
    error::ReportError,
    failure::LaneFailure,
    finding::Finding,
    lane::Lane,
    outcome::LaneOutcome,
    v1_receipt::QualityReceiptV1 as QualityReceipt,
};

/// A single per-lane entry: lane name plus its outcome.
///
/// Serialized as `{"lane": "Fmt", "outcome": {"variant": "clean", ...}}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PerLaneEntry {
    /// Lane identifier (e.g. `Fmt`, `Clippy`, `Check`).
    pub lane: Lane,
    /// Outcome of the lane run (clean, failed, or errored).
    pub outcome: LaneOutcome,
}
/// Classification of a [`Report::Reject`] by which collections are populated.
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum Report {
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

#[derive(Deserialize)]
#[serde(tag = "variant", rename_all = "snake_case", deny_unknown_fields)]
enum ReportWire {
    Pass {
        receipt: QualityReceipt,
        per_lane: Box<[PerLaneEntry]>,
    },
    Reject {
        #[serde(rename = "code_findings")]
        code: Box<[Finding]>,
        #[serde(rename = "gate_failures")]
        gate: Box<[LaneFailure]>,
        #[serde(rename = "per_lane")]
        lane: Box<[PerLaneEntry]>,
    },
    PolicyError {
        diagnostics: Box<[PolicyDiagnostic]>,
    },
    InputError {
        diagnostics: Box<[InputDiagnostic]>,
    },
}

impl ReportWire {
    /// Converts a wire report into a constructor-validated domain report.
    ///
    /// # Errors
    ///
    /// Returns `E` when the serialized wire shape violates report invariants.
    fn into_report<E: serde::de::Error>(self) -> Result<Report, E> {
        match self {
            Self::Pass { receipt, per_lane } => Report::pass(receipt, per_lane).map_err(E::custom),
            Self::Reject { code, gate, lane } => reject::<E>(code, gate, lane),
            Self::PolicyError { diagnostics } => Ok(Report::PolicyError { diagnostics }),
            Self::InputError { diagnostics } => Ok(Report::InputError { diagnostics }),
        }
    }
}

impl<'de> Deserialize<'de> for Report {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        ReportWire::deserialize(de)?.into_report()
    }
}

/// Build a reject report from deserialized wire fields.
///
/// # Errors
/// Returns `E` when the reject invariant is invalid.
fn reject<E: serde::de::Error>(
    code_findings: Box<[Finding]>,
    gate_failures: Box<[LaneFailure]>,
    per_lane: Box<[PerLaneEntry]>,
) -> Result<Report, E> {
    Report::reject(code_findings, gate_failures, per_lane).map_err(E::custom)
}

/// Check a reject report contains at least one finding or failure.
///
/// # Errors
/// Returns [`ReportError::EmptyReject`] when both reject collections are empty.
fn check_reject_not_empty(
    code_findings: &[Finding],
    gate_failures: &[LaneFailure],
) -> Result<(), ReportError> {
    (!code_findings.is_empty() || !gate_failures.is_empty())
        .then_some(())
        .ok_or(ReportError::EmptyReject)
}

/// Check a pass report carries per-lane evidence.
///
/// # Errors
/// Returns [`ReportError::EmptyPerLane`] when `per_lane` is empty.
fn check_per_lane_not_empty(per_lane: &[PerLaneEntry]) -> Result<(), ReportError> {
    (!per_lane.is_empty()).then_some(()).ok_or(ReportError::EmptyPerLane)
}
/// Check every lane outcome in `per_lane` is pass-shaped (Clean, Skipped,
/// or Findings with only informational findings).
///
/// # Errors
///
/// Returns [`ReportError::NonPassLaneOutcome`] for the first lane outcome
/// that is not pass-shaped.
fn validate_per_lane_pass(per_lane: &[PerLaneEntry]) -> Result<(), ReportError> {
    per_lane.iter().find(|e| !e.outcome.is_pass()).map_or(Ok(()), |first_bad| {
        Err(ReportError::NonPassLaneOutcome(first_bad.lane, format!("{:?}", first_bad.outcome)))
    })
}

#[must_use]
const fn reject_kind_from_empty(code_empty: bool, gate_empty: bool) -> Option<RejectKind> {
    match (code_empty, gate_empty) {
        (false, true) => Some(RejectKind::CodeOnly),
        (true, false) => Some(RejectKind::GateOnly),
        (false, false) => Some(RejectKind::Mixed),
        (true, true) => None,
    }
}

#[must_use]
const fn reject_kind_for(
    code_findings: &[Finding],
    gate_failures: &[LaneFailure],
) -> Option<RejectKind> {
    reject_kind_from_empty(code_findings.is_empty(), gate_failures.is_empty())
}

impl Report {
    /// Create a `Report::Reject`, validating the invariant.
    ///
    /// # Errors
    /// - [`ReportError::EmptyReject`] if both `code_findings` and
    ///   `gate_failures` are empty.
    pub fn reject(
        code_findings: Box<[Finding]>,
        gate_failures: Box<[LaneFailure]>,
        per_lane: Box<[PerLaneEntry]>,
    ) -> Result<Self, ReportError> {
        check_reject_not_empty(&code_findings, &gate_failures)?;
        Ok(Self::Reject { code_findings, gate_failures, per_lane })
    }
    /// Create a `Report::Pass`.
    ///
    /// # Errors
    /// - [`ReportError::EmptyPerLane`] if `per_lane` is empty.
    /// - [`ReportError::NonPassLaneOutcome`] if any lane outcome is not
    ///   pass-shaped (i.e., not `Clean`, `Skipped`, or informational-only
    ///   `Findings`).
    pub fn pass(
        receipt: QualityReceipt,
        per_lane: Box<[PerLaneEntry]>,
    ) -> Result<Self, ReportError> {
        check_per_lane_not_empty(&per_lane)?;
        validate_per_lane_pass(&per_lane)?;
        Ok(Self::Pass { receipt, per_lane })
    }

    /// Classify the reject kind, if this report is a reject.
    ///
    /// Returns `None` for non-reject reports or if both collections are
    /// empty (invariant violation).
    #[must_use]
    pub fn reject_kind(&self) -> Option<RejectKind> {
        match self {
            Self::Reject { code_findings: c, gate_failures: g, .. } => reject_kind_for(c, g),
            _ => None,
        }
    }

    /// Whether this report represents a pass.
    #[must_use]
    pub const fn is_pass(&self) -> bool {
        matches!(self, Self::Pass { .. })
    }

    /// Whether this report represents a reject.
    #[must_use]
    pub const fn is_reject(&self) -> bool {
        matches!(self, Self::Reject { .. })
    }

    /// Whether this report is a policy error.
    #[must_use]
    pub const fn is_policy_error(&self) -> bool {
        matches!(self, Self::PolicyError { .. })
    }

    /// Whether this report is an input error.
    #[must_use]
    pub const fn is_input_error(&self) -> bool {
        matches!(self, Self::InputError { .. })
    }

    /// If this is a reject, return the code findings.
    #[must_use]
    pub fn code_findings(&self) -> Option<&[Finding]> {
        match self {
            Self::Reject { code_findings, .. } => Some(code_findings),
            _ => None,
        }
    }

    /// If this is a reject, return the gate failures.
    #[must_use]
    pub fn gate_failures(&self) -> Option<&[LaneFailure]> {
        match self {
            Self::Reject { gate_failures, .. } => Some(gate_failures),
            _ => None,
        }
    }
}
