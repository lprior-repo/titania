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

fn reject<E: serde::de::Error>(
    code_findings: Box<[Finding]>,
    gate_failures: Box<[LaneFailure]>,
    per_lane: Box<[PerLaneEntry]>,
) -> Result<Report, E> {
    let scope = infer_reject_scope(&per_lane);
    Report::reject(scope, code_findings, gate_failures, per_lane).map_err(E::custom)
}

fn infer_reject_scope(per_lane: &[PerLaneEntry]) -> crate::GateScope {
    use crate::Lane;
    if per_lane.iter().any(|e| matches!(e.lane(), Lane::Kani | Lane::Mutants)) {
        crate::GateScope::Full
    } else {
        crate::GateScope::Release
    }
}
