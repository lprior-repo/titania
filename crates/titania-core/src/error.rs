//! Typed errors for the domain primitives. One error enum per constructor,
//! using `thiserror` so the messages are stable and machine-consumable.

use std::io;

use thiserror::Error;

/// Errors produced by [`crate::Digest::from_hex`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DigestError {
    /// Digest text was not exactly 64 hexadecimal characters.
    #[error("digest must be exactly 64 characters, got {0}")]
    WrongLength(usize),
    /// Digest text contained a non-lowercase-hex character at this byte index.
    #[error("digest must contain only lowercase hex characters; bad position {0}")]
    NonHexChar(usize),
}

/// Errors produced by [`crate::RuleId::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RuleIdError {
    /// Rule identifier was empty.
    #[error("rule id must not be empty")]
    Empty,
    /// Rule identifier did not contain the required underscore separator.
    #[error("rule id must contain at least one underscore")]
    NoUnderscore,
    /// Rule identifier contained a non-uppercase-ASCII character.
    #[error("rule id must be uppercase ASCII; bad character {0:?} at byte {1}")]
    NotUppercase(char, usize),
}

/// Errors produced by [`crate::WorkspacePath::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkspacePathError {
    /// Workspace-relative path was empty.
    #[error("workspace path must not be empty")]
    Empty,
    /// Workspace-relative path started with `/`.
    #[error("workspace path must not start with '/'")]
    LeadingSlash,
    /// Workspace-relative path contained a `..` component.
    #[error("workspace path must not contain '..'")]
    ContainsDotDot,
    /// Workspace-relative path contained a backslash separator.
    #[error("workspace path must not contain backslashes")]
    ContainsBackslash,
    /// Workspace-relative path contained a NUL byte.
    #[error("workspace path must not contain null bytes")]
    ContainsNull,
    /// Workspace-relative path contained an ASCII control byte.
    #[error("workspace path must not contain control characters; bad byte {0}")]
    ControlByte(u8),
}

/// Errors produced by [`crate::TextRange::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TextRangeError {
    /// Range end was before range start.
    #[error("text range end ({end}) must be >= start ({start})")]
    EndBeforeStart {
        /// Start byte offset supplied by the caller.
        start: u32,
        /// End byte offset supplied by the caller.
        end: u32,
    },
}

/// Errors produced by [`crate::TargetProject::try_from_path`] and
/// [`crate::discover_target`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TargetProjectError {
    /// Target project path was empty.
    #[error("target project path must not be empty")]
    Empty,
    /// Target project path was not absolute.
    #[error("target project path must be absolute, got {0:?}")]
    NonAbsolute(String),
    /// Target project path could not be represented as UTF-8.
    #[error("target project path is not valid UTF-8")]
    NotUtf8,
    /// Target project path did not exist.
    #[error("target project path does not exist")]
    NotFound,
    /// Target project path existed but was not a directory.
    #[error("target project path exists but is not a directory")]
    NotADirectory,
    /// Target project directory did not contain a `Cargo.toml` file.
    #[error("target project directory does not contain a Cargo.toml file")]
    NoCargoToml,
    /// Target project `Cargo.toml` path existed but was not a file.
    #[error("target project Cargo.toml path exists but is not a file")]
    CargoTomlNotFile,
    /// Target project manifest could not be parsed.
    #[error("target project Cargo.toml is malformed: {path}")]
    MalformedCargoToml {
        /// Workspace or filesystem path to the malformed manifest.
        path: String,
    },
    /// Target project discovery hit an I/O error.
    #[error("I/O error accessing {path}: {kind:?}")]
    Io {
        /// Filesystem path being accessed when the error occurred.
        path: String,
        /// Stable [`io::ErrorKind`] observed from the filesystem operation.
        kind: io::ErrorKind,
    },
}

/// Errors produced by [`crate::QualityReceipt`] and [`crate::LaneDigest`]
/// constructors or deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReceiptError {
    /// Receipt schema version was not supported by this crate.
    #[error("unsupported receipt schema version {0}")]
    UnsupportedSchemaVersion(u32),
    /// Lane name was empty.
    #[error("lane name must not be empty")]
    EmptyLaneName,
    /// Lane name contained a NUL byte.
    #[error("lane name must not contain NUL bytes")]
    InvalidLaneName,
    /// A lane digest reported more passing items than scanned items.
    #[error("lane passed count {passed} exceeds scanned count {scanned}")]
    PassedExceedsScanned {
        /// Number of items reported as passing.
        passed: u32,
        /// Number of items reported as scanned.
        scanned: u32,
    },
    /// Receipt finish timestamp was earlier than its start timestamp.
    #[error("receipt finished_at {finished_at} is before started_at {started_at}")]
    FinishedBeforeStarted {
        /// Receipt start timestamp.
        started_at: u64,
        /// Receipt finish timestamp.
        finished_at: u64,
    },
    /// A v1 quality receipt contained no per-lane receipt entries.
    #[error("quality receipt must include at least one lane receipt")]
    EmptyLaneReceiptList,
    /// Receipt target root was empty.
    #[error("receipt target_root must not be empty")]
    TargetRootEmpty,
    /// Receipt target root was not absolute.
    #[error("receipt target_root must be absolute, got {0:?}")]
    TargetRootNonAbsolute(String),
    /// Receipt target root contained a NUL byte.
    #[error("receipt target_root must not contain NUL bytes")]
    TargetRootContainsNul,
}

/// Errors produced by [`crate::Lane`] string parsing.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LaneError {
    /// Lane string did not match any known v1 lane.
    #[error("unknown lane: {0}")]
    UnknownLane(String),
}

/// Errors produced by [`crate::GateScope`] string parsing.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GateScopeError {
    /// Scope string did not match any known gate scope.
    #[error("unknown scope: {0}")]
    UnknownScope(String),
}

/// Errors produced by [`crate::Location::span`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LocationError {
    /// Span line start was less than one.
    #[error("line_start must be >= 1")]
    LineStartBeforeOne,
    /// Span end line was before the span start line.
    #[error("line_end ({line_end}) must be >= line_start ({line_start})")]
    EndBeforeStart {
        /// First line of the reported span.
        line_start: u32,
        /// Last line of the reported span.
        line_end: u32,
    },
    /// Span end column was before the span start column.
    #[error("col_end ({col_end}) must be >= col_start ({col_start})")]
    ColEndBeforeStart {
        /// First column of the reported span.
        col_start: u32,
        /// Last column of the reported span.
        col_end: u32,
    },
}

/// Errors produced by [`crate::RepairHint::patch`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RepairHintError {
    /// Patch replacement range was empty.
    #[error("patch range must be non-empty (width > 0)")]
    EmptyRange,
}

/// Errors produced by [`crate::Finding::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FindingError {
    /// Finding location failed validation.
    #[error(transparent)]
    Location(#[from] LocationError),
    /// Finding repair hint failed validation.
    #[error(transparent)]
    RepairHint(#[from] RepairHintError),
}

/// Errors produced by [`crate::ProcessTermination::signaled`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FailureError {
    /// Signal number was outside the supported Unix signal range.
    #[error("signal number must be 1–31, got {0}")]
    InvalidSignal(i32),
}

/// Errors produced by [`crate::CommandEvidence::new`] and [`crate::LaneEvidence::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OutcomeError {
    /// Captured command argument vector was empty.
    #[error("argv must not be empty")]
    EmptyArgv,
    /// Captured `argv[0]` did not match the executable field.
    #[error("argv[0] ({found}) must equal executable ({expected})")]
    Argv0Mismatch {
        /// Expected executable name.
        expected: String,
        /// Actual first argument value.
        found: String,
    },
    /// Clean lane evidence carried a non-zero process exit.
    #[error("exit status must be Exited(0) for Clean lanes")]
    NonZeroExit,
}

/// Errors produced by [`crate::Report::reject`] and [`crate::Report::pass`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReportError {
    /// Reject report did not contain findings or failures.
    #[error("reject must have at least one non-empty collection")]
    EmptyReject,
    /// Pass report did not contain any lane outcomes.
    #[error("pass must have at least one lane outcome")]
    EmptyPerLane,
}

/// Aggregate for callers that want a single error type across primitives.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    /// Digest construction failed.
    #[error(transparent)]
    Digest(#[from] DigestError),
    /// Rule identifier construction failed.
    #[error(transparent)]
    RuleId(#[from] RuleIdError),
    /// Workspace path construction failed.
    #[error(transparent)]
    WorkspacePath(#[from] WorkspacePathError),
    /// Text range construction failed.
    #[error(transparent)]
    TextRange(#[from] TextRangeError),
    /// Target project discovery or validation failed.
    #[error(transparent)]
    TargetProject(#[from] TargetProjectError),
    /// Receipt construction failed.
    #[error(transparent)]
    Receipt(#[from] ReceiptError),
    /// Lane parsing failed.
    #[error(transparent)]
    Lane(#[from] LaneError),
    /// Gate scope parsing failed.
    #[error(transparent)]
    GateScope(#[from] GateScopeError),
    /// Finding location validation failed.
    #[error(transparent)]
    Location(#[from] LocationError),
    /// Repair hint validation failed.
    #[error(transparent)]
    RepairHint(#[from] RepairHintError),
    /// Finding construction failed.
    #[error(transparent)]
    Finding(#[from] FindingError),
    /// Lane failure construction failed.
    #[error(transparent)]
    Failure(#[from] FailureError),
    /// Lane outcome construction failed.
    #[error(transparent)]
    Outcome(#[from] OutcomeError),
    /// Report construction failed.
    #[error(transparent)]
    Report(#[from] ReportError),
}
