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

/// Evidence attached to a clean lane outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        Ok(LaneEvidence { command, tool_version, exit_status, parsed_result_digest })
    }

    #[must_use]
    pub fn command(&self) -> &CommandEvidence {
        &self.command
    }

    #[must_use]
    pub fn tool_version(&self) -> &str {
        &self.tool_version
    }

    #[must_use]
    pub fn exit_status(&self) -> ProcessTermination {
        self.exit_status
    }

    #[must_use]
    pub fn parsed_result_digest(&self) -> &Digest {
        &self.parsed_result_digest
    }
}

/// Evidence of the command that was executed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
            Some(first) if first == &executable => Ok(CommandEvidence { executable, argv }),
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

/// Outcome of a single lane execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum LaneOutcome {
    Clean { evidence: LaneEvidence },
    Findings(Box<[Finding]>),
    Failed(LaneFailure),
    Skipped(SkipReason),
}

impl LaneOutcome {
    #[must_use]
    pub fn is_pass(&self) -> bool {
        matches!(self, LaneOutcome::Clean { .. } | LaneOutcome::Skipped { .. })
    }

    #[must_use]
    pub fn is_findings(&self) -> bool {
        matches!(self, LaneOutcome::Findings { .. })
    }

    #[must_use]
    pub fn is_failed(&self) -> bool {
        matches!(self, LaneOutcome::Failed { .. })
    }

    #[must_use]
    pub fn is_skipped(&self) -> bool {
        matches!(self, LaneOutcome::Skipped { .. })
    }
}
