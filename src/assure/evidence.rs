#![forbid(unsafe_code)]

use crate::assure::model::{
    AssumptionId, BeadId, ClaimCeilingId, ContractClauseId, Digest, EvidenceId, ObligationId, TcbId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub id: EvidenceId,
    pub obligation: ObligationId,
    pub command: String,
    pub runner: String,
    pub runner_identity: String,
    pub spec_digest: Digest,
    pub ir_digest: Digest,
    pub oracle_bank_digest: Digest,
    pub source_digest: Digest,
    pub generator_digest: Digest,
    pub tool_versions: BTreeMap<String, String>,
    pub cwd_digest: Digest,
    pub env_digest: Digest,
    pub exit_code: i32,
    pub stdout_digest: Digest,
    pub stderr_digest: Digest,
    pub signature: EvidenceSignature,
    pub status: EvidenceStatus,
    pub claim_ceiling_id: ClaimCeilingId,
}

impl EvidenceRecord {
    #[must_use]
    pub fn accepted_for_landing(&self) -> bool {
        self.exit_code == 0 && self.status.accepted_for_landing() && self.signature.verified
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSignature {
    pub kind: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    LocalUntrusted,
    CiUnsigned,
    CiSigned,
    VerifiedAttestation,
}

impl EvidenceStatus {
    #[must_use]
    pub const fn accepted_for_landing(self) -> bool {
        matches!(self, Self::CiSigned | Self::VerifiedAttestation)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimCeiling {
    pub id: ClaimCeilingId,
    pub proves: Vec<String>,
    pub not_proven: Vec<String>,
    pub blocks_claims: Vec<ContractClauseId>,
}

impl ClaimCeiling {
    #[must_use]
    pub fn blocks_claim(&self, claim: &ContractClauseId) -> bool {
        self.blocks_claims.iter().any(|blocked| blocked == claim)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TcbLedgerEntry {
    pub id: TcbId,
    pub component: String,
    pub reason: String,
    pub owner: String,
    pub introduced_in: String,
    pub expires_review: String,
    pub required_tests: Vec<String>,
}

impl TcbLedgerEntry {
    #[must_use]
    pub fn has_landing_metadata(&self) -> bool {
        !self.owner.is_empty()
            && !self.reason.is_empty()
            && !self.expires_review.is_empty()
            && !self.required_tests.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssumptionLedgerEntry {
    pub id: AssumptionId,
    pub claim: String,
    pub required_for: Vec<ContractClauseId>,
    pub evidence_dependency: Option<EvidenceDependency>,
    pub owner: String,
    pub expires: String,
    pub blocks_landing_if_unclosed: bool,
}

impl AssumptionLedgerEntry {
    #[must_use]
    pub fn blocks_claim(&self, claim: &ContractClauseId) -> bool {
        self.blocks_landing_if_unclosed && self.required_for.iter().any(|entry| entry == claim)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceDependency {
    pub bead: BeadId,
    pub required: bool,
}
