//! Outcome of a single lane execution.
//!
//! Each lane produces exactly one [`LaneOutcome`]: clean, findings, failure,
//! or skipped.

use serde::{Deserialize, Serialize};

use crate::{
    digest::Digest,
    error::OutcomeError,
    failure::{LaneFailure, ProcessTermination},
    finding::Finding,
};

/// Why a lane was skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    /// A prior compilation failure caused dependent lanes to be skipped.
    PriorCompilationFailure,
    /// The scope did not include this lane.
    NotSelectedByScope,
    /// No applicable files were found for this lane.
    NotApplicable,
    /// The lane is disabled in the policy configuration.
    PolicyDisabled,
}
impl core::str::FromStr for SkipReason {
    type Err = OutcomeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "prior_compilation_failure" | "PriorCompilationFailure" => {
                Ok(Self::PriorCompilationFailure)
            }
            "not_selected_by_scope" | "NotSelectedByScope" => Ok(Self::NotSelectedByScope),
            "not_applicable" | "NotApplicable" => Ok(Self::NotApplicable),
            "policy_disabled" | "PolicyDisabled" => Ok(Self::PolicyDisabled),
            _ => Err(OutcomeError::UnknownSkipReason(value.to_string())),
        }
    }
}

/// Evidence attached to a clean lane outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LaneEvidence {
    command: CommandEvidence,
    tool_version: String,
    exit_status: ProcessTermination,
    parsed_result_digest: Digest,
}

impl LaneEvidence {
    /// Construct lane evidence.
    ///
    /// # Errors
    /// - [`OutcomeError::NonZeroExit`] if `exit_status` is not
    ///   `ProcessTermination::Exited { code: 0 }`.
    pub fn new(
        command: CommandEvidence,
        tool_version: String,
        exit_status: ProcessTermination,
        parsed_result_digest: Digest,
    ) -> Result<Self, OutcomeError> {
        if !exit_status.is_success() {
            return Err(OutcomeError::NonZeroExit);
        }
        Ok(Self { command, tool_version, exit_status, parsed_result_digest })
    }

    #[must_use]
    pub const fn command(&self) -> &CommandEvidence {
        &self.command
    }

    #[must_use]
    pub fn tool_version(&self) -> &str {
        &self.tool_version
    }

    #[must_use]
    pub const fn exit_status(&self) -> ProcessTermination {
        self.exit_status
    }

    #[must_use]
    pub const fn parsed_result_digest(&self) -> &Digest {
        &self.parsed_result_digest
    }
}

#[derive(Deserialize)]
struct LaneEvidenceWire {
    command: CommandEvidence,
    tool_version: String,
    exit_status: ProcessTermination,
    parsed_result_digest: Digest,
}

impl<'de> Deserialize<'de> for LaneEvidence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = LaneEvidenceWire::deserialize(deserializer)?;
        Self::new(wire.command, wire.tool_version, wire.exit_status, wire.parsed_result_digest)
            .map_err(serde::de::Error::custom)
    }
}

/// Evidence of the command that was executed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandEvidence {
    executable: String,
    argv: Box<[String]>,
}

impl CommandEvidence {
    /// Construct command evidence.
    ///
    /// # Errors
    /// - [`OutcomeError::EmptyArgv`] if `argv` is empty.
    /// - [`OutcomeError::Argv0Mismatch`] if `argv.first()` does not equal `executable`.
    pub fn new(executable: String, argv: Box<[String]>) -> Result<Self, OutcomeError> {
        if argv.is_empty() {
            return Err(OutcomeError::EmptyArgv);
        }
        match argv.first() {
            Some(first) if first == &executable => Ok(Self { executable, argv }),
            Some(found) => {
                Err(OutcomeError::Argv0Mismatch { expected: executable, found: found.clone() })
            }
            // Cannot reach: argv is non-empty (checked above)
            None => Err(OutcomeError::EmptyArgv),
        }
    }

    #[must_use]
    pub fn executable(&self) -> &str {
        &self.executable
    }

    #[must_use]
    pub fn argv(&self) -> &[String] {
        &self.argv
    }
}

#[derive(Deserialize)]
struct CommandEvidenceWire {
    executable: String,
    argv: Box<[String]>,
}

impl<'de> Deserialize<'de> for CommandEvidence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = CommandEvidenceWire::deserialize(deserializer)?;
        Self::new(wire.executable, wire.argv).map_err(serde::de::Error::custom)
    }
}

/// Outcome of a single lane execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaneOutcome {
    Clean { evidence: LaneEvidence },
    Findings(Box<[Finding]>),
    Failed(LaneFailure),
    Skipped(SkipReason),
}

impl LaneOutcome {
    /// Construct a clean lane outcome from validated evidence.
    ///
    /// # Errors
    /// Returns [`OutcomeError::NonZeroExit`] if the evidence does not record a
    /// successful process exit.
    pub fn clean(evidence: LaneEvidence) -> Result<Self, OutcomeError> {
        if !evidence.exit_status().is_success() {
            return Err(OutcomeError::NonZeroExit);
        }
        Ok(Self::Clean { evidence })
    }

    /// Construct a findings lane outcome.
    #[must_use]
    pub const fn findings(findings: Box<[Finding]>) -> Self {
        Self::Findings(findings)
    }

    /// Construct a failed lane outcome.
    #[must_use]
    pub const fn failed(failure: LaneFailure) -> Self {
        Self::Failed(failure)
    }

    /// Construct a skipped lane outcome.
    #[must_use]
    pub const fn skipped(reason: SkipReason) -> Self {
        Self::Skipped(reason)
    }

    #[must_use]
    pub const fn is_pass(&self) -> bool {
        matches!(self, Self::Clean { .. } | Self::Skipped { .. })
    }

    #[must_use]
    pub const fn is_findings(&self) -> bool {
        matches!(self, Self::Findings { .. })
    }

    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    #[must_use]
    pub const fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped { .. })
    }
}

#[derive(Serialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
enum LaneOutcomeRef<'a> {
    Clean { evidence: &'a LaneEvidence },
    Findings { findings: &'a [Finding] },
    Failed { failure: &'a LaneFailure },
    Skipped { reason: SkipReason },
}

impl Serialize for LaneOutcome {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Clean { evidence } => LaneOutcomeRef::Clean { evidence }.serialize(serializer),
            Self::Findings(findings) => LaneOutcomeRef::Findings { findings }.serialize(serializer),
            Self::Failed(failure) => LaneOutcomeRef::Failed { failure }.serialize(serializer),
            Self::Skipped(reason) => {
                LaneOutcomeRef::Skipped { reason: *reason }.serialize(serializer)
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
enum LaneOutcomeWire {
    Clean { evidence: LaneEvidence },
    Findings { findings: Box<[Finding]> },
    Failed { failure: LaneFailure },
    Skipped { reason: SkipReason },
}

impl<'de> Deserialize<'de> for LaneOutcome {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match LaneOutcomeWire::deserialize(deserializer)? {
            LaneOutcomeWire::Clean { evidence } => {
                Self::clean(evidence).map_err(serde::de::Error::custom)
            }
            LaneOutcomeWire::Findings { findings } => Ok(Self::Findings(findings)),
            LaneOutcomeWire::Failed { failure } => Ok(Self::Failed(failure)),
            LaneOutcomeWire::Skipped { reason } => Ok(Self::Skipped(reason)),
        }
    }
}
