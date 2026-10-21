use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    digest::Digest,
    error::OutcomeError,
    failure::{LaneFailure, ProcessTermination},
    finding::Finding,
    proof_id::ToolKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkipReason {
    PriorCompilationFailure,
    NotSelectedByScope,
    NotApplicable,
    PolicyDisabled,
    ToolUnavailable(ToolKind),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LaneEvidence {
    command: CommandEvidence,
    tool_version: String,
    exit_status: ProcessTermination,
    parsed_result_digest: Digest,
}

impl LaneEvidence {
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
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = LaneEvidenceWire::deserialize(deserializer)?;
        Self::new(wire.command, wire.tool_version, wire.exit_status, wire.parsed_result_digest)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandMode {
    ChildProcess,
    Embedded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandEvidence {
    executable: String,
    argv: Box<[String]>,
    #[serde(default, skip_serializing)]
    mode: Option<CommandMode>,
}

impl CommandEvidence {
    pub fn new(executable: String, argv: Box<[String]>) -> Result<Self, OutcomeError> {
        Self::with_mode(executable, argv, None)
    }

    pub fn embedded(executable: String, argv: Box<[String]>) -> Result<Self, OutcomeError> {
        Self::with_mode(executable, argv, Some(CommandMode::Embedded))
    }

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

    #[must_use]
    pub const fn mode(&self) -> Option<CommandMode> {
        self.mode
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
    #[serde(default)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaneOutcome {
    Clean { evidence: LaneEvidence },
    Findings { findings: Box<[Finding]> },
    Failed { failure: LaneFailure },
    Skipped { reason: SkipReason },
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

fn findings_non_empty(findings: Box<[Finding]>) -> Result<Box<[Finding]>, OutcomeError> {
    if findings.is_empty() { Err(OutcomeError::EmptyFindings) } else { Ok(findings) }
}

impl LaneOutcome {
    #[must_use]
    pub fn is_pass(&self) -> bool {
        match self {
            Self::Clean { .. } | Self::Skipped { .. } => true,
            Self::Findings { findings } => findings.iter().all(Finding::is_informational),
            Self::Failed { .. } => false,
        }
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
