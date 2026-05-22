#![forbid(unsafe_code)]

use crate::assure::model::{
    BeadId, ContractClauseId, Digest, ExpectedOutcome, OracleId, PathId, VarId,
};
use crate::assure::path::Valuation;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleRecord {
    pub id: OracleId,
    pub bead: BeadId,
    pub maps_to: Vec<ContractClauseId>,
    pub path: PathId,
    pub input: Valuation,
    pub expected: ExpectedOutcome,
    pub provenance: OracleProvenance,
    pub generated_by_assurec: bool,
    pub artifact_path: String,
    pub included_in_oracle_bank: bool,
    pub consumed_by_oracle_replay: bool,
    pub digest: Digest,
}

impl OracleRecord {
    #[must_use]
    pub fn is_independent_for_landing(&self) -> bool {
        !self.generated_by_assurec
            && !is_generated_artifact_path(&self.artifact_path)
            && !self.maps_to.is_empty()
            && self.included_in_oracle_bank
            && self.consumed_by_oracle_replay
            && self.provenance.is_trusted_for_landing()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleProvenance {
    VcsPreexisting {
        commit: String,
        present_in_merge_base: bool,
        signer: Option<String>,
        signature_verified: bool,
    },
    ApprovedReview {
        pr: String,
        reviewer: String,
        approval_digest: Digest,
        signature_verified: bool,
    },
    HistoricalBug {
        issue: String,
        regression_id: String,
    },
    Generated {
        generator_digest: Digest,
    },
}

impl OracleProvenance {
    #[must_use]
    pub fn is_trusted_for_landing(&self) -> bool {
        match self {
            Self::VcsPreexisting {
                present_in_merge_base,
                signature_verified,
                ..
            } => *present_in_merge_base || *signature_verified,
            Self::ApprovedReview {
                signature_verified, ..
            } => *signature_verified,
            Self::HistoricalBug {
                issue,
                regression_id,
            } => !issue.is_empty() && !regression_id.is_empty(),
            Self::Generated { .. } => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleCheckReport {
    pub total: usize,
    pub trusted: usize,
    pub failures: Vec<OracleCheckFailure>,
}

impl OracleCheckReport {
    #[must_use]
    pub fn is_pass(&self) -> bool {
        self.failures.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleCheckFailure {
    GeneratedOracle { oracle: String },
    GeneratedArtifactPath { oracle: String, path: String },
    MissingTraceability { oracle: String },
    MissingOracleBankDigestMembership { oracle: String },
    NotConsumedByReplay { oracle: String },
    UntrustedProvenance { oracle: String },
}

pub fn check_oracles(records: &[OracleRecord]) -> OracleCheckReport {
    let failures = records.iter().flat_map(oracle_failures).collect::<Vec<_>>();
    let trusted = records
        .iter()
        .filter(|record| record.is_independent_for_landing())
        .count();
    OracleCheckReport {
        total: records.len(),
        trusted,
        failures,
    }
}

fn oracle_failures(record: &OracleRecord) -> Vec<OracleCheckFailure> {
    [
        generated_oracle_failure(record),
        generated_artifact_path_failure(record),
        missing_traceability_failure(record),
        missing_bank_failure(record),
        missing_replay_failure(record),
        untrusted_provenance_failure(record),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn generated_oracle_failure(record: &OracleRecord) -> Option<OracleCheckFailure> {
    record
        .generated_by_assurec
        .then(|| OracleCheckFailure::GeneratedOracle {
            oracle: record.id.as_str().to_string(),
        })
}

fn generated_artifact_path_failure(record: &OracleRecord) -> Option<OracleCheckFailure> {
    is_generated_artifact_path(&record.artifact_path).then(|| {
        OracleCheckFailure::GeneratedArtifactPath {
            oracle: record.id.as_str().to_string(),
            path: record.artifact_path.clone(),
        }
    })
}

fn missing_traceability_failure(record: &OracleRecord) -> Option<OracleCheckFailure> {
    record
        .maps_to
        .is_empty()
        .then(|| OracleCheckFailure::MissingTraceability {
            oracle: record.id.as_str().to_string(),
        })
}

fn missing_bank_failure(record: &OracleRecord) -> Option<OracleCheckFailure> {
    (!record.included_in_oracle_bank).then(|| {
        OracleCheckFailure::MissingOracleBankDigestMembership {
            oracle: record.id.as_str().to_string(),
        }
    })
}

fn missing_replay_failure(record: &OracleRecord) -> Option<OracleCheckFailure> {
    (!record.consumed_by_oracle_replay).then(|| OracleCheckFailure::NotConsumedByReplay {
        oracle: record.id.as_str().to_string(),
    })
}

fn untrusted_provenance_failure(record: &OracleRecord) -> Option<OracleCheckFailure> {
    (!record.provenance.is_trusted_for_landing()).then(|| OracleCheckFailure::UntrustedProvenance {
        oracle: record.id.as_str().to_string(),
    })
}

fn is_generated_artifact_path(path: &str) -> bool {
    path.starts_with("assurance/generated/")
        || path.starts_with("src/generated/")
        || path.starts_with("tests/generated/")
        || path.starts_with("proofs/kani/generated/")
        || path.contains("/generated/")
}

#[must_use]
pub fn valuation_with_pair(var: VarId, value: crate::assure::model::EnumVariantId) -> Valuation {
    [(var, value)].into_iter().collect()
}
