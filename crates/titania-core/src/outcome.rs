//! Outcome of a single lane execution.
//!
//! Each lane produces exactly one [`LaneOutcome`]: clean, findings, failure,
//! or skipped. Findings with only informational effect are treated as pass-shaped.

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
        exit_status
            .is_success()
            .then_some(Self { command, tool_version, exit_status, parsed_result_digest })
            .ok_or(OutcomeError::NonZeroExit)
    }

    /// Command evidence captured for the clean lane.
    #[must_use]
    pub const fn command(&self) -> &CommandEvidence {
        &self.command
    }

    /// Version string reported by the executed tool.
    #[must_use]
    pub fn tool_version(&self) -> &str {
        &self.tool_version
    }

    /// Process exit status captured for the lane.
    #[must_use]
    pub const fn exit_status(&self) -> ProcessTermination {
        self.exit_status
    }

    /// Digest of the parsed lane result payload.
    #[must_use]
    pub const fn parsed_result_digest(&self) -> &Digest {
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
        match argv.first() {
            Some(first) if first == &executable => Ok(Self { executable, argv }),
            Some(found) => Err(argv0_mismatch(executable, found)),
            None => Err(OutcomeError::EmptyArgv),
        }
    }

    /// Executable used as `argv[0]`.
    #[must_use]
    pub fn executable(&self) -> &str {
        &self.executable
    }

    /// Full argument vector used for command execution.
    #[must_use]
    pub fn argv(&self) -> &[String] {
        &self.argv
    }
}

fn argv0_mismatch(expected: String, found: &str) -> OutcomeError {
    OutcomeError::Argv0Mismatch { expected, found: found.to_owned() }
}

/// Outcome of a single lane execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum LaneOutcome {
    /// Lane completed cleanly and carries command evidence.
    Clean {
        /// Evidence proving the clean lane execution.
        evidence: LaneEvidence,
    },
    /// Lane completed and emitted one or more findings.
    Findings {
        /// Findings emitted by the lane.
        findings: Box<[Finding]>,
    },
    /// Lane failed before producing a clean or findings verdict.
    Failed(LaneFailure),
    /// Lane was intentionally skipped for a recorded reason.
    Skipped {
        /// Reason the lane was skipped.
        reason: SkipReason,
    },
}

impl LaneOutcome {
    /// Whether this outcome is acceptable for a passing report.
    ///
    /// `Clean` and `Skipped` always pass. `Findings` passes only when every
    /// finding is informational; any rejecting finding blocks the pass shape.
    #[must_use]
    pub fn is_pass(&self) -> bool {
        match self {
            Self::Clean { .. } | Self::Skipped { .. } => true,
            Self::Findings { findings } => findings.iter().all(Finding::is_informational),
            Self::Failed(_) => false,
        }
    }

    /// Whether this outcome contains code or policy findings.
    #[must_use]
    pub const fn is_findings(&self) -> bool {
        matches!(self, Self::Findings { .. })
    }

    /// Whether this outcome is a lane execution failure.
    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Whether this outcome records a skipped lane.
    #[must_use]
    pub const fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped { .. })
    }
}
