use crate::{GateScope, Lane};
use std::io;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DigestError {
    #[error("digest must be exactly 64 characters, got {0}")]
    WrongLength(usize),
    #[error("digest must contain only lowercase hex characters; bad position {0}")]
    NonHexChar(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RuleIdError {
    #[error("rule id must not be empty")]
    Empty,
    #[error("rule id must contain at least one underscore")]
    NoUnderscore,
    #[error("rule id must be uppercase ASCII; bad character {0:?} at byte {1}")]
    NotUppercase(char, usize),
    #[error("rule id must not exceed 96 characters; got {0}")]
    TooLong(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KaniHarnessIdError {
    #[error("kani harness id must not be empty")]
    Empty,
    #[error("kani harness id must not exceed {max} characters, got {0}",
        max = crate::proof_id::KANI_HARNESS_ID_MAX_LEN)]
    TooLong(usize),
    #[error("kani harness id must start with an ASCII letter; bad first byte 0x{byte:02x}")]
    LeadingNonLetter { byte: u8 },
    #[error(
        "kani harness id must contain only ASCII letters, digits, and underscores; bad byte 0x{byte:02x} at offset {offset}"
    )]
    NotAscii { byte: u8, offset: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MutantOperatorError {
    #[error("mutant operator {0:?} is not in the recognised operator set")]
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PathSegmentError {
    #[error("segment contains NUL byte")]
    ContainsNull,
    #[error("segment contains control byte 0x{0:02x}")]
    ControlByte(u8),
    #[error("segment contains backslash")]
    ContainsBackslash,
    #[error("segment contains ':' which is a positional separator")]
    ContainsColon,
    #[error("segment starts with '/' (absolute)")]
    LeadingSlash,
    #[error("segment contains '..' component")]
    ContainsDotDot,
    #[error("segment uses Windows drive prefix {0:?}")]
    DriveAbsolute(String),
    #[error("segment uses UNC path")]
    UncForm,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MutantIdError {
    #[error("mutant id package must not be empty")]
    EmptyPackage,
    #[error("mutant id path must not be empty")]
    EmptyPath,
    #[error("mutant id path must be workspace-relative, not absolute")]
    PathAbsolute,
    #[error("mutant id line must be >= 1, got 0")]
    LineNotPositive,
    #[error("mutant id column must be >= 1, got 0")]
    ColNotPositive,
    #[error("mutant id wire line component is not a decimal integer")]
    LineNotAnInteger,
    #[error("mutant id wire column component is not a decimal integer")]
    ColNotAnInteger,
    #[error("mutant id wire form {0:?} is missing the '::' separator")]
    MissingSeparator(String),
    #[error("mutant id wire form {0:?} is missing the ':<operator>' suffix")]
    MissingOperator(String),
    #[error("mutant id operator {0:?} is not in the recognised operator set")]
    UnknownOperator(String),
    #[error(
        "mutant id wire form {0:?} has ':' inside the path segment; the canonical form reserves ':' as a positional separator"
    )]
    PathContainsColon(String),
    #[error("mutant id package must not exceed {max} characters, got {found}")]
    PackageTooLong { found: usize, max: usize },
    #[error("mutant id path must not exceed {max} characters, got {found}")]
    PathTooLong { found: usize, max: usize },
    #[error("mutant id package is invalid: {0}")]
    PackageInvalid(#[source] PathSegmentError),
    #[error("mutant id path is invalid: {0}")]
    PathInvalid(#[source] PathSegmentError),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MutantsBaselineError {
    #[error("mutants baseline JSON parse failed at {path}: {reason}")]
    JsonParse { path: Box<str>, reason: Box<str> },
    #[error(
        "mutants baseline schema version {found} at {path} is not supported (expected {expected})"
    )]
    UnsupportedSchemaVersion { path: Box<str>, found: u32, expected: u32 },
    #[error(
        "mutants baseline entry at {path} has invalid accepted_by_rule {accepted_by_rule:?}: {reason}"
    )]
    InvalidAcceptedByRule { path: Box<str>, accepted_by_rule: Box<str>, reason: Box<str> },
    #[error("mutants baseline entry at {path} has invalid reason {reason:?}")]
    InvalidReason { path: Box<str>, reason: Box<str> },
}

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
    #[error("workspace path must not use a Windows drive prefix ({0})")]
    DriveAbsolute(String),
    #[error("workspace path must not use a UNC path")]
    UncForm,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TextRangeError {
    #[error("text range end ({end}) must be >= start ({start})")]
    EndBeforeStart { start: u32, end: u32 },
}

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
    #[error("quality receipt must include at least one lane receipt")]
    EmptyLaneReceiptList,
    #[error("receipt target_root must not be empty")]
    TargetRootEmpty,
    #[error("receipt target_root must be absolute, got {0:?}")]
    TargetRootNonAbsolute(String),
    #[error("receipt target_root must not contain NUL bytes")]
    TargetRootContainsNul,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LaneError {
    #[error("unknown lane: {0}")]
    UnknownLane(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GateScopeError {
    #[error("unknown scope: {0}")]
    UnknownScope(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LocationError {
    #[error("line_start must be >= 1")]
    LineStartBeforeOne,
    #[error("line_end ({line_end}) must be >= line_start ({line_start})")]
    EndBeforeStart { line_start: u32, line_end: u32 },
    #[error("col_end ({col_end}) must be >= col_start ({col_start})")]
    ColEndBeforeStart { col_start: u32, col_end: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FindingError {
    #[error(transparent)]
    Location(#[from] LocationError),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FailureError {
    #[error("signal number must be 1–31, got {0}")]
    InvalidSignal(i32),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ArtifactError {
    #[error("artifact outcome variant {variant} is missing field {field}")]
    FieldMissing { variant: &'static str, field: &'static str },
}

const ARGV0_DISPLAY_BUDGET_BYTES: usize = 200;
const ARGV0_DISPLAY_BODY_BYTES: usize = ARGV0_DISPLAY_BUDGET_BYTES.saturating_sub(3);

fn argv0_display_piece(value: &str) -> String {
    let clean: String = value.chars().map(|ch| if ch.is_control() { '?' } else { ch }).collect();
    let head: String = clean
        .char_indices()
        .filter(|(index, ch)| index.saturating_add(ch.len_utf8()) <= ARGV0_DISPLAY_BODY_BYTES)
        .map(|(_, ch)| ch)
        .collect();
    if clean.len() > ARGV0_DISPLAY_BODY_BYTES { format!("{head}…") } else { head }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OutcomeError {
    #[error("argv must not be empty")]
    EmptyArgv,
    #[error(
        "argv[0] ({}) must equal executable ({})",
        argv0_display_piece(found),
        argv0_display_piece(expected)
    )]
    Argv0Mismatch { expected: String, found: String },
    #[error("exit status must be Exited(0) for Clean lanes")]
    NonZeroExit,
    #[error("lane outcome Findings payload must contain at least one finding")]
    EmptyFindings,
}
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReportError {
    #[error("reject must have at least one non-empty collection")]
    EmptyReject,
    #[error("pass must have at least one lane outcome")]
    EmptyPerLane,
    #[error(
        "pass requires all lane outcomes to be Clean, Skipped, or informational-only Findings; lane {0} has {1}"
    )]
    NonPassLaneOutcome(Lane, String),
    #[error("per_lane does not match canonical scope ordering for {scope:?}: {error}")]
    PerLaneScopeMismatch { scope: GateScope, error: PerLaneScopeError },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PerLaneScopeError {
    #[error("per_lane contains duplicate lane identity {0}")]
    Duplicate(Lane),
    #[error("per_lane is missing lane identity {0} required by scope")]
    Missing(Lane),
    #[error("per_lane contains lane identity {0} not in canonical scope")]
    Extra(Lane),
    #[error("per_lane lane identity {got} appears out of order after {previous}")]
    OutOfOrder { previous: Lane, got: Lane },
}
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
    Finding(#[from] FindingError),
    #[error(transparent)]
    Failure(#[from] FailureError),
    #[error(transparent)]
    Artifact(#[from] ArtifactError),
    #[error(transparent)]
    Outcome(#[from] OutcomeError),
    #[error(transparent)]
    Report(#[from] ReportError),
    #[error(transparent)]
    KaniHarnessId(#[from] KaniHarnessIdError),
    #[error(transparent)]
    MutantId(#[from] MutantIdError),
    #[error(transparent)]
    MutantsBaseline(#[from] MutantsBaselineError),
    #[error(transparent)]
    KaniInventory(#[from] KaniInventoryError),
    #[error(transparent)]
    MutantsOutcomes(#[from] MutantsOutcomesError),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KaniInventoryError {
    #[error("kani inventory JSON parse failed at {path}: {reason}")]
    JsonParse { path: Box<str>, reason: Box<str> },
    #[error("kani inventory at {path} has {found} harnesses exceeding the static bound of {max}")]
    TooManyHarnesses { path: Box<str>, found: usize, max: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MutantsOutcomesError {
    #[error("mutants outcomes JSON parse failed at {path}: {reason}")]
    OutcomesJsonParse { path: Box<str>, reason: Box<str> },
    #[error("mutants records JSON parse failed at {path}: {reason}")]
    RecordsJsonParse { path: Box<str>, reason: Box<str> },
    #[error("mutant record {mutation_name:?} at {path} has no source span start point")]
    MissingSourceSpan { path: Box<str>, mutation_name: Box<str> },
    #[error("mutant record {mutation_name:?} at {path} has file outside its declared package")]
    PathOutsidePackage { path: Box<str>, mutation_name: Box<str> },
    #[error("mutant record {mutation_name:?} at {path} has invalid id: {reason}")]
    InvalidMutantId { path: Box<str>, mutation_name: Box<str>, reason: Box<str> },
    #[error("mutants outcomes at {path} has {found} entries exceeding the static bound of {max}")]
    TooManyOutcomes { path: Box<str>, found: usize, max: usize },
    #[error("mutants records at {path} has {found} records exceeding the static bound of {max}")]
    TooManyRecords { path: Box<str>, found: usize, max: usize },
}
