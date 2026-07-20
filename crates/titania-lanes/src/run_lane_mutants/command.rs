//! Cargo-mutants process invocation: cgroup wrapper and bare fallback.
//!
//! The lane runs the workspace-level full test-mode command exactly
//! once. The shape is locked by the v1.5 spec §4.3 / §7:
//!
//! - `systemd-run --user --scope -p MemoryHigh=20G -p MemoryMax=24G
//!   -p MemorySwapMax=0 -- cargo mutants --no-shuffle --output <dir>
//!   --workspace` when the host provides `systemd-run`;
//! - `cargo mutants --no-shuffle --output <dir> --workspace` otherwise.
//!
//! Both the [`Command`] builder and the public argv materialisation
//! live here so the receipt auditor can re-derive the run from
//! `LaneEvidence::command` without ambiguity.

use std::{
    io,
    path::Path,
    process::{Command, ExitStatus, Stdio},
    time::Duration,
};

use wait_timeout::ChildExt;

use super::{
    constants::{
        CGROUP_MEMORY_HIGH, CGROUP_MEMORY_MAX, MUTANTS_OUTPUT_DIR, MUTANTS_WALLCLOCK_TIMEOUT_SECS,
    },
    error::MutantsLaneError,
};

/// Build the [`Command`] used for the workspace-level cargo-mutants run.
///
/// The shape is locked by the v1.5 spec §4.3 / §7: a single
/// `cargo mutants --no-shuffle --output <dir> --workspace` invocation,
/// optionally wrapped in `systemd-run --user --scope -p MemoryHigh=20G
/// -p MemoryMax=24G -p MemorySwapMax=0 --` when the host provides it.
pub(super) fn build_cargo_mutants_command(
    workspace_root: &Path,
    cgroup_available: bool,
) -> Command {
    if cgroup_available {
        build_cgroup_command(workspace_root)
    } else {
        build_bare_command(workspace_root)
    }
}

/// Build a `systemd-run --user --scope … -- cargo mutants --no-shuffle
/// --output <dir> --workspace` command.
fn build_cgroup_command(workspace_root: &Path) -> Command {
    let mut cmd = Command::new("systemd-run");
    let _ = cmd.current_dir(workspace_root);
    let _ = cmd.arg("--user");
    let _ = cmd.arg("--scope");
    let _ = cmd.arg("-p");
    let _ = cmd.arg(format!("MemoryHigh={CGROUP_MEMORY_HIGH}"));
    let _ = cmd.arg("-p");
    let _ = cmd.arg(format!("MemoryMax={CGROUP_MEMORY_MAX}"));
    let _ = cmd.arg("-p");
    let _ = cmd.arg("MemorySwapMax=0");
    let _ = cmd.arg("--");
    let _ = cmd.arg("cargo");
    let _ = cmd.arg("mutants");
    let _ = cmd.arg("--no-shuffle");
    let _ = cmd.arg("--output");
    let _ = cmd.arg(MUTANTS_OUTPUT_DIR);
    let _ = cmd.arg("--workspace");
    cmd
}

/// Build the bare `cargo mutants --no-shuffle --output <dir>
/// --workspace` command (cgroup fallback).
fn build_bare_command(workspace_root: &Path) -> Command {
    let mut cmd = Command::new("cargo");
    let _ = cmd.current_dir(workspace_root);
    let _ = cmd.arg("mutants");
    let _ = cmd.arg("--no-shuffle");
    let _ = cmd.arg("--output");
    let _ = cmd.arg(MUTANTS_OUTPUT_DIR);
    let _ = cmd.arg("--workspace");
    cmd
}

/// Materialise the argv that the cgroup wrapper would invoke.
///
/// Used by [`super::outcomes::build_clean_outcome`] to satisfy the
/// "command evidence must match actual argv" requirement when the run
/// completes clean.
#[must_use]
pub(super) fn cgroup_argv() -> Vec<String> {
    Vec::from([
        String::from("systemd-run"),
        String::from("--user"),
        String::from("--scope"),
        String::from("-p"),
        format!("MemoryHigh={CGROUP_MEMORY_HIGH}"),
        String::from("-p"),
        format!("MemoryMax={CGROUP_MEMORY_MAX}"),
        String::from("-p"),
        String::from("MemorySwapMax=0"),
        String::from("--"),
        String::from("cargo"),
        String::from("mutants"),
        String::from("--no-shuffle"),
        String::from("--output"),
        String::from(MUTANTS_OUTPUT_DIR),
        String::from("--workspace"),
    ])
}

/// Materialise the bare fallback argv.
#[must_use]
pub(super) fn bare_argv() -> Vec<String> {
    Vec::from([
        String::from("cargo"),
        String::from("mutants"),
        String::from("--no-shuffle"),
        String::from("--output"),
        String::from(MUTANTS_OUTPUT_DIR),
        String::from("--workspace"),
    ])
}

/// Spawn the workspace-level cargo-mutants invocation and wait up to
/// [`MUTANTS_WALLCLOCK_TIMEOUT_SECS`] for completion.
///
/// Reaps the child on timeout and surfaces a typed [`MutantsLaneError`]
/// on every failure path so the dispatch site can map it to a
/// `LaneFailure::Infra`.
///
/// # Errors
///
/// Returns [`MutantsLaneError::Spawn`] when `Command::spawn` fails,
/// [`MutantsLaneError::Wait`] for `wait_timeout` I/O errors,
/// [`MutantsLaneError::KillAfterTimeout`] /
/// [`MutantsLaneError::ReapAfterTimeout`] for post-timeout kill/reap
/// failures, and [`MutantsLaneError::TimedOut`] when the wallclock cap
/// is exceeded.
pub(super) fn run_workspace_command(
    workspace_root: &Path,
    cgroup_used: bool,
) -> Result<i32, MutantsLaneError> {
    let mut command = build_cargo_mutants_command(workspace_root, cgroup_used);
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| MutantsLaneError::Spawn(boxed_io_error(&error)))?;
    let timeout = Duration::from_secs(MUTANTS_WALLCLOCK_TIMEOUT_SECS);
    let wait_outcome = child
        .wait_timeout(timeout)
        .map_err(|error| MutantsLaneError::Wait(boxed_io_error(&error)))?;
    if let Some(status) = wait_outcome {
        return Ok(exit_code_from_status(status));
    }
    child.kill().map_err(|error| MutantsLaneError::KillAfterTimeout(boxed_io_error(&error)))?;
    let _ =
        child.wait().map_err(|error| MutantsLaneError::ReapAfterTimeout(boxed_io_error(&error)))?;
    Err(MutantsLaneError::TimedOut { timeout_secs: MUTANTS_WALLCLOCK_TIMEOUT_SECS })
}

/// Box the display representation of an [`io::Error`] so the
/// surrounding variant stays under the workspace
/// `large-error-threshold = 64`.
///
/// The typed source is recoverable at the dispatch site from the
/// formatted text.
fn boxed_io_error(error: &io::Error) -> Box<str> {
    Box::from(error.to_string().as_str())
}

/// Extract the exit code from an [`ExitStatus`].
///
/// Cargo-mutants is a normal child process that exits via `_exit`, so
/// we always expect a code. A signal-terminated child (negative codes
/// are reserved for kernel sentinels) collapses to `-1` and the caller
/// can decide how to render it; cargo-mutants does not raise real-time
/// signals under normal shutdown.
fn exit_code_from_status(status: ExitStatus) -> i32 {
    status.code().map_or(-1, |code| code)
}

#[cfg(test)]
mod tests {
    use super::{CGROUP_MEMORY_HIGH, CGROUP_MEMORY_MAX, MUTANTS_OUTPUT_DIR, cgroup_argv};

    #[test]
    fn cgroup_argv_first_arg_matches_systemd_run_executable() {
        let argv = cgroup_argv();
        assert_eq!(argv.first().map(String::as_str), Some("systemd-run"));
        assert!(argv.iter().any(|arg| arg == "--user"));
        assert!(argv.iter().any(|arg| arg == "--scope"));
        assert!(argv.iter().any(|arg| arg.starts_with("MemoryHigh=")));
        assert!(argv.iter().any(|arg| arg.starts_with("MemoryMax=")));
        assert!(argv.iter().any(|arg| arg == "MemorySwapMax=0"));
        assert!(argv.iter().any(|arg| arg == "--"));
        assert!(argv.iter().any(|arg| arg == "cargo"));
        assert!(argv.iter().any(|arg| arg == "mutants"));
        assert!(argv.iter().any(|arg| arg == "--no-shuffle"));
        assert!(argv.iter().any(|arg| arg == "--output"));
        assert!(argv.iter().any(|arg| arg == "--workspace"));
    }

    #[test]
    fn cgroup_argv_memory_limits_match_brief() {
        let argv = cgroup_argv();
        assert!(argv.iter().any(|arg| arg == &*format!("MemoryHigh={CGROUP_MEMORY_HIGH}")));
        assert!(argv.iter().any(|arg| arg == &*format!("MemoryMax={CGROUP_MEMORY_MAX}")));
        assert!(argv.iter().any(|arg| arg == "MemorySwapMax=0"));
    }

    #[test]
    fn cgroup_argv_preserves_direct_flag_order() {
        let argv = cgroup_argv();
        // --no-shuffle must precede --output for evidence clarity.
        let no_shuffle_idx = argv
            .iter()
            .position(|arg| arg == "--no-shuffle")
            .unwrap_or_else(|| panic!("cgroup argv must contain --no-shuffle"));
        let output_idx = argv
            .iter()
            .position(|arg| arg == "--output")
            .unwrap_or_else(|| panic!("cgroup argv must contain --output"));
        assert!(no_shuffle_idx < output_idx, "--no-shuffle must precede --output");
    }

    #[test]
    fn cgroup_argv_output_uses_locked_spec_directory_name() {
        let argv = cgroup_argv();
        let output_idx = argv
            .iter()
            .position(|arg| arg == "--output")
            .unwrap_or_else(|| panic!("cgroup argv must contain --output"));
        let value = argv
            .get(output_idx + 1)
            .unwrap_or_else(|| panic!("--output must be followed by a directory"));
        assert_eq!(value, MUTANTS_OUTPUT_DIR);
    }
}
