//! Typed errors for the domain primitives. One error enum per constructor,
//! using `thiserror` so the messages are stable and machine-consumable.

use crate::{GateScope, Lane};
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
    /// Rule identifier exceeded the maximum allowed length of 96 characters.
    #[error("rule id must not exceed 96 characters; got {0}")]
    TooLong(usize),
}

/// Errors produced by [`crate::proof_id::KaniHarnessId::new`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KaniHarnessIdError {
    /// Kani harness identifier was empty.
    #[error("kani harness id must not be empty")]
    Empty,
    /// Kani harness identifier exceeded [`crate::proof_id::KANI_HARNESS_ID_MAX_LEN`].
    #[error("kani harness id must not exceed {max} characters, got {0}",
        max = crate::proof_id::KANI_HARNESS_ID_MAX_LEN)]
    TooLong(usize),
    /// Kani harness identifier's first byte was not an ASCII letter
    /// (`[a-zA-Z]`). Covers leading digits, leading underscores, leading
    /// non-ASCII bytes, and any other non-letter lead byte.
    #[error("kani harness id must start with an ASCII letter; bad first byte 0x{byte:02x}")]
    LeadingNonLetter {
        /// First byte of the candidate id.
        byte: u8,
    },
    /// Kani harness identifier at offset ≥ 1 contained a byte outside
    /// `[A-Za-z0-9_]`.
    #[error(
        "kani harness id must contain only ASCII letters, digits, and underscores; bad byte 0x{byte:02x} at offset {offset}"
    )]
    NotAscii {
        /// Offending byte.
        byte: u8,
        /// Byte offset within the input (always ≥ 1).
        offset: usize,
    },
}

/// Errors produced when parsing a [`crate::proof_id::MutantOperator`] from
/// its wire-form literal via the `FromStr` implementation (i.e.
/// `let op: MutantOperator = "...".parse()?`).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MutantOperatorError {
    /// Operator literal was outside the recognised closed set.
    #[error("mutant operator {0:?} is not in the recognised operator set")]
    Unknown(String),
}

/// Single violation of the bounded package / path character policy used by
/// [`crate::proof_id::MutantId`].
///
/// Both segments share the same hostile-input rejection rules: no NUL, no
/// ASCII control, no backslash, no `:` (which is a positional separator in
/// the wire form), no Windows drive prefix, no UNC prefix, no `..` traversal
/// component, no leading `/` for path segments.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PathSegmentError {
    /// Segment carried an embedded NUL byte.
    #[error("segment contains NUL byte")]
    ContainsNull,
    /// Segment carried an ASCII control byte (other than NUL).
    #[error("segment contains control byte 0x{0:02x}")]
    ControlByte(u8),
    /// Segment carried a backslash (Windows separator / escape surface).
    #[error("segment contains backslash")]
    ContainsBackslash,
    /// Segment carried a `:`. The canonical wire form fixes `:` as a
    /// positional separator, so any embedded `:` creates ambiguous
    /// path/line/col partitions.
    #[error("segment contains ':' which is a positional separator")]
    ContainsColon,
    /// Segment started with `/`, making it absolute rather than
    /// workspace-relative.
    #[error("segment starts with '/' (absolute)")]
    LeadingSlash,
    /// Segment contained a `..` path-traversal component.
    #[error("segment contains '..' component")]
    ContainsDotDot,
    /// Segment used a Windows drive-absolute prefix such as `C:`. The
    /// `String` carries the original drive prefix as supplied.
    #[error("segment uses Windows drive prefix {0:?}")]
    DriveAbsolute(String),
    /// Segment used a UNC prefix (`\\server\share\...` or its
    /// forward-slash form `//server/share/...`).
    #[error("segment uses UNC path")]
    UncForm,
}

/// Errors produced by [`crate::proof_id::MutantId::new`] and
/// [`crate::proof_id::MutantId::parse`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MutantIdError {
    /// Package name was empty.
    #[error("mutant id package must not be empty")]
    EmptyPackage,
    /// Relative path was empty.
    #[error("mutant id path must not be empty")]
    EmptyPath,
    /// Relative path started with `/`.
    #[error("mutant id path must be workspace-relative, not absolute")]
    PathAbsolute,
    /// Line number was zero (must be 1-based).
    #[error("mutant id line must be >= 1, got 0")]
    LineNotPositive,
    /// Column number was zero (must be 1-based).
    #[error("mutant id column must be >= 1, got 0")]
    ColNotPositive,
    /// Line component of the parsed wire form was not a decimal integer.
    #[error("mutant id wire line component is not a decimal integer")]
    LineNotAnInteger,
    /// Column component of the parsed wire form was not a decimal integer.
    #[error("mutant id wire column component is not a decimal integer")]
    ColNotAnInteger,
    /// Wire form lacked the `::` package separator.
    #[error("mutant id wire form {0:?} is missing the '::' separator")]
    MissingSeparator(String),
    /// Wire form lacked the trailing `:operator` suffix.
    #[error("mutant id wire form {0:?} is missing the ':<operator>' suffix")]
    MissingOperator(String),
    /// Operator literal was outside the recognised closed set.
    #[error("mutant id operator {0:?} is not in the recognised operator set")]
    UnknownOperator(String),
    /// Relative path component contained a `:`. The canonical wire form
    /// disambiguates the four positional segments by counting `:` from
    /// the right, so an embedded `:` creates ambiguous path/line/col
    /// partitions; we reject those forms outright.
    #[error(
        "mutant id wire form {0:?} has ':' inside the path segment; the canonical form reserves ':' as a positional separator"
    )]
    PathContainsColon(String),
    /// Package name exceeded the bounded character budget declared by
    /// [`crate::proof_id::MUTANT_PKG_MAX_LEN`].
    #[error("mutant id package must not exceed {max} characters, got {found}")]
    PackageTooLong {
        /// Length of the rejected package.
        found: usize,
        /// Static upper bound declared by the parser.
        max: usize,
    },
    /// Relative path exceeded the bounded character budget declared by
    /// [`crate::proof_id::MUTANT_PATH_MAX_LEN`].
    #[error("mutant id path must not exceed {max} characters, got {found}")]
    PathTooLong {
        /// Length of the rejected path.
        found: usize,
        /// Static upper bound declared by the parser.
        max: usize,
    },
    /// Package segment violated the bounded character policy.
    #[error("mutant id package is invalid: {0}")]
    PackageInvalid(#[source] PathSegmentError),
    /// Relative path segment violated the bounded character policy.
    #[error("mutant id path is invalid: {0}")]
    PathInvalid(#[source] PathSegmentError),
}

/// Errors produced by [`crate::mutants_baseline::MutantsBaseline::parse_str`].
///
/// The core is filesystem-free — file existence and read errors are
/// classified by the lane (`titania-lanes::MutantsLaneError`) before the
/// lane hands the decoded `&str` to `parse_str`. The variants below are
/// only ever constructed over validated UTF-8 input.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MutantsBaselineError {
    /// Baseline source contained malformed JSON.
    #[error("mutants baseline JSON parse failed at {path}: {reason}")]
    JsonParse {
        /// Source label (typically the path) that failed to parse.
        path: Box<str>,
        /// Underlying `serde_json` error description.
        reason: Box<str>,
    },
    /// Baseline source had an unsupported schema version.
    #[error(
        "mutants baseline schema version {found} at {path} is not supported (expected {expected})"
    )]
    UnsupportedSchemaVersion {
        /// Source label (typically the path) that failed schema check.
        path: Box<str>,
        /// Version observed on disk.
        found: u32,
        /// Version this crate understands.
        expected: u32,
    },
    /// Baseline entry's `accepted_by_rule` did not match the contract family
    /// `mutant-accept/<owner>/<reason>/<expiry>`.
    #[error(
        "mutants baseline entry at {path} has invalid accepted_by_rule {accepted_by_rule:?}: {reason}"
    )]
    InvalidAcceptedByRule {
        /// Source label (typically the path) that failed validation.
        path: Box<str>,
        /// Offending `accepted_by_rule` literal.
        accepted_by_rule: Box<str>,
        /// Reason the rule was rejected.
        reason: Box<str>,
    },
    /// Baseline entry's human-readable `reason` field was empty or
    /// whitespace-only. The `reason` is the audit trail for the bypass and
    /// must carry meaningful content.
    #[error("mutants baseline entry at {path} has invalid reason {reason:?}")]
    InvalidReason {
        /// Source label (typically the path) that failed validation.
        path: Box<str>,
        /// Offending `reason` literal.
        reason: Box<str>,
    },
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
    /// Workspace-relative path used a Windows drive-absolute prefix (e.g.
    /// `C:`, `c:`). The `String` carries the original drive prefix as
    /// supplied, preserving upper- or lowercase.
    #[error("workspace path must not use a Windows drive prefix ({0})")]
    DriveAbsolute(String),
    /// Workspace-relative path used a UNC prefix (`\\server\share\...` or
    /// its forward-slash form `//server/share/...`).
    #[error("workspace path must not use a UNC path")]
    UncForm,
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

/// Errors produced while validating a target project path or discovering its
/// Cargo manifest.
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

/// Errors produced by [`crate::Finding`] construction.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FindingError {
    /// Finding location failed validation.
    #[error(transparent)]
    Location(#[from] LocationError),
}

/// Errors produced by [`crate::ProcessTermination::signaled`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FailureError {
    /// Signal number was outside the supported Unix signal range.
    #[error("signal number must be 1–31, got {0}")]
    InvalidSignal(i32),
}

/// Errors produced when reconstructing a [`crate::LaneOutcome`] from an
/// [`crate::ArtifactOutcome`] read back from disk.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ArtifactError {
    /// The artifact discriminator named a payload field that was absent.
    #[error("artifact outcome variant {variant} is missing field {field}")]
    FieldMissing {
        /// Discriminator value read from disk.
        variant: &'static str,
        /// Expected payload field name that was absent.
        field: &'static str,
    },
}

/// Errors produced by [`crate::CommandEvidence::new`], [`crate::LaneEvidence::new`],
/// and the lane outcome wire deserializers.
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
    /// A wire-form `LaneOutcome::Findings` payload carried zero findings.
    ///
    /// An empty findings list is not a valid findings outcome on the wire —
    /// it would otherwise deserialize into a vacuous pass. The constructor
    /// path is unchanged; this variant is only ever produced by
    /// [`crate::LaneOutcome`]'s `Deserialize` implementation.
    #[error("lane outcome Findings payload must contain at least one finding")]
    EmptyFindings,
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
    /// Pass report contained a lane outcome that is not pass-shaped
    /// (rejecting finding or failed lane execution).
    #[error(
        "pass requires all lane outcomes to be Clean, Skipped, or informational-only Findings; lane {0} has {1}"
    )]
    NonPassLaneOutcome(Lane, String),
    /// `per_lane` lane identities did not match the canonical lane sequence
    /// required by the receipt scope (Pass) or the v1 lane DAG (Reject).
    #[error("per_lane does not match canonical scope ordering for {scope:?}: {error}")]
    PerLaneScopeMismatch {
        /// Scope the constructor was validating against.
        scope: GateScope,
        /// Specific scope-mismatch failure.
        error: PerLaneScopeError,
    },
}

/// Scope-mismatch sub-error for [`ReportError::PerLaneScopeMismatch`].
///
/// The variants cover every shape of per-lane sequence violation:
/// duplicates, missing lanes (in scope but not `per_lane`), extra lanes
/// (in `per_lane` but not scope), and out-of-order lane appearances.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PerLaneScopeError {
    /// A lane appeared more than once in `per_lane`.
    #[error("per_lane contains duplicate lane identity {0}")]
    Duplicate(Lane),
    /// A lane expected by `scope` was missing from `per_lane`.
    #[error("per_lane is missing lane identity {0} required by scope")]
    Missing(Lane),
    /// A lane present in `per_lane` was not part of the canonical scope.
    #[error("per_lane contains lane identity {0} not in canonical scope")]
    Extra(Lane),
    /// A lane appeared at a position inconsistent with canonical ordering.
    #[error("per_lane lane identity {got} appears out of order after {previous}")]
    OutOfOrder {
        /// Lane that previously appeared in the canonical sequence.
        previous: Lane,
        /// Lane that broke the canonical ordering.
        got: Lane,
    },
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
    /// Finding construction failed.
    #[error(transparent)]
    Finding(#[from] FindingError),
    /// Lane failure construction failed.
    #[error(transparent)]
    Failure(#[from] FailureError),
    /// Lane outcome reconstruction from an on-disk artifact failed.
    #[error(transparent)]
    Artifact(#[from] ArtifactError),
    /// Lane outcome construction failed.
    #[error(transparent)]
    Outcome(#[from] OutcomeError),
    /// Report construction failed.
    #[error(transparent)]
    Report(#[from] ReportError),
    /// Kani harness identifier construction failed.
    #[error(transparent)]
    KaniHarnessId(#[from] KaniHarnessIdError),
    /// Mutant identifier construction failed.
    #[error(transparent)]
    MutantId(#[from] MutantIdError),
    /// Mutants baseline load failed.
    #[error(transparent)]
    MutantsBaseline(#[from] MutantsBaselineError),
    /// Kani harness inventory load failed.
    #[error(transparent)]
    KaniInventory(#[from] KaniInventoryError),
    /// Mutants outcomes / per-mutant records load failed.
    #[error(transparent)]
    MutantsOutcomes(#[from] MutantsOutcomesError),
}

/// Errors produced by [`crate::kani_inventory::KaniInventory::parse_str`].
///
/// The core is filesystem-free — file existence and read errors are
/// classified by the lane (`titania-lanes::KaniLaneError`) before the
/// lane hands the decoded `&str` to `parse_str`. The variants below are
/// only ever constructed over validated UTF-8 input.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KaniInventoryError {
    /// Inventory source contained malformed JSON.
    #[error("kani inventory JSON parse failed at {path}: {reason}")]
    JsonParse {
        /// Source label (typically the path) that failed to parse.
        path: Box<str>,
        /// Underlying `serde_json` error description.
        reason: Box<str>,
    },
    /// Inventory contained more harnesses than the static upper bound.
    ///
    /// Bounded validation rejects pathologically large inputs before
    /// they can exhaust the parser's allocation budget; the bound is
    /// generous (one million) so a well-formed inventory never trips it.
    #[error("kani inventory at {path} has {found} harnesses exceeding the static bound of {max}")]
    TooManyHarnesses {
        /// Source label (typically the path) that failed validation.
        path: Box<str>,
        /// Number of harnesses observed after deserialise.
        found: usize,
        /// Static per-file bound declared by the parser.
        max: usize,
    },
}

/// Errors produced by [`crate::mutants_outcomes::MutantsOutcomes::parse_str`]
/// and [`crate::mutants_outcomes::MutantsRecords::parse_str`].
///
/// The core is filesystem-free — file existence and read errors are
/// classified by the lane (`titania-lanes::MutantsLaneError`) before the
/// lane hands the decoded `&str` to `parse_str`. The variants below are
/// only ever constructed over validated UTF-8 input.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MutantsOutcomesError {
    /// Outcomes source contained malformed JSON.
    #[error("mutants outcomes JSON parse failed at {path}: {reason}")]
    OutcomesJsonParse {
        /// Source label (typically the path) that failed to parse.
        path: Box<str>,
        /// Underlying `serde_json` error description.
        reason: Box<str>,
    },
    /// Per-mutant records source contained malformed JSON.
    #[error("mutants records JSON parse failed at {path}: {reason}")]
    RecordsJsonParse {
        /// Source label (typically the path) that failed to parse.
        path: Box<str>,
        /// Underlying `serde_json` error description.
        reason: Box<str>,
    },
    /// Mutant record had no source span (line/column unavailable).
    ///
    /// `MutantId::new` requires positive 1-based line and column; a
    /// record that omits the start point cannot be promoted to a typed
    /// id and is rejected up front rather than coerced.
    #[error("mutant record {mutation_name:?} at {path} has no source span start point")]
    MissingSourceSpan {
        /// Source label (typically the path) that failed validation.
        path: Box<str>,
        /// cargo-mutants record name for the offending entry.
        mutation_name: Box<str>,
    },
    /// Mutant record's `file` did not match the expected
    /// `crates/<package>/...` prefix and is therefore outside the
    /// package directory.
    #[error("mutant record {mutation_name:?} at {path} has file outside its declared package")]
    PathOutsidePackage {
        /// Source label (typically the path) that failed validation.
        path: Box<str>,
        /// cargo-mutants record name for the offending entry. The
        /// `file` and `package` are encoded in the cargo-mutants
        /// record name per the
        /// `crates/<package>/<file>:<line>:<col>: ...` convention so
        /// the diagnostic does not duplicate them.
        mutation_name: Box<str>,
    },
    /// Mutant record's `MutantId::new` rejected the assembled fields.
    #[error("mutant record {mutation_name:?} at {path} has invalid id: {reason}")]
    InvalidMutantId {
        /// Source label (typically the path) that failed validation.
        path: Box<str>,
        /// cargo-mutants record name for the offending entry.
        mutation_name: Box<str>,
        /// Underlying [`crate::MutantIdError`] description.
        reason: Box<str>,
    },
    /// Outcomes document carried more entries than the static upper bound.
    #[error("mutants outcomes at {path} has {found} entries exceeding the static bound of {max}")]
    TooManyOutcomes {
        /// Source label (typically the path) that failed validation.
        path: Box<str>,
        /// Number of outcome entries observed after deserialise.
        found: usize,
        /// Static per-file bound declared by the parser.
        max: usize,
    },
    /// Per-mutant records list carried more entries than the static upper bound.
    #[error("mutants records at {path} has {found} records exceeding the static bound of {max}")]
    TooManyRecords {
        /// Source label (typically the path) that failed validation.
        path: Box<str>,
        /// Number of records observed after deserialise.
        found: usize,
        /// Static per-file bound declared by the parser.
        max: usize,
    },
}
