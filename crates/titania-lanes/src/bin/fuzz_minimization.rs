//! cargo fuzz run wrapper for libfuzzer minimization.
//!
//! Rust re-implementation of the bash lane `scripts/fuzz-minimization.sh`. Run via
//! `cargo run --bin fuzz-minimization -- <target> [extra args...]` from the
//! repository root or via the matching Moon task in `.moon/tasks/all.yml`.
//!
//! ## Behavior parity
//! The bash wrapper exists because `cargo-fuzz`'s TOML schema cannot
//! simultaneously accept `[package.metadata] cargo-fuzz = true` and
//! `[package.metadata.cargo-fuzz] sancov_timeout = 60`. We therefore pass
//! libfuzzer minimization options on the command line:
//!
//! ```text
//! cargo fuzz run <target> \
//!     --target x86_64-unknown-linux-gnu \
//!     -- \
//!     -len_control=1 \
//!     -minimize_contribs=1 \
//!     <extra args...>
//! ```
//!
//! The Rust wrapper spawns `cargo fuzz run` and propagates the child exit
//! code. If `cargo` itself fails to launch (missing binary, etc.) we map
//! the I/O error to `LaneExit::Failure` so the lane surfaces a clear CI
//! error rather than silently exiting 0.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

use thiserror::Error;
use titania_lanes::{CommandIn, LaneError, LaneExit, current_target_project, exit};

/// Boundary-parsed lane input.
enum LaneInput {
    DiscoverDefault,
    Explicit { fuzz_target: String, extra_args: Vec<String> },
}

enum LaneOutcome {
    NotApplicable(String),
    Child(LaneExit),
}

#[derive(Debug, Error)]
enum FuzzMinError {
    #[error("failed to read {}: {source}", path.display())]
    ReadDir { path: PathBuf, source: io::Error },
    #[error("failed to inspect {}: {source}", path.display())]
    InspectDir { path: PathBuf, source: io::Error },
    #[error("failed to prepare cargo fuzz: {source}")]
    PrepareCargo { source: LaneError },
    #[error("failed to spawn cargo fuzz: {source}")]
    RunCargo { source: LaneError },
}

impl LaneOutcome {
    #[must_use]
    const fn to_lane_exit(&self) -> LaneExit {
        match self {
            Self::NotApplicable(_) => LaneExit::NotApplicable,
            Self::Child(code) => *code,
        }
    }
}

fn parse_lane_input(args: &[String]) -> LaneInput {
    match args.split_first() {
        Some((target, extra)) if !target.is_empty() => {
            LaneInput::Explicit { fuzz_target: target.clone(), extra_args: extra.to_vec() }
        }
        _ => LaneInput::DiscoverDefault,
    }
}

const fn status_to_lane(code: Option<i32>) -> LaneExit {
    match code {
        Some(0) => LaneExit::Clean,
        Some(2) => LaneExit::Usage,
        Some(1) => LaneExit::Violations,
        Some(_) | None => LaneExit::Failure,
    }
}

fn main() -> std::process::ExitCode {
    let input = {
        let args: Vec<String> = std::env::args().skip(1).collect();
        parse_lane_input(&args)
    };
    let target = match current_target_project() {
        Ok(target) => target,
        Err(err) => {
            return exit_after_stderr_line(
                &format!("[fuzz-minimization] cannot resolve target project: {err}"),
                LaneExit::Usage,
            );
        }
    };

    match run_lane(&target, input) {
        Ok(outcome) => outcome_exit(outcome),
        Err(err) => {
            exit_after_stderr_line(&format!("[fuzz-minimization] {err}"), LaneExit::Failure)
        }
    }
}

fn outcome_exit(outcome: LaneOutcome) -> std::process::ExitCode {
    let code = outcome.to_lane_exit();
    match outcome {
        LaneOutcome::NotApplicable(reason) => {
            exit_after_stderr_line(&format!("[fuzz-minimization] NotApplicable: {reason}"), code)
        }
        LaneOutcome::Child(_) => exit(code),
    }
}

/// Write one line to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr_line(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())?;
    stderr.write_all(b"\n")
}

fn exit_after_stderr_line(text: &str, code: LaneExit) -> std::process::ExitCode {
    match write_stderr_line(text) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}

/// Run fuzz-minimization command selection for a target project.
///
/// # Errors
///
/// Returns an error string when fuzz-target discovery fails or cargo-fuzz
/// cannot be spawned.
fn run_lane(
    target: &titania_core::TargetProject,
    input: LaneInput,
) -> Result<LaneOutcome, FuzzMinError> {
    let has_targets = has_fuzz_targets(target)?;
    if !has_targets {
        return Ok(LaneOutcome::NotApplicable("target project has no fuzz target".to_owned()));
    }
    match input {
        LaneInput::DiscoverDefault => Ok(LaneOutcome::NotApplicable(
            "fuzz targets exist, but no target name was provided".to_owned(),
        )),
        LaneInput::Explicit { fuzz_target, extra_args } => {
            run_fuzz_target(target, &fuzz_target, &extra_args)
        }
    }
}

/// Detect whether the target project has cargo-fuzz target sources.
///
/// # Errors
///
/// Returns an error string when the fuzz target directory exists but cannot be
/// read or inspected.
fn has_fuzz_targets(target: &titania_core::TargetProject) -> Result<bool, FuzzMinError> {
    let fuzz_dir = target.as_std_path().join("fuzz");
    if !fuzz_dir.join("Cargo.toml").is_file() {
        return Ok(false);
    }
    let targets_dir = fuzz_dir.join("fuzz_targets");
    let entries = match fs::read_dir(&targets_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(err) => {
            return Err(FuzzMinError::ReadDir { path: targets_dir, source: err });
        }
    };
    entries
        .map(|entry| entry.map(|entry| is_rust_source(&entry.path())))
        .try_fold(false, |found, entry| entry.map(|is_target| found || is_target))
        .map_err(|source| FuzzMinError::InspectDir { path: targets_dir, source })
}

fn is_rust_source(path: &Path) -> bool {
    path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("rs")
}

/// Run cargo-fuzz minimization for a named target.
///
/// # Errors
///
/// Returns an error string when command construction or process spawning fails.
fn run_fuzz_target(
    target: &titania_core::TargetProject,
    fuzz_target: &str,
    extra_args: &[String],
) -> Result<LaneOutcome, FuzzMinError> {
    let mut command =
        CommandIn::new(target, "cargo").map_err(|source| FuzzMinError::PrepareCargo { source })?;
    let _ = command.inherit_env();
    let _ = command.arg("fuzz").arg("run").arg(fuzz_target);
    let _ = command.arg("--target").arg("x86_64-unknown-linux-gnu");
    let _ = command.arg("--");
    let _ = command.arg("-len_control=1");
    let _ = command.arg("-minimize_contribs=1");
    let _ = command.args_strings(extra_args);

    command
        .run_status_raw()
        .map(|status| LaneOutcome::Child(status_to_lane(status.code())))
        .map_err(|source| FuzzMinError::RunCargo { source })
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, process::ExitCode};

    use super::{LaneInput, LaneOutcome, run_lane};
    use titania_core::TargetProject;
    use titania_lanes::LaneExit;

    #[test]
    fn project_without_fuzz_targets_emits_not_applicable_disposition() -> ExitCode {
        let Ok(temp) = tempfile::tempdir() else {
            return ExitCode::FAILURE;
        };
        if fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .is_err()
        {
            return ExitCode::FAILURE;
        }
        let Ok(target) = target_project(temp.path()) else {
            return ExitCode::FAILURE;
        };

        let Ok(outcome) = run_lane(&target, LaneInput::DiscoverDefault) else {
            return ExitCode::FAILURE;
        };

        if matches!(outcome, LaneOutcome::NotApplicable(_))
            && outcome.to_lane_exit() == LaneExit::NotApplicable
        {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }

    fn target_project(path: &Path) -> Result<TargetProject, titania_core::TargetProjectError> {
        TargetProject::try_from_path(path)
    }
}
