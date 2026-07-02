//! Typed errors for the domain primitives. One error enum per constructor,
//! using `thiserror` so the messages are stable and machine-consumable.

use std::io;

use thiserror::Error;

/// Errors produced by [`crate::Digest::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DigestError {
    /// Input length was not exactly 64 hex characters.
    #[error("digest must be exactly 64 characters, got {0}")]
    WrongLength(usize),
    /// Input contained a non-lowercase-hex byte at the given position.
    #[error("digest must contain only lowercase hex characters; bad position {0}")]
    NonHexChar(usize),
}

/// Errors produced by [`crate::RuleId::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RuleIdError {
    /// Input was the empty string.
    #[error("rule id must not be empty")]
    Empty,
    /// Input contained no underscore.
    #[error("rule id must contain at least one underscore")]
    NoUnderscore,
    /// Input contained a non-uppercase-ASCII character at the given byte offset.
    #[error("rule id must be uppercase ASCII; bad character {0:?} at byte {1}")]
    NotUppercase(char, usize),
    /// Input exceeded the configured maximum length.
    #[error("rule id must be at most {max} characters, got {actual}")]
    TooLong {
        /// Configured maximum length.
        max: usize,
        /// Actual length of the input.
        actual: usize,
    },
}

/// Errors produced by [`crate::WorkspacePath::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkspacePathError {
    /// Input was the empty string.
    #[error("workspace path must not be empty")]
    Empty,
    /// Input started with `/`; only relative paths are accepted.
    #[error("workspace path must not start with '/'")]
    LeadingSlash,
    /// Input contained a `..` path segment.
    #[error("workspace path must not contain '..'")]
    ContainsDotDot,
    /// Input contained a backslash byte.
    #[error("workspace path must not contain backslashes")]
    ContainsBackslash,
    /// Input contained a NUL byte.
    #[error("workspace path must not contain null bytes")]
    ContainsNull,
    /// Input contained an ASCII control byte (other than NUL, reported separately).
    #[error("workspace path must not contain control characters; bad byte {0}")]
    ControlByte(u8),
}

/// Errors produced by [`crate::TextRange::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TextRangeError {
    /// `end` was strictly less than `start`.
    #[error("text range end ({end}) must be >= start ({start})")]
    EndBeforeStart {
        /// Inclusive start byte position.
        start: u32,
        /// Exclusive end byte position.
        end: u32,
    },
}

/// Errors produced by [`crate::TargetProject::try_from_path`] and
/// [`crate::discover_target`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TargetProjectError {
    /// The path was empty after UTF-8 conversion.
    #[error("target project path must not be empty")]
    Empty,
    /// The path was relative; absolute paths are required.
    #[error("target project path must be absolute, got {0:?}")]
    NonAbsolute(String),
    /// The path contained non-UTF-8 bytes.
    #[error("target project path is not valid UTF-8")]
    NotUtf8,
    /// The path does not exist on the filesystem.
    #[error("target project path does not exist")]
    NotFound,
    /// The path exists but is not a directory.
    #[error("target project path exists but is not a directory")]
    NotADirectory,
    /// No `Cargo.toml` file was found at the project root.
    #[error("target project directory does not contain a Cargo.toml file")]
    NoCargoToml,
    /// A `Cargo.toml` path exists but is not a regular file.
    #[error("target project Cargo.toml path exists but is not a file")]
    CargoTomlNotFile,
    /// The selected `Cargo.toml` could not be parsed as TOML.
    #[error("target project Cargo.toml is malformed: {path}")]
    MalformedCargoToml {
        /// Manifest path that failed to parse.
        path: String,
    },
    /// Any other filesystem error with full path and kind.
    #[error("I/O error accessing {path}: {kind:?}")]
    Io {
        /// Filesystem path that produced the error.
        path: String,
        /// OS error classification.
        kind: io::ErrorKind,
    },
}

/// Errors produced by [`crate::QualityReceipt`] and [`crate::LaneDigest`]
/// constructors or deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReceiptError {
    /// The deserialized or constructed schema version is not the current one.
    #[error("unsupported receipt schema version {0}")]
    UnsupportedSchemaVersion(u32),
    /// A lane name was the empty string.
    #[error("lane name must not be empty")]
    EmptyLaneName,
    /// A lane name contained a NUL byte.
    #[error("lane name must not contain NUL bytes")]
    InvalidLaneName,
    /// `passed` exceeded `scanned` for a per-lane digest.
    #[error("lane passed count {passed} exceeds scanned count {scanned}")]
    PassedExceedsScanned {
        /// Number of items the lane reported as passed.
        passed: u32,
        /// Number of items the lane scanned in total.
        scanned: u32,
    },
    /// Receipt `finished_at` was strictly before `started_at`.
    #[error("receipt finished_at {finished_at} is before started_at {started_at}")]
    FinishedBeforeStarted {
        /// Unix-second timestamp at run start.
        started_at: u64,
        /// Unix-second timestamp at run finish.
        finished_at: u64,
    },
    /// Recorded target root was the empty string.
    #[error("receipt target_root must not be empty")]
    TargetRootEmpty,
    /// Recorded target root was not absolute.
    #[error("receipt target_root must be absolute, got {0:?}")]
    TargetRootNonAbsolute(String),
    /// Recorded target root contained a NUL byte.
    #[error("receipt target_root must not contain NUL bytes")]
    TargetRootContainsNul,
}

/// Aggregate for callers that want a single error type across primitives.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreError {
    /// [`Digest`] construction or deserialization failure.
    #[error(transparent)]
    Digest(#[from] DigestError),
    /// [`RuleId`] construction or deserialization failure.
    #[error(transparent)]
    RuleId(#[from] RuleIdError),
    /// [`WorkspacePath`] construction or deserialization failure.
    #[error(transparent)]
    WorkspacePath(#[from] WorkspacePathError),
    /// [`TextRange`] construction or deserialization failure.
    #[error(transparent)]
    TextRange(#[from] TextRangeError),
    /// [`TargetProject`] discovery failure.
    #[error(transparent)]
    TargetProject(#[from] TargetProjectError),
    /// [`crate::QualityReceipt`] construction or deserialization failure.
    #[error(transparent)]
    Receipt(#[from] ReceiptError),
}