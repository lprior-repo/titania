//! Optional `lake build` runner for `proofs/lean/`.
//!
//! Rust re-implementation of the bash lane in
//! `velvet-ballistics/scripts/verify-lean.sh`. Run via
//! `cargo run --bin verify-lean --` from the repository root or via the
//! matching Moon task in `.moon/tasks/all.yml`.
//!
//! ## Behavior parity
//! Mirrors the bash's three-tier decision tree:
//! 1. If the proof directory is missing OR has no `lakefile.lean` /
//!    `lakefile.toml`, exit 0 unless `LEAN_REQUIRED=1` (then exit 1).
//! 2. If `lake` is not on `PATH`, fail with a clear message (the bash
//!    version exits 1 unconditionally; we surface `LaneExit::Failure`).
//! 3. Otherwise `cd` into the proof directory and run `lake build`,
//!    propagating the child's exit code.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

use titania_lanes::{CommandIn, LaneExit, current_target_project, exit};

/// Environment variable that forces the lane to fail if Lean artifacts
/// are missing (mirrors the bash's `LEAN_REQUIRED` contract).
const LEAN_REQUIRED_ENV: &str = "LEAN_REQUIRED";
/// Optional override for the proof directory (matches `LEAN_PROOF_DIR`).
const LEAN_PROOF_DIR_ENV: &str = "LEAN_PROOF_DIR";

fn main() -> std::process::ExitCode {
    let target = match current_target_project() {
        Ok(target) => target,
        Err(err) => {
            return exit_after_stderr_line(
                format_args!("[verify:lean] cannot resolve target project: {err}"),
                LaneExit::Usage,
            );
        }
    };
    let proof_dir = resolve_proof_dir(&target);

    if !proof_dir.is_dir() {
        return handle_missing_dir(&proof_dir);
    }
    if !has_lakefile(&proof_dir) {
        return handle_missing_lakefile(&proof_dir);
    }
    if !lake_on_path(&target) {
        return exit_after_stderr_line(
            format_args!("lake is required for Lean proof verification but is unavailable."),
            LaneExit::Failure,
        );
    }

    if write_stderr_line(format_args!("[verify:lean] lake build in {}", proof_dir.display()))
        .is_err()
    {
        return exit(LaneExit::Failure);
    }
    run_lake_build(&target, &proof_dir)
}

fn resolve_proof_dir(target: &titania_core::TargetProject) -> PathBuf {
    let Ok(dir) = std::env::var(LEAN_PROOF_DIR_ENV) else {
        return target.as_std_path().join("proofs/lean");
    };
    let path = PathBuf::from(dir);
    if path.is_absolute() { path } else { target.as_std_path().join(path) }
}

fn handle_missing_dir(proof_dir: &Path) -> std::process::ExitCode {
    if is_lean_required() {
        return exit_after_stderr_line(
            format_args!("Lean proof directory is required but missing: {}", proof_dir.display()),
            LaneExit::Failure,
        );
    }
    exit_after_stderr_line(
        format_args!(
            "[verify:lean] no Lean proof directory found at {}; skipped",
            proof_dir.display()
        ),
        LaneExit::Clean,
    )
}

fn handle_missing_lakefile(proof_dir: &Path) -> std::process::ExitCode {
    if is_lean_required() {
        return exit_after_stderr_line(
            format_args!(
                "Lean proof directory exists but has no lakefile: {}",
                proof_dir.display()
            ),
            LaneExit::Failure,
        );
    }
    exit_after_stderr_line(
        format_args!("[verify:lean] no lakefile found in {}; skipped", proof_dir.display()),
        LaneExit::Clean,
    )
}

fn has_lakefile(proof_dir: &Path) -> bool {
    proof_dir.join("lakefile.lean").is_file() || proof_dir.join("lakefile.toml").is_file()
}

fn lake_on_path(target: &titania_core::TargetProject) -> bool {
    // `which`-equivalent: try to spawn `lake --version`. The child writes
    // to captured buffers; we only care about the spawn outcome.
    let Ok(mut command) = CommandIn::new(target, "lake") else { return false };
    let _ = command.inherit_env().arg("--version");
    command.run_capture_raw().is_ok_and(|output| output.status().success())
}

fn is_lean_required() -> bool {
    std::env::var(LEAN_REQUIRED_ENV).ok().as_deref() == Some("1")
}

fn run_lake_build(
    target: &titania_core::TargetProject,
    proof_dir: &Path,
) -> std::process::ExitCode {
    let proof_dir_arg = proof_dir.display().to_string();
    let mut command = match CommandIn::new(target, "lake") {
        Ok(command) => command,
        Err(err) => {
            return exit_after_stderr_line(
                format_args!("[verify:lean] failed to prepare lake: {err}"),
                LaneExit::Failure,
            );
        }
    };
    let _ = command.inherit_env().arg("-d").arg(&proof_dir_arg).arg("build");
    match command.run_status_raw() {
        Ok(status) => match status.code() {
            Some(0) => exit(LaneExit::Clean),
            Some(2) => exit(LaneExit::Usage),
            Some(1) => exit(LaneExit::Violations),
            Some(_) | None => exit(LaneExit::Failure),
        },
        Err(io_err) => exit_after_stderr_line(
            format_args!("[verify:lean] failed to spawn lake: {io_err}"),
            LaneExit::Failure,
        ),
    }
}

/// Write formatted text to stderr followed by a newline.
///
/// # Errors
///
/// Returns an [`io::Error`] when stderr cannot be written.
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}

fn exit_after_stderr_line(args: std::fmt::Arguments<'_>, code: LaneExit) -> std::process::ExitCode {
    match write_stderr_line(args) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}
