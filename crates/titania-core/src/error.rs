//! Typed errors for the domain primitives. One error enum per constructor,
//! using `thiserror` so the messages are stable and machine-consumable.

use std::io;

use thiserror::Error;

/// Errors produced by [`crate::Digest::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DigestError {
    #[error("digest must be exactly 64 characters, got {0}")]
    WrongLength(usize),
    #[error("digest must contain only lowercase hex characters; bad position {0}")]
    NonHexChar(usize),
}

/// Errors produced by [`crate::RuleId::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RuleIdError {
    #[error("rule id must not be empty")]
    Empty,
    #[error("rule id must contain at least one underscore")]
    NoUnderscore,
    #[error("rule id must be uppercase ASCII; bad character {0:?} at byte {1}")]
    NotUppercase(char, usize),
}

/// Errors produced by [`crate::WorkspacePath::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkspacePathError {
    #[error("workspace path must not be empty")]
    Empty,
    #[error("workspace path must not start with '/'")]
    LeadingSlash,
    #[error("workspace path must not contain '..'")]
    ContainsDotDot,
    #[error("workspace path must not contain backslashes")]
    ContainsBackslash,
    #[error("workspace path must not contain null bytes")]
    ContainsNull,
    #[error("workspace path must not contain control characters; bad byte {0}")]
    ControlByte(u8),
}

/// Errors produced by [`crate::TextRange::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TextRangeError {
    #[error("text range end ({end}) must be >= start ({start})")]
    EndBeforeStart { start: u32, end: u32 },
}

/// Errors produced by [`crate::TargetProject::try_from_path`] and
/// [`crate::discover_target`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TargetProjectError {
    #[error("target project path must not be empty")]
    Empty,
    #[error("target project path must be absolute, got {0:?}")]
    NonAbsolute(String),
    #[error("target project path is not valid UTF-8")]
    NotUtf8,
    #[error("target project path does not exist")]
    NotFound,
    #[error("target project path exists but is not a directory")]
    NotADirectory,
    #[error("target project directory does not contain a Cargo.toml file")]
    NoCargoToml,
    #[error("target project Cargo.toml path exists but is not a file")]
    CargoTomlNotFile,
    #[error("target project Cargo.toml is malformed: {path}")]
    MalformedCargoToml { path: String },
    #[error("I/O error accessing {path}: {kind:?}")]
    Io { path: String, kind: io::ErrorKind },
}

/// Errors produced by [`crate::QualityReceipt`] and [`crate::LaneDigest`]
/// constructors or deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReceiptError {
    #[error("unsupported receipt schema version {0}")]
    UnsupportedSchemaVersion(u32),
    #[error("lane name must not be empty")]
    EmptyLaneName,
    #[error("lane name must not contain NUL bytes")]
    InvalidLaneName,
    #[error("lane passed count {passed} exceeds scanned count {scanned}")]
    PassedExceedsScanned { passed: u32, scanned: u32 },
    #[error("receipt finished_at {finished_at} is before started_at {started_at}")]
    FinishedBeforeStarted { started_at: u64, finished_at: u64 },
    #[error("receipt target_root must not be empty")]
    TargetRootEmpty,
    #[error("receipt target_root must be absolute, got {0:?}")]
    TargetRootNonAbsolute(String),
    #[error("receipt target_root must not contain NUL bytes")]
    TargetRootContainsNul,
}

/// Errors produced by [`crate::Lane::from_str`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LaneError {
    #[error("unknown lane: {0}")]
    UnknownLane(String),
}

/// Errors produced by [`crate::GateScope::from_str`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GateScopeError {
    #[error("unknown scope: {0}")]
    UnknownScope(String),
}

/// Errors produced by [`crate::Location::span`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LocationError {
    #[error("line_start must be >= 1")]
    LineStartBeforeOne,
    #[error("line_end ({line_end}) must be >= line_start ({line_start})")]
    EndBeforeStart { line_start: u32, line_end: u32 },
    #[error("col_end ({col_end}) must be >= col_start ({col_start})")]
    ColEndBeforeStart { col_start: u32, col_end: u32 },
}

/// Errors produced by [`crate::RepairHint::patch`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RepairHintError {
    #[error("patch range must be non-empty (width > 0)")]
    EmptyRange,
}

/// Errors produced by [`crate::Finding::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FindingError {
    #[error(transparent)]
    Location(#[from] LocationError),
    #[error(transparent)]
    RepairHint(#[from] RepairHintError),
}

/// Errors produced by [`crate::ProcessTermination::signaled`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FailureError {
    #[error("signal number must be 1–31, got {0}")]
    InvalidSignal(i32),
}

/// Errors produced by [`crate::CommandEvidence::new`] and [`crate::LaneEvidence::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OutcomeError {
    #[error("argv must not be empty")]
    EmptyArgv,
    #[error("argv[0] ({found}) must equal executable ({expected})")]
    Argv0Mismatch { expected: String, found: String },
    #[error("exit status must be Exited(0) for Clean lanes")]
    NonZeroExit,
}

/// Errors produced by [`crate::Report::reject`] and [`crate::Report::pass`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReportError {
    #[error("reject must have at least one non-empty collection")]
    EmptyReject,
    #[error("pass must have at least one lane outcome")]
    EmptyPerLane,
}

/// Aggregate for callers that want a single error type across primitives.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    #[error(transparent)]
    Digest(#[from] DigestError),
    #[error(transparent)]
    RuleId(#[from] RuleIdError),
    #[error(transparent)]
    WorkspacePath(#[from] WorkspacePathError),
    #[error(transparent)]
    TextRange(#[from] TextRangeError),
    #[error(transparent)]
    TargetProject(#[from] TargetProjectError),
    #[error(transparent)]
    Receipt(#[from] ReceiptError),
    #[error(transparent)]
    Lane(#[from] LaneError),
    #[error(transparent)]
    GateScope(#[from] GateScopeError),
    #[error(transparent)]
    Location(#[from] LocationError),
    #[error(transparent)]
    RepairHint(#[from] RepairHintError),
    #[error(transparent)]
    Finding(#[from] FindingError),
    #[error(transparent)]
    Failure(#[from] FailureError),
    #[error(transparent)]
    Outcome(#[from] OutcomeError),
    #[error(transparent)]
    Report(#[from] ReportError),
}
