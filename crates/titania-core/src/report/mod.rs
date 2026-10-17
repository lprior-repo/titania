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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PerLaneEntry(PerLaneEntryInner);

impl PerLaneEntry {
    #[must_use]
    pub const fn new(lane: Lane, outcome: LaneOutcome) -> Self {
        Self(PerLaneEntryInner { lane, outcome })
    }

    #[must_use]
    pub const fn lane(&self) -> &Lane {
        &self.0.lane
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectKind {
    CodeOnly,
    GateOnly,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Report(ReportInner);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "variant")]
enum ReportInner {
    Pass {
        receipt: QualityReceipt,
        per_lane: Box<[PerLaneEntry]>,
    },
    Reject {
        code_findings: Box<[Finding]>,
        gate_failures: Box<[LaneFailure]>,
        per_lane: Box<[PerLaneEntry]>,
    },
    PolicyError {
        diagnostics: Box<[PolicyDiagnostic]>,
    },
    InputError {
        diagnostics: Box<[InputDiagnostic]>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    Pass,
    Reject,
    PolicyError,
    InputError,
}
