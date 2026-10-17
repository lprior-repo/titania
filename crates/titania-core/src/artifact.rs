use serde::{Deserialize, Serialize};

use crate::{
    error::ArtifactError,
    failure::LaneFailure,
    finding::Finding,
    lane::Lane,
    outcome::{LaneEvidence, LaneOutcome, SkipReason},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactVariant {
    Clean,
    Findings,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactOutcome {
    variant: ArtifactVariant,
    evidence: Option<LaneEvidence>,
    findings: Option<Box<[Finding]>>,
    failure: Option<LaneFailure>,
    skipped: Option<SkipReason>,
}

impl Serialize for ArtifactOutcome {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let outcome = self.clone().into_lane_outcome().map_err(serde::ser::Error::custom)?;
        outcome.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ArtifactOutcome {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let outcome = LaneOutcome::deserialize(deserializer)?;
        Ok(Self::from(&outcome))
    }
}

impl ArtifactOutcome {
    pub fn into_lane_outcome(self) -> Result<LaneOutcome, ArtifactError> {
        match self.variant {
            ArtifactVariant::Clean => self
                .evidence
                .map(|evidence| LaneOutcome::Clean { evidence })
                .ok_or(ArtifactError::FieldMissing { variant: "Clean", field: "evidence" }),
            ArtifactVariant::Findings => self
                .findings
                .map(|findings| LaneOutcome::Findings { findings })
                .ok_or(ArtifactError::FieldMissing { variant: "Findings", field: "findings" }),
            ArtifactVariant::Failed => self
                .failure
                .map(|failure| LaneOutcome::Failed { failure })
                .ok_or(ArtifactError::FieldMissing { variant: "Failed", field: "failure" }),
            ArtifactVariant::Skipped => self
                .skipped
                .map(|reason| LaneOutcome::Skipped { reason })
                .ok_or(ArtifactError::FieldMissing { variant: "Skipped", field: "skipped" }),
        }
    }
}

impl From<&LaneOutcome> for ArtifactOutcome {
    fn from(outcome: &LaneOutcome) -> Self {
        match outcome {
            LaneOutcome::Clean { evidence } => Self::clean(evidence.clone()),
            LaneOutcome::Findings { findings } => Self::findings(findings.clone()),
            LaneOutcome::Failed { failure } => Self::failed(failure.clone()),
            LaneOutcome::Skipped { reason } => Self::skipped(*reason),
        }
    }
}

impl ArtifactOutcome {
    #[must_use]
    pub(crate) const fn clean(evidence: LaneEvidence) -> Self {
        Self::new(ArtifactVariant::Clean, Some(evidence), None, None, None)
    }

    #[must_use]
    pub(crate) const fn findings(findings: Box<[Finding]>) -> Self {
        Self::new(ArtifactVariant::Findings, None, Some(findings), None, None)
    }

    #[must_use]
    pub(crate) const fn failed(failure: LaneFailure) -> Self {
        Self::new(ArtifactVariant::Failed, None, None, Some(failure), None)
    }

    #[must_use]
    pub(crate) const fn skipped(reason: SkipReason) -> Self {
        Self::new(ArtifactVariant::Skipped, None, None, None, Some(reason))
    }

    const fn new(
        variant: ArtifactVariant,
        evidence: Option<LaneEvidence>,
        findings: Option<Box<[Finding]>>,
        failure: Option<LaneFailure>,
        skipped: Option<SkipReason>,
    ) -> Self {
        Self { variant, evidence, findings, failure, skipped }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneArtifact {
    lane: Lane,
    outcome: ArtifactOutcome,
}

impl LaneArtifact {
    #[must_use]
    pub const fn new(lane: Lane, outcome: ArtifactOutcome) -> Self {
        Self { lane, outcome }
    }

    #[must_use]
    pub const fn lane(&self) -> Lane {
        self.lane
    }

    #[must_use]
    pub fn into_outcome(self) -> ArtifactOutcome {
        self.outcome
    }
}
