//! Report: the single disjoint root type for every Xtask gate invocation.

use serde::{Deserialize, Serialize};

use crate::{
    finding::Finding,
    lane::{LaneFailure, LaneOutcome},
    receipt::QualityReceipt,
};

/// The result of a single `xtask gate` invocation.
///
/// `Reject` can carry BOTH code findings AND gate failures simultaneously.
/// Use `reject_kind()` to determine the mix.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum Report {
    /// All scoped lanes passed. `QualityReceipt` emitted.
    #[serde(rename = "pass")]
    Pass {
        receipt: QualityReceipt,
        per_lane: Box<[LaneOutcome]>,
    },
    /// One or more lanes found violations or failed.
    #[serde(rename = "reject")]
    Reject {
        code_findings: Box<[Finding]>,
        gate_failures: Box<[LaneFailure]>,
        per_lane: Box<[LaneOutcome]>,
    },
    /// Policy files are malformed. Edit policy, not code.
    #[serde(rename = "policy_error")]
    PolicyError {
        diagnostics: Box<[PolicyDiagnostic]>,
    },
    /// Input contract violated (not a crate, unreadable, etc.).
    #[serde(rename = "input_error")]
    InputError { diagnostics: Box<[InputDiagnostic]> },
}

impl Report {
    /// Returns the reject kind if this is a Reject, otherwise None.
    #[must_use]
    pub fn reject_kind(&self) -> Option<RejectKind> {
        match self {
            Self::Reject {
                code_findings,
                gate_failures,
                ..
            } => {
                let has_code = !code_findings.is_empty();
                let has_gate = !gate_failures.is_empty();
                match (has_code, has_gate) {
                    (true, true) => Some(RejectKind::Mixed),
                    (true | false, false) => Some(RejectKind::CodeOnly),
                    (false, true) => Some(RejectKind::GateOnly),
                }
            }
            _ => None,
        }
    }

    /// Returns true if this report indicates success.
    #[must_use]
    pub const fn is_pass(&self) -> bool {
        matches!(self, Self::Pass { .. })
    }
}

/// What kind of rejection occurred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectKind {
    /// Only code findings (AI should edit code).
    CodeOnly,
    /// Only gate/tool failures (infra issue).
    GateOnly,
    /// Both code findings and gate failures.
    Mixed,
}

/// A policy file diagnostic.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PolicyDiagnostic {
    pub file: String,
    pub message: String,
}

/// An input contract diagnostic.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InputDiagnostic {
    pub message: String,
}
