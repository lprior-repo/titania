use std::io;

use thiserror::Error;

use titania_core::RuleIdError;

#[derive(Debug, Error)]
pub enum MutantsLaneError {
    #[error("mutants baseline file is missing at {0}; run scripts/dev/mutants-bootstrap.sh")]
    BaselineMissing(String),
    #[error("mutants baseline read failed at {path}: {reason}")]
    BaselineRead { path: Box<str>, reason: Box<str> },
    #[error("malformed mutants baseline at {path}: {reason}")]
    BaselineMalformed { path: Box<str>, reason: Box<str> },
    #[error("mutants lane could not construct a rule id: {0}")]
    RuleId(#[source] RuleIdError),
    #[error(
        "cargo-mutants version {found}.x is older than the v1.5 spec floor {floor}.x (per spec §7)"
    )]
    CargoMutantsTooOld { found: u32, floor: u32 },
    #[error("cargo mutants spawn failed: {0}")]
    Spawn(Box<str>),
    #[error("cargo mutants wait failed: {0}")]
    Wait(Box<str>),
    #[error("cargo mutants timed out after {timeout_secs}s")]
    TimedOut { timeout_secs: u64 },
    #[error("kill of timed-out cargo mutants child failed: {0}")]
    KillAfterTimeout(Box<str>),
    #[error("reap of killed cargo mutants child failed: {0}")]
    ReapAfterTimeout(Box<str>),
    #[error("could not remove mutants output directory {path}: {kind:?}")]
    OutputDirRemove { path: String, kind: io::ErrorKind },
    #[error("could not read cargo-mutants artifact directory: {0}")]
    ArtifactDir(String),
    #[error("cargo-mutants did not produce outcomes.json below {0}")]
    ArtifactMissing(String),
    #[error("outcomes.json parse failed at {path}: {reason}")]
    OutcomesParse { path: Box<str>, reason: Box<str> },
    #[error("mutants.json parse failed at {path}: {reason}")]
    MutantsParse { path: Box<str>, reason: Box<str> },
    #[error("survivor `{0}` is absent from mutants.json")]
    SurvivorAbsent(String),
    #[error("mutant `{name}` carries an unknown operator (genre `{genre}`); raw `{raw}`")]
    UnknownOperator { name: Box<str>, genre: Box<str>, raw: Box<str> },
    #[error("mutant `{name}` has no source span")]
    SpanMissing { name: Box<str> },
    #[error("mutant `{name}` has invalid id: {reason}")]
    MutantIdInvalid { name: Box<str>, reason: Box<str> },
    #[error("mutant `{name}` reports file `{file}` outside package `{package}`")]
    PathOutsidePackage { name: Box<str>, file: Box<str>, package: Box<str> },
    #[error("survivor `{mutation_id}` reports package `{found}`, expected `{expected}`")]
    PackageMismatch { mutation_id: Box<str>, expected: Box<str>, found: Box<str> },
    #[error("outcomes.json reports missed count disagrees with bodies: {0}")]
    MissedCountMismatch(String),
    #[error("a missed mutant outcome has no scenario.Mutant.name")]
    MissingSurvivorName,
    #[error("cargo mutants exited with code {0}")]
    CargoMutantsExit(i32),
}

#[cfg(test)]
mod tests {
    #[test]
    fn assert_mutants_lane_error_under_threshold() {
        const SIZE: usize = std::mem::size_of::<super::MutantsLaneError>();
        assert!(SIZE < 64, "MutantsLaneError is {SIZE} bytes (threshold = 64)");
    }
}
