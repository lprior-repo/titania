//! On-disk serialization shape for a single lane's outcome artifact.
//!
//! A lane artifact is the JSON record written to
//! `.titania/out/<scope>/<lane>.json` by the lane runner and read back by the
//! aggregator. [`LaneArtifact`] is the canonical envelope and
//! [`ArtifactOutcome`] is the canonical outcome projection. Defining both next
//! to [`crate::LaneOutcome`] in the domain core guarantees the writer and the
//! reader share one source of truth for the on-disk format, so every
//! [`LaneOutcome`] round-trips through the aggregator without parse errors.
//!
//! The [`crate::LaneOutcome::Failed`] variant is a struct variant with a
//! `failure` field, which produces an on-disk shape of
//! `{"variant": "failed", "failure": { ... }}` that the shared
//! [`ArtifactOutcome`] projection can both write and read.

use serde::{Deserialize, Serialize};

use crate::{
    error::ArtifactError,
    failure::LaneFailure,
    finding::Finding,
    lane::Lane,
    outcome::{LaneEvidence, LaneOutcome, SkipReason},
};

/// On-disk discriminator naming which outcome payload field is populated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactVariant {
    /// Lane completed cleanly and carries command evidence.
    Clean,
    /// Lane emitted one or more findings.
    Findings,
    /// Lane failed before producing a clean or findings verdict.
    Failed,
    /// Lane was intentionally skipped for a recorded reason.
    Skipped,
}

/// On-disk projection of a [`LaneOutcome`].
///
/// Exactly one optional payload field is populated, matching
/// [`ArtifactVariant`]. Both the lane artifact writer and the aggregator
/// reader construct and parse this type, so the on-disk shape cannot drift
/// between them and a lane outcome always round-trips.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactOutcome {
    /// Discriminator naming which optional payload field is populated.
    variant: ArtifactVariant,
    /// Present when [`ArtifactVariant::Clean`].
    #[serde(skip_serializing_if = "Option::is_none")]
    evidence: Option<LaneEvidence>,
    /// Present when [`ArtifactVariant::Findings`].
    #[serde(skip_serializing_if = "Option::is_none")]
    findings: Option<Box<[Finding]>>,
    /// Present when [`ArtifactVariant::Failed`].
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<LaneFailure>,
    /// Present when [`ArtifactVariant::Skipped`].
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped: Option<SkipReason>,
}

impl ArtifactOutcome {
    /// Reconstruct the domain [`LaneOutcome`] from this on-disk projection.
    ///
    /// # Errors
    ///
    /// Returns [`ArtifactError::FieldMissing`] when the discriminator names a
    /// payload field that is absent, i.e. a corrupt or hand-edited artifact.
    pub fn into_lane_outcome(self) -> Result<LaneOutcome, ArtifactError> {
        match self.variant {
            ArtifactVariant::Clean => self
                .evidence
                .map(|evidence| LaneOutcome::Clean { evidence })
                .ok_or(ArtifactError::FieldMissing { variant: "clean", field: "evidence" }),
            ArtifactVariant::Findings => self
                .findings
                .map(|findings| LaneOutcome::Findings { findings })
                .ok_or(ArtifactError::FieldMissing { variant: "findings", field: "findings" }),
            ArtifactVariant::Failed => self
                .failure
                .map(|failure| LaneOutcome::Failed { failure })
                .ok_or(ArtifactError::FieldMissing { variant: "failed", field: "failure" }),
            ArtifactVariant::Skipped => self
                .skipped
                .map(|reason| LaneOutcome::Skipped { reason })
                .ok_or(ArtifactError::FieldMissing { variant: "skipped", field: "skipped" }),
        }
    }
}

impl From<&LaneOutcome> for ArtifactOutcome {
    fn from(outcome: &LaneOutcome) -> Self {
        match outcome {
            LaneOutcome::Clean { evidence } => Self {
                variant: ArtifactVariant::Clean,
                evidence: Some(evidence.clone()),
                findings: None,
                failure: None,
                skipped: None,
            },
            LaneOutcome::Findings { findings } => Self {
                variant: ArtifactVariant::Findings,
                evidence: None,
                findings: Some(findings.clone()),
                failure: None,
                skipped: None,
            },
            LaneOutcome::Failed { failure } => Self {
                variant: ArtifactVariant::Failed,
                evidence: None,
                findings: None,
                failure: Some(failure.clone()),
                skipped: None,
            },
            LaneOutcome::Skipped { reason } => Self {
                variant: ArtifactVariant::Skipped,
                evidence: None,
                findings: None,
                failure: None,
                skipped: Some(*reason),
            },
        }
    }
}

/// On-disk envelope for one lane artifact file.
///
/// Serialized as `{"lane": "Fmt", "outcome": { ... }}`. The writer emits this
/// shape and the reader parses it back, sharing the canonical
/// [`ArtifactOutcome`] payload so the format is symmetric.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneArtifact {
    /// Lane this artifact records.
    lane: Lane,
    /// Projected lane outcome payload.
    outcome: ArtifactOutcome,
}

impl LaneArtifact {
    /// Construct a new lane artifact envelope from its parts.
    #[must_use]
    pub const fn new(lane: Lane, outcome: ArtifactOutcome) -> Self {
        Self { lane, outcome }
    }

    /// Lane this artifact records.
    #[must_use]
    pub const fn lane(&self) -> Lane {
        self.lane
    }

    /// Consume the envelope and return its projected outcome payload.
    #[must_use]
    pub fn into_outcome(self) -> ArtifactOutcome {
        self.outcome
    }
}
