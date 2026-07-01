//! cargo fuzz run wrapper for libfuzzer minimization.
//!
//! Rust re-implementation of the bash lane in
//! `titania/scripts/fuzz-minimization.sh`. Run via
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

use std::{fs, io, path::Path};

use thiserror::Error;
use titania_core::TargetProject;
use titania_lanes::{CommandIn, LaneError, LaneExit, current_target_project, exit};

/// Boundary-parsed lane input.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LaneInput {
    DiscoverDefault,
    Explicit { fuzz_target: String, extra_args: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LaneOutcome {
    Child(LaneExit),
    NotApplicable(String),
}

impl LaneOutcome {
    fn to_lane_exit(&self) -> LaneExit {
        match self {
            LaneOutcome::Child(code) => *code,
            LaneOutcome::NotApplicable(_) => LaneExit::NotApplicable,
        }
    }
}

fn parse_lane_input(args: Vec<String>) -> LaneInput {
    let mut iter = args.into_iter();
    let next = iter.next();
    match next {
        None => LaneInput::DiscoverDefault,
        Some(target) => LaneInput::Explicit { fuzz_target: target, extra_args: iter.collect() },
    }
}

fn status_to_lane(code: Option<i32>) -> LaneExit {
    match code {
        Some(0) => LaneExit::Clean,
        Some(2) => LaneExit::Usage,
        Some(1) => LaneExit::Violations,
        _ => LaneExit::Failure,
    }
}

/// Bin-local errors. The structured variants survive end-to-end; `main`
/// matches on them and emits the right `LaneExit` (Fuzz `Io` / `Spawn`
/// become `LaneExit::Failure`, not a string-collapse).
#[derive(Debug, Error)]
enum FuzzError {
    #[error("I/O error reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("failed to spawn cargo fuzz: {0}")]
    Spawn(LaneError),
    #[error("failed to prepare cargo fuzz: {0}")]
    Prepare(LaneError),
}

fn main() -> std::process::ExitCode {
    let input = parse_lane_input(std::env::args().skip(1).collect());
    let target = match current_target_project() {
        Ok(target) => target,
        Err(err) => {
            eprintln!("[fuzz-minimization] cannot resolve target project: {err}");
            return exit(LaneExit::Usage);
        }
    };

    match run_lane(&target, input) {
        Ok(outcome) => {
            let code = outcome.to_lane_exit();
            if let LaneOutcome::NotApplicable(reason) = outcome {
                eprintln!("[fuzz-minimization] NotApplicable: {reason}");
            }
            exit(code)
        }
        Err(err) => {
            eprintln!("[fuzz-minimization] {err}");
            exit(LaneExit::Failure)
        }
    }
}

fn run_lane(target: &TargetProject, input: LaneInput) -> Result<LaneOutcome, FuzzError> {
    match input {
        LaneInput::DiscoverDefault => {
            if has_fuzz_targets(target)? {
                Ok(LaneOutcome::NotApplicable(
                    "fuzz targets exist, but no target name was provided".to_owned(),
                ))
            } else {
                Ok(LaneOutcome::NotApplicable("target project has no fuzz target".to_owned()))
            }
        }
        LaneInput::Explicit { fuzz_target, extra_args } => {
            if has_fuzz_targets(target)? {
                run_fuzz_target(target, &fuzz_target, &extra_args)
            } else {
                Ok(LaneOutcome::NotApplicable("target project has no fuzz target".to_owned()))
            }
        }
    }
}

fn has_fuzz_targets(target: &TargetProject) -> Result<bool, FuzzError> {
    let fuzz_dir = target.as_std_path().join("fuzz");
    if !fuzz_dir.join("Cargo.toml").is_file() {
        return Ok(false);
    }
    let targets_dir = fuzz_dir.join("fuzz_targets");
    let entries = match fs::read_dir(&targets_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(err) => {
            return Err(FuzzError::Io { path: targets_dir.display().to_string(), source: err });
        }
    };
    entries
        .map(|entry| entry.map(|entry| is_rust_source(&entry.path())))
        .try_fold(false, |found, entry| entry.map(|is_target| found || is_target))
        .map_err(|err| FuzzError::Io { path: targets_dir.display().to_string(), source: err })
}

fn is_rust_source(path: &Path) -> bool {
    path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("rs")
}

fn run_fuzz_target(
    target: &TargetProject,
    fuzz_target: &str,
    extra_args: &[String],
) -> Result<LaneOutcome, FuzzError> {
    let mut command = CommandIn::new(target, "cargo").map_err(FuzzError::Prepare)?;
    command.inherit_env();
    command.arg("fuzz").arg("run").arg(fuzz_target);
    command.arg("--target").arg("x86_64-unknown-linux-gnu");
    command.arg("--");
    command.arg("-len_control=1");
    command.arg("-minimize_contribs=1");
    extra_args.iter().for_each(|arg| {
        command.arg(arg.as_str());
    });

    command
        .run_status_raw()
        .map(|status| LaneOutcome::Child(status_to_lane(status.code())))
        .map_err(FuzzError::Spawn)
}

#[cfg(test)]
mod tests {
    use std::io;

    use titania_core::TargetProject;
    use titania_lanes::LaneExit;

    use super::{LaneInput, LaneOutcome, has_fuzz_targets, run_lane};

    fn target_from(path: &std::path::Path) -> io::Result<TargetProject> {
        TargetProject::try_from_path(path).map_err(|e| io::Error::other(e.to_string()))
    }

    fn fixture_target() -> io::Result<(tempfile::TempDir, TargetProject)> {
        let temp = tempfile::TempDir::new()?;
        std::fs::write(temp.path().join("Cargo.toml"), "[workspace]\nmembers=[\"fuzz\"]\n")?;
        let fuzz_dir = temp.path().join("fuzz");
        std::fs::create_dir_all(fuzz_dir.join("fuzz_targets"))?;
        std::fs::write(
            fuzz_dir.join("Cargo.toml"),
            "[package]\nname=\"fuzz\"\nversion=\"0.0.0\"\nedition=\"2024\"\n",
        )?;
        let target = target_from(temp.path())?;
        Ok((temp, target))
    }

    fn empty_target() -> io::Result<(tempfile::TempDir, TargetProject)> {
        let temp = tempfile::TempDir::new()?;
        std::fs::write(temp.path().join("Cargo.toml"), "[workspace]\nmembers=[]\n")?;
        let target = target_from(temp.path())?;
        Ok((temp, target))
    }

    #[test]
    fn project_without_fuzz_targets_emits_not_applicable_disposition() -> io::Result<()> {
        let (_temp, target) = empty_target()?;
        let outcome = run_lane(&target, LaneInput::DiscoverDefault)
            .map_err(|e| io::Error::other(e.to_string()))?;
        assert!(matches!(outcome, LaneOutcome::NotApplicable(_)));
        assert_eq!(outcome.to_lane_exit(), LaneExit::NotApplicable);
        Ok(())
    }

    #[test]
    fn project_with_fuzz_targets_but_no_explicit_target_emits_not_applicable() -> io::Result<()> {
        let (_temp, target) = fixture_target()?;
        // Create a fuzz target so has_fuzz_targets returns true.
        std::fs::write(
            target.as_std_path().join("fuzz/fuzz_targets/empty.rs"),
            "pub fn empty() {}\n",
        )?;
        assert!(has_fuzz_targets(&target).map_err(|e| io::Error::other(e.to_string()))?);
        let outcome = run_lane(&target, LaneInput::DiscoverDefault)
            .map_err(|e| io::Error::other(e.to_string()))?;
        assert!(matches!(outcome, LaneOutcome::NotApplicable(_)));
        Ok(())
    }
}
