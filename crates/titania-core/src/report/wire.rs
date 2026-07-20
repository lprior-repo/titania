//! Wire (de)serialization for [`Report`].
//!
//! The public [`Report`] type enforces invariants in its smart-constructor
//! methods, so the wire path uses a private mirror that defers to those
//! constructors.

use serde::Deserialize;

use super::{PerLaneEntry, QualityReceipt, Report};
use crate::{
    diagnostic::{InputDiagnostic, PolicyDiagnostic},
    failure::LaneFailure,
    finding::Finding,
};

#[derive(Deserialize)]
#[serde(tag = "variant", deny_unknown_fields)]
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
    /// Convert a wire report into a constructor-validated domain report.
    ///
    /// # Errors
    ///
    /// Returns `E` when the serialized wire shape violates report invariants.
    fn into_report<E: serde::de::Error>(self) -> Result<Report, E> {
        match self {
            Self::Pass { receipt, per_lane } => Report::pass(receipt, per_lane).map_err(E::custom),
            Self::Reject { code, gate, lane } => reject::<E>(code, gate, lane),
            Self::PolicyError { diagnostics } => Ok(Report::policy_error(diagnostics)),
            Self::InputError { diagnostics } => Ok(Report::input_error(diagnostics)),
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
    let scope = infer_reject_scope(&per_lane);
    Report::reject(scope, code_findings, gate_failures, per_lane).map_err(E::custom)
}

/// Infer the gate scope for a reject deserialization by checking whether
/// `per_lane` references Full-only lanes (`Kani`, `Mutants`). Falls back
/// to `Release` when neither is present.
fn infer_reject_scope(per_lane: &[PerLaneEntry]) -> crate::GateScope {
    use crate::Lane;
    if per_lane.iter().any(|e| matches!(e.lane(), Lane::Kani | Lane::Mutants)) {
        crate::GateScope::Full
    } else {
        crate::GateScope::Release
    }
}
