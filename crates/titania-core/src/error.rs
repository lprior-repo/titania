//! Typed errors for the domain primitives. One error enum per constructor,
//! using `thiserror` so the messages are stable and machine-consumable.

use std::io;

use thiserror::Error;

/// Errors produced by [`crate::Digest::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DigestError {
    /// Digest was not exactly 64 characters long.
    #[error("digest must be exactly 64 characters, got {0}")]
    WrongLength(usize),
    /// Digest contained a non-lowercase-hex character at the given byte.
    #[error("digest must contain only lowercase hex characters; bad position {0}")]
    NonHexChar(usize),
}

/// Errors produced by [`crate::RuleId::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RuleIdError {
    /// Rule id was the empty string.
    #[error("rule id must not be empty")]
    Empty,
    /// Rule id did not contain at least one underscore separator.
    #[error("rule id must contain at least one underscore")]
    NoUnderscore,
    /// Rule id contained a non-uppercase ASCII character.
    #[error("rule id must be uppercase ASCII; bad character {0:?} at byte {1}")]
    NotUppercase(char, usize),
    /// Rule id exceeded the maximum allowed byte length.
    #[error("rule id must not exceed {max} bytes, got {got}")]
    TooLong {
        /// Maximum permitted byte length.
        max: usize,
        /// Observed byte length.
        got: usize,
    },
}

/// Errors produced by [`crate::WorkspacePath::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkspacePathError {
    /// Workspace path was the empty string.
    #[error("workspace path must not be empty")]
    Empty,
    /// Workspace path began with a leading forward slash.
    #[error("workspace path must not start with '/'")]
    LeadingSlash,
    /// Workspace path contained a parent-directory (`..`) component.
    #[error("workspace path must not contain '..'")]
    ContainsDotDot,
    /// Workspace path contained a backslash separator.
    #[error("workspace path must not contain backslashes")]
    ContainsBackslash,
    /// Workspace path contained a NUL byte.
    #[error("workspace path must not contain null bytes")]
    ContainsNull,
    /// Workspace path contained an ASCII control byte.
    #[error("workspace path must not contain control characters; bad byte {0}")]
    ControlByte(u8),
}

/// Errors produced by [`crate::TextRange::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TextRangeError {
    /// Range end byte offset preceded the start byte offset.
    #[error("text range end ({end}) must be >= start ({start})")]
    EndBeforeStart {
        /// Inclusive start byte offset.
        start: u32,
        /// Exclusive end byte offset.
        end: u32,
    },
}

/// Errors produced by [`crate::TargetProject::try_from_path`] and
/// [`crate::discover_target`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TargetProjectError {
    /// Target project path was the empty string.
    #[error("target project path must not be empty")]
    Empty,
    /// Target project path was not absolute.
    #[error("target project path must be absolute, got {0:?}")]
    NonAbsolute(String),
    /// Target project path was not valid UTF-8.
    #[error("target project path is not valid UTF-8")]
    NotUtf8,
    /// Target project path did not exist on disk.
    #[error("target project path does not exist")]
    NotFound,
    /// Target project path existed but was not a directory.
    #[error("target project path exists but is not a directory")]
    NotADirectory,
    /// Target project directory did not contain a `Cargo.toml` file.
    #[error("target project directory does not contain a Cargo.toml file")]
    NoCargoToml,
    /// Target project `Cargo.toml` path existed but was not a regular file.
    #[error("target project Cargo.toml path exists but is not a file")]
    CargoTomlNotFile,
    /// Target project `Cargo.toml` could not be parsed as valid TOML.
    #[error("target project Cargo.toml is malformed: {path}")]
    MalformedCargoToml {
        /// Path to the unparseable manifest.
        path: String,
    },
    /// I/O error while probing the target project path.
    #[error("I/O error accessing {path}: {kind:?}")]
    Io {
        /// Path that triggered the I/O failure.
        path: String,
        /// Underlying `std::io::Error` kind.
        kind: io::ErrorKind,
    },
}

/// Errors produced by [`crate::QualityReceipt`] and [`crate::LaneDigest`]
/// constructors or deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReceiptError {
    /// Receipt schema version is not supported by this build.
    #[error("unsupported receipt schema version {0}")]
    UnsupportedSchemaVersion(u32),
    /// Receipt lane name was the empty string.
    #[error("lane name must not be empty")]
    EmptyLaneName,
    /// Receipt lane name contained a NUL byte.
    #[error("lane name must not contain NUL bytes")]
    InvalidLaneName,
    /// Receipt lane `passed` count exceeded its `scanned` count.
    #[error("lane passed count {passed} exceeds scanned count {scanned}")]
    PassedExceedsScanned {
        /// Count of items that passed.
        passed: u32,
        /// Count of items that were scanned.
        scanned: u32,
    },
    /// Receipt `finished_at` timestamp preceded `started_at`.
    #[error("receipt finished_at {finished_at} is before started_at {started_at}")]
    FinishedBeforeStarted {
        /// Run start time, in Unix seconds.
        started_at: u64,
        /// Run finish time, in Unix seconds.
        finished_at: u64,
    },
    /// Receipt `target_root` was the empty string.
    #[error("receipt target_root must not be empty")]
    TargetRootEmpty,
    /// Receipt `target_root` was not an absolute path.
    #[error("receipt target_root must be absolute, got {0:?}")]
    TargetRootNonAbsolute(String),
    /// Receipt `target_root` contained a NUL byte.
    #[error("receipt target_root must not contain NUL bytes")]
    TargetRootContainsNul,
}

/// Aggregate for callers that want a single error type across primitives.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    /// Wrapped [`DigestError`].
    #[error(transparent)]
    Digest(#[from] DigestError),
    /// Wrapped [`RuleIdError`].
    #[error(transparent)]
    RuleId(#[from] RuleIdError),
    /// Wrapped [`WorkspacePathError`].
    #[error(transparent)]
    WorkspacePath(#[from] WorkspacePathError),
    /// Wrapped [`TextRangeError`].
    #[error(transparent)]
    TextRange(#[from] TextRangeError),
    /// Wrapped [`TargetProjectError`].
    #[error(transparent)]
    TargetProject(#[from] TargetProjectError),
    /// Wrapped [`ReceiptError`].
    #[error(transparent)]
    Receipt(#[from] ReceiptError),
}
