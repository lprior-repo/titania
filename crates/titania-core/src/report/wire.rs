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
    Report::reject(code_findings, gate_failures, per_lane).map_err(E::custom)
}
