//! Outcome of a single lane execution.
//!
//! Each lane produces exactly one [`LaneOutcome`]: clean, findings, failure,
//! or skipped. Findings with only informational effect are treated as pass-shaped.
//!
//! Wire-format invariants are enforced by hand-written `Deserialize` impls:
//! [`CommandEvidence`] cannot bypass `argv[0] == executable`, [`LaneEvidence`]
//! cannot promote a non-success termination into clean evidence, and
//! [`LaneOutcome::Findings`] cannot deserialize an empty findings list into a
//! vacuous pass. Constructors are unchanged.

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    digest::Digest,
    error::OutcomeError,
    failure::{LaneFailure, ProcessTermination},
    finding::Finding,
    proof_id::ToolKind,
};

/// Why a lane was skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkipReason {
    /// A prior compilation failure caused dependent lanes to be skipped.
    PriorCompilationFailure,
    /// The scope did not include this lane.
    NotSelectedByScope,
    /// No applicable files were found for this lane.
    NotApplicable,
    /// The lane is disabled in the policy configuration.
    PolicyDisabled,
    /// The lane's required external tool is not installed on the host, or its
    /// installed version is older than the contract floor. The payload names
    /// which tool is unavailable (per v1.5 spec §7).
    ToolUnavailable(ToolKind),
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

/// Wire mirror for [`LaneEvidence`] used only by the `Deserialize` impl.
///
/// Mirrors the wire shape byte-for-byte so the deserializer can run the same
/// constructor invariants as the public smart constructor
/// ([`LaneEvidence::new`]). Keeping the mirror local avoids leaking private
/// field visibility and lets the public API stay sealed behind `new`.
#[derive(Deserialize)]
struct LaneEvidenceWire {
    command: CommandEvidence,
    tool_version: String,
    exit_status: ProcessTermination,
    parsed_result_digest: Digest,
}

impl<'de> Deserialize<'de> for LaneEvidence {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = LaneEvidenceWire::deserialize(deserializer)?;
        Self::new(wire.command, wire.tool_version, wire.exit_status, wire.parsed_result_digest)
            .map_err(serde::de::Error::custom)
    }
}

/// How a command's findings were produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandMode {
    /// Lane spawned a separate process whose output it captured.
    ChildProcess,
    /// Lane executed the rule/logic in-process inside this binary. The
    /// `executable` and `argv` record which invocation would yield the
    /// same outcome; not a separate binary on `PATH`.
    Embedded,
}

/// Evidence of the command that was executed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandEvidence {
    executable: String,
    argv: Box<[String]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mode: Option<CommandMode>,
}

impl CommandEvidence {
    /// Construct command evidence for a child-process lane (default mode).
    ///
    /// # Errors
    /// - [`OutcomeError::EmptyArgv`] if `argv` is empty.
    /// - [`OutcomeError::Argv0Mismatch`] if `argv.first()` does not equal `executable`.
    pub fn new(executable: String, argv: Box<[String]>) -> Result<Self, OutcomeError> {
        Self::with_mode(executable, argv, None)
    }

    /// Construct command evidence for an embedded (in-process) lane.
    ///
    /// Records `executable` and `argv` so receipt auditors see a real
    /// binary path, then marks `mode: embedded` so consumers can
    /// distinguish in-process lanes from shell-out lanes (bead tn-e65p).
    ///
    /// # Errors
    /// Returns [`OutcomeError::EmptyArgv`] or [`OutcomeError::Argv0Mismatch`]
    /// under the same conditions as [`Self::new`].
    pub fn embedded(executable: String, argv: Box<[String]>) -> Result<Self, OutcomeError> {
        Self::with_mode(executable, argv, Some(CommandMode::Embedded))
    }

    /// Construct command evidence with an explicit mode.
    ///
    /// # Errors
    /// Returns [`OutcomeError::EmptyArgv`] if `argv` is empty.
    /// Returns [`OutcomeError::Argv0Mismatch`] if `argv.first()` does not
    /// equal `executable`.
    fn with_mode(
        executable: String,
        argv: Box<[String]>,
        mode: Option<CommandMode>,
    ) -> Result<Self, OutcomeError> {
        match argv.first() {
            Some(first) if first == &executable => Ok(Self { executable, argv, mode }),
            Some(found) => Err(argv0_mismatch(executable, found)),
            None => Err(OutcomeError::EmptyArgv),
        }
    }

    /// How this command's findings were produced (if recorded).
    #[must_use]
    pub const fn mode(&self) -> Option<CommandMode> {
        self.mode
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

/// Wire mirror for [`CommandEvidence`] used only by the `Deserialize` impl.
///
/// Mirrors the wire shape byte-for-byte so the deserializer can run the
/// constructor invariant (argv non-empty, argv[0] == executable). The
/// `mode` field is optional in the wire format and re-emitted by
/// [`CommandEvidence::Serialize`] when present, so the mirror omits fields
/// the deserializer does not need to enforce.
#[derive(Deserialize)]
struct CommandEvidenceWire {
    executable: String,
    argv: Box<[String]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mode: Option<CommandMode>,
}

impl<'de> Deserialize<'de> for CommandEvidence {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = CommandEvidenceWire::deserialize(deserializer)?;
        Self::with_mode(wire.executable, wire.argv, wire.mode).map_err(serde::de::Error::custom)
    }
}

fn argv0_mismatch(expected: String, found: &str) -> OutcomeError {
    OutcomeError::Argv0Mismatch { expected, found: found.to_owned() }
}

/// Outcome of a single lane execution.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    Failed {
        /// The lane failure classification.
        failure: LaneFailure,
    },
    /// Lane was intentionally skipped for a recorded reason.
    Skipped {
        /// Reason the lane was skipped.
        reason: SkipReason,
    },
}

#[derive(Serialize)]
enum LaneOutcomeWriteWire<'a> {
    Clean { evidence: &'a LaneEvidence },
    Findings(&'a [Finding]),
    Failed(&'a LaneFailure),
    Skipped(SkipReason),
}

#[derive(Deserialize)]
enum LaneOutcomeReadWire {
    Clean { evidence: LaneEvidence },
    Findings(Box<[Finding]>),
    Failed(LaneFailure),
    Skipped(SkipReason),
}

impl Serialize for LaneOutcome {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Clean { evidence } => LaneOutcomeWriteWire::Clean { evidence },
            Self::Findings { findings } => LaneOutcomeWriteWire::Findings(findings),
            Self::Failed { failure } => LaneOutcomeWriteWire::Failed(failure),
            Self::Skipped { reason } => LaneOutcomeWriteWire::Skipped(*reason),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LaneOutcome {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = LaneOutcomeReadWire::deserialize(deserializer)?;
        // Reject empty findings lists on the wire so a crafted artifact
        // cannot deserialize into a vacuous pass. The constructor path
        // is unchanged: empty findings via in-process construction
        // remains the caller's responsibility.
        match wire {
            LaneOutcomeReadWire::Clean { evidence } => Ok(Self::Clean { evidence }),
            LaneOutcomeReadWire::Findings(findings) => findings_non_empty(findings)
                .map(|findings| Self::Findings { findings })
                .map_err(serde::de::Error::custom),
            LaneOutcomeReadWire::Failed(failure) => Ok(Self::Failed { failure }),
            LaneOutcomeReadWire::Skipped(reason) => Ok(Self::Skipped { reason }),
        }
    }
}

/// Reject an empty wire-form findings list, mapping the typed
/// [`OutcomeError::EmptyFindings`] into a `serde::de::Error` for the
/// `Deserialize` impl. Extracted to keep the parent `Deserialize` flat.
///
/// # Errors
/// - [`OutcomeError::EmptyFindings`] when `findings` is empty.
fn findings_non_empty(findings: Box<[Finding]>) -> Result<Box<[Finding]>, OutcomeError> {
    if findings.is_empty() { Err(OutcomeError::EmptyFindings) } else { Ok(findings) }
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
            Self::Failed { .. } => false,
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
