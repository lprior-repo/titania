//! Typed error enum for the cargo-mutants lane.
//!
//! v1.5 spec §4.2 / §4.3 typed errors the lane can produce. Every
//! variant carries structured fields so the dispatcher can render
//! machine-readable stderr without `String`-flavoured `Result`s.

use std::io;

use thiserror::Error;

use titania_core::RuleIdError;

/// v1.5 spec §4.2 / §4.3 typed errors the lane can produce. Every
/// variant carries structured fields so the dispatcher can render
/// machine-readable stderr without `String`-flavoured `Result`s.
#[derive(Debug, Error)]
pub enum MutantsLaneError {
    /// Baseline file does not exist on disk. The lane never propagates
    /// this to the dispatcher; the orchestrator converts it to a typed
    /// `MUTANT_BASELINE_MISSING` finding so the operator reminder
    /// reaches the receipt directly.
    #[error("mutants baseline file is missing at {0}; run scripts/dev/mutants-bootstrap.sh")]
    BaselineMissing(String),
    /// Baseline file is present but cannot be read.
    #[error("mutants baseline read failed at {path}: {reason}")]
    BaselineRead {
        /// Source label (typically the on-disk path).
        path: Box<str>,
        /// Underlying I/O error description.
        reason: Box<str>,
    },
    /// Baseline parse or schema validation failed. The lane already
    /// types this at [`titania_core::MutantsBaselineError`] so the
    /// reason field here is a flattened string.
    #[error("malformed mutants baseline at {path}: {reason}")]
    BaselineMalformed {
        /// Source label (typically the on-disk path).
        path: Box<str>,
        /// Underlying parse error description.
        reason: Box<str>,
    },
    /// `RuleId::new` rejected the rule-id literal — a doctrine-class
    /// bug since both rule ids are catalog constants. Surfaced as
    /// `LaneFailure::Infra`.
    #[error("mutants lane could not construct a rule id: {0}")]
    RuleId(#[source] RuleIdError),
    /// cargo-mutants major version is older than the v1.5 spec floor of
    /// `25.x`.
    #[error(
        "cargo-mutants version {found}.x is older than the v1.5 spec floor {floor}.x (per spec §7)"
    )]
    CargoMutantsTooOld {
        /// Major version reported by `cargo mutants --version`.
        found: u32,
        /// Floor major version (v1.5 spec §7).
        floor: u32,
    },
    /// `Command::spawn` returned an I/O error. The cause is
    /// heap-boxed as a string so the variant stays under the
    /// workspace `large-error-threshold = 64` bound; the typed source
    /// chain is preserved at the dispatch layer via the
    /// `io::Error::to_string()` representation.
    #[error("cargo mutants spawn failed: {0}")]
    Spawn(Box<str>),
    /// `Child::wait_timeout` returned an I/O error.
    #[error("cargo mutants wait failed: {0}")]
    Wait(Box<str>),
    /// Workspace-level invocation exceeded the wallclock cap.
    #[error("cargo mutants timed out after {timeout_secs}s")]
    TimedOut {
        /// Configured wallclock cap.
        timeout_secs: u64,
    },
    /// `Child::kill` after a timeout returned an I/O error.
    #[error("kill of timed-out cargo mutants child failed: {0}")]
    KillAfterTimeout(Box<str>),
    /// `Child::wait` after a kill returned an I/O error.
    #[error("reap of killed cargo mutants child failed: {0}")]
    ReapAfterTimeout(Box<str>),
    /// `std::fs::remove_dir_all` on `<output>` failed.
    #[error("could not remove mutants output directory {path}: {kind:?}")]
    OutputDirRemove {
        /// Path that could not be removed.
        path: String,
        /// Stable [`io::ErrorKind`] reported by the filesystem.
        kind: io::ErrorKind,
    },
    /// Could not read or enumerate the cargo-mutants artifact directory.
    #[error("could not read cargo-mutants artifact directory: {0}")]
    ArtifactDir(String),
    /// Neither direct nor nested `outcomes.json` is present.
    #[error("cargo-mutants did not produce outcomes.json below {0}")]
    ArtifactMissing(String),
    /// `outcomes.json` failed JSON parsing.
    #[error("outcomes.json parse failed at {path}: {reason}")]
    OutcomesParse {
        /// Path label.
        path: Box<str>,
        /// Underlying serde error description.
        reason: Box<str>,
    },
    /// `mutants.json` failed JSON parsing.
    #[error("mutants.json parse failed at {path}: {reason}")]
    MutantsParse {
        /// Path label.
        path: Box<str>,
        /// Underlying serde error description.
        reason: Box<str>,
    },
    /// A `MissedMutant` outcome is not present in `mutants.json`.
    #[error("survivor `{0}` is absent from mutants.json")]
    SurvivorAbsent(String),
    /// A `BinaryOperator` / `UnaryOperator` genre carries a textual
    /// name the v1.5 closed operator set does not recognise. The lane
    /// fails closed instead of coercing to `ArithmeticOpFlip`.
    ///
    /// Uses `Box<str>` (16 bytes) instead of `String` (24 bytes) so
    /// the 3-field variant stays under the
    /// `large-error-threshold = 64` clippy policy.
    #[error("mutant `{name}` carries an unknown operator (genre `{genre}`); raw `{raw}`")]
    UnknownOperator {
        /// Human-readable cargo-mutants name.
        name: Box<str>,
        /// cargo-mutants genre tag.
        genre: Box<str>,
        /// Underlying textual name we failed to classify.
        raw: Box<str>,
    },
    /// The mutant record had no `span.start` line/column pair.
    #[error("mutant `{name}` has no source span")]
    SpanMissing {
        /// Human-readable cargo-mutants name.
        name: Box<str>,
    },
    /// `MutantId::new` rejected the assembled package/path/line/col.
    #[error("mutant `{name}` has invalid id: {reason}")]
    MutantIdInvalid {
        /// Human-readable cargo-mutants name.
        name: Box<str>,
        /// Underlying `MutantIdError` description.
        reason: Box<str>,
    },
    /// The mutant's declared file does not belong to the declared package.
    #[error("mutant `{name}` reports file `{file}` outside package `{package}`")]
    PathOutsidePackage {
        /// Human-readable cargo-mutants name.
        name: Box<str>,
        /// Offending file literal.
        file: Box<str>,
        /// Package the record claims ownership of.
        package: Box<str>,
    },
    /// The survivor's package does not match the workspace package that
    /// was invoked. The workspace-level run never crosses packages, so
    /// this should be unreachable today; we keep the variant for
    /// forward-compat against cargo-mutants 28 output shapes.
    ///
    /// Uses `Box<str>` so the 3-field variant stays under the
    /// `large-error-threshold = 64` clippy policy.
    #[error("survivor `{mutation_id}` reports package `{found}`, expected `{expected}`")]
    PackageMismatch {
        /// cargo-mutants mutation name.
        mutation_id: Box<str>,
        /// Expected package (resolved from the workspace).
        expected: Box<str>,
        /// Actual package reported by the survivor record.
        found: Box<str>,
    },
    /// cargo-mutants aggregate `missed` count disagrees with the number
    /// of `MissedMutant` entries it actually wrote.
    #[error("outcomes.json reports missed count disagrees with bodies: {0}")]
    MissedCountMismatch(String),
    /// A `MissedMutant` outcome has no `scenario.Mutant.name`.
    #[error("a missed mutant outcome has no scenario.Mutant.name")]
    MissingSurvivorName,
    /// cargo-mutants exited non-zero without an `outcomes.json`.
    #[error("cargo mutants exited with code {0}")]
    CargoMutantsExit(i32),
}

#[cfg(test)]
mod tests {
    /// Compile-time guard that keeps the lane-error enum under the
    /// workspace `large-error-threshold = 64` byte bound (see
    /// `clippy.toml`). If this ever fires, re-box one of the inner
    /// fields rather than relaxing the threshold (matches the
    /// Kani lane's `assert_kani_lane_error_under_threshold` guard).
    #[test]
    fn assert_mutants_lane_error_under_threshold() {
        const SIZE: usize = std::mem::size_of::<super::MutantsLaneError>();
        assert!(SIZE < 64, "MutantsLaneError is {SIZE} bytes (threshold = 64)");
    }
}
