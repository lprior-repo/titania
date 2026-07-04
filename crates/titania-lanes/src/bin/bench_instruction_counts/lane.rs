#![expect(
    clippy::redundant_pub_crate,
    reason = "lane entrypoint is called by the private bench_instruction_counts wrapper module"
)]

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use titania_core::TargetProject;
use titania_lanes::{CommandIn, LaneExit, current_target_project, exit};

/// Default benches the bash lane exercised. Override by passing names on argv.
const DEFAULT_BENCHES: &[&str] = &["ir_traversal", "action_dispatch", "timer_wheel_tick"];

/// Toolchain pinned to the bash original.
const RUSTUP_TOOLCHAIN: &str = "nightly-2026-04-28";

const USAGE: &str = "usage: bench_instruction_counts [bench-name ...]\n  \
     default benches: ir_traversal, action_dispatch, timer_wheel_tick";

enum BenchPlan {
    Run(Vec<String>),
    NotApplicable(String),
}

pub(super) fn main_exit(args: Vec<String>) -> ExitCode {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return exit_after_stderr(format_args!("{USAGE}"), LaneExit::Clean);
    }
    let target = match target_project() {
        Ok(target) => target,
        Err(code) => return code,
    };
    let benches = match runnable_benches(&target, args) {
        Ok(benches) => benches,
        Err(code) => return code,
    };
    run_bench_plan(&target, &benches)
}

/// # Errors
///
/// Returns an exit code when the current target project cannot be resolved.
fn target_project() -> Result<TargetProject, ExitCode> {
    current_target_project().map_err(|err| {
        exit_after_stderr(
            format_args!("[bench-instruction-counts] cannot resolve target project: {err}"),
            LaneExit::Usage,
        )
    })
}

/// # Errors
///
/// Returns an exit code when bench selection is invalid or not applicable.
fn runnable_benches(target: &TargetProject, args: Vec<String>) -> Result<Vec<String>, ExitCode> {
    match bench_plan(target, args) {
        Ok(BenchPlan::Run(benches)) => Ok(benches),
        Ok(BenchPlan::NotApplicable(reason)) => Err(not_applicable_exit(&reason)),
        Err(err) => Err(usage_error_exit(&err)),
    }
}

fn not_applicable_exit(reason: &str) -> ExitCode {
    exit_after_stderr(
        format_args!("[bench-instruction-counts] NotApplicable: {reason}"),
        LaneExit::NotApplicable,
    )
}

fn usage_error_exit(err: &str) -> ExitCode {
    exit_after_stderr(format_args!("[bench-instruction-counts] {err}"), LaneExit::Usage)
}

fn run_bench_plan(target: &TargetProject, benches: &[String]) -> ExitCode {
    if let Err(code) = require_perf(target) {
        return code;
    }
    let Some((target_dir, evidence_dir)) = prepare_evidence_dirs(target) else {
        return exit(LaneExit::Failure);
    };
    benches
        .iter()
        .map(|bench| run_one_bench(target, &target_dir, &evidence_dir, bench))
        .find(|code| *code != LaneExit::Clean)
        .map_or_else(|| exit(LaneExit::Clean), exit)
}

/// # Errors
///
/// Returns an exit code when `perf` cannot be prepared or executed.
fn require_perf(target: &TargetProject) -> Result<(), ExitCode> {
    let mut perf_check = CommandIn::new(target, "perf").map_err(|err| {
        exit_after_stderr(
            format_args!("[bench-instruction-counts] failed to prepare perf check: {err}"),
            LaneExit::Failure,
        )
    })?;
    let _ = perf_check.inherit_env().arg("--version");
    perf_check.run_capture_raw().map(|_| ()).map_err(|_error| missing_perf_exit())
}

fn missing_perf_exit() -> ExitCode {
    exit_after_stderr(format_args!("Missing required instruction counter: perf"), LaneExit::Failure)
}

fn prepare_evidence_dirs(target: &TargetProject) -> Option<(PathBuf, PathBuf)> {
    let target_dir = target.as_std_path().join("target/bench-instruction-counts");
    let evidence_dir = target_dir.join("evidence");
    if let Err(e) = fs::create_dir_all(&evidence_dir) {
        return evidence_dir_error(&e);
    }
    Some((target_dir, evidence_dir))
}

fn evidence_dir_error(error: &io::Error) -> Option<(PathBuf, PathBuf)> {
    match write_stderr_line(format_args!(
        "[bench-instruction-counts] could not create evidence dir: {error}"
    )) {
        Ok(()) | Err(_) => None,
    }
}

fn run_one_bench(
    target: &TargetProject,
    target_dir: &Path,
    evidence_dir: &Path,
    bench: &str,
) -> LaneExit {
    if bench.is_empty() {
        return empty_benchmark_exit();
    }
    let log_file = evidence_dir.join(format!("{bench}.perf.log"));
    if write_stderr_line(format_args!("[bench-instruction-counts] running {bench}")).is_err() {
        return LaneExit::Failure;
    }
    if let Err(code) = run_compile(target, target_dir, bench, &[])
        .and_then(|()| run_perf_stat(target, target_dir, bench, &log_file))
    {
        return failed_benchmark_exit(bench, &code);
    }
    if is_non_empty(&log_file) { LaneExit::Clean } else { empty_log_exit(&log_file) }
}

fn empty_benchmark_exit() -> LaneExit {
    lane_after_stderr(format_args!("Empty benchmark name is not allowed."), LaneExit::Usage)
}

fn failed_benchmark_exit(bench: &str, code: &str) -> LaneExit {
    lane_after_stderr(
        format_args!("[bench-instruction-counts] {bench} failed: {code}"),
        LaneExit::Violations,
    )
}

fn lane_after_stderr(args: std::fmt::Arguments<'_>, code: LaneExit) -> LaneExit {
    match write_stderr_line(args) {
        Ok(()) => code,
        Err(_) => LaneExit::Failure,
    }
}

fn empty_log_exit(log_file: &Path) -> LaneExit {
    if write_stderr_line(format_args!("Instruction-count log is empty: {}", log_file.display()))
        .is_err()
    {
        return LaneExit::Failure;
    }
    LaneExit::Violations
}

/// # Errors
///
/// Returns an error when requested benchmark names are invalid.
fn bench_plan(target: &TargetProject, args: Vec<String>) -> Result<BenchPlan, String> {
    if bench_roots(target).is_empty() {
        return Ok(BenchPlan::NotApplicable(
            "target project has no instruction-count bench directories".to_owned(),
        ));
    }
    let requested = requested_benches(args)?;
    let available = available_benches(target, requested);
    if available.is_empty() {
        Ok(BenchPlan::NotApplicable(
            "target project has no requested instruction-count benches".to_owned(),
        ))
    } else {
        Ok(BenchPlan::Run(available))
    }
}

fn bench_roots(target: &TargetProject) -> Vec<PathBuf> {
    let root = target.as_std_path();
    let workspace_benches = std::iter::once(root.join("benches"));
    let crate_benches = fs::read_dir(root.join("crates"))
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("benches"));
    workspace_benches.chain(crate_benches).filter(|path| path.is_dir()).collect()
}

/// # Errors
///
/// Returns an error when any requested benchmark name is empty.
fn requested_benches(args: Vec<String>) -> Result<Vec<String>, String> {
    let requested = if args.is_empty() {
        DEFAULT_BENCHES.iter().map(|bench| (*bench).to_owned()).collect()
    } else {
        args
    };
    if requested.iter().any(std::string::String::is_empty) {
        Err("empty benchmark name is not allowed".to_owned())
    } else {
        Ok(requested)
    }
}

fn available_benches(target: &TargetProject, requested: Vec<String>) -> Vec<String> {
    let roots = bench_roots(target);
    requested
        .into_iter()
        .filter(|bench| {
            roots.iter().any(|benches_root| benches_root.join(format!("{bench}.rs")).is_file())
        })
        .collect()
}

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/bench_instruction_counts/commands.rs"));

/// # Errors
///
/// Returns an error when the command cannot spawn or exits unsuccessfully.
fn run_with_status(cmd: &CommandIn<'_>, label: &str) -> Result<(), String> {
    let status = cmd.run_status_raw().map_err(|e| format!("failed to spawn {label}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} failed with exit {:?}", status.code()))
    }
}

fn is_non_empty(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|m| m.len() > 0)
}

fn exit_after_stderr(args: std::fmt::Arguments<'_>, code: LaneExit) -> ExitCode {
    match write_stderr_line(args) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}

/// # Errors
///
/// Returns an error when stderr cannot be written.
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_fmt(args)?;
    stderr.write_all(b"\n")
}
