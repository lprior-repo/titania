use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use thiserror::Error;
use titania_core::TargetProject;
use titania_lanes::{CommandIn, LaneError, LaneExit, current_target_project, exit};

/// Default benches the bash lane exercised. Override by passing names on argv.
const DEFAULT_BENCHES: &[&str] = &["ir_traversal", "action_dispatch", "timer_wheel_tick"];

/// Toolchain pinned to the bash original.
const RUSTUP_TOOLCHAIN: &str = "nightly-2026-04-28";

/// `-p` target that owns the criterion benches.
const BENCH_PACKAGE: &str = "titania-workspace-tests";

#[derive(Debug, Clone, PartialEq, Eq)]
enum BenchPlan {
    Run(Vec<String>),
    NotApplicable(String),
}

#[derive(Debug, Error)]
enum BenchError {
    #[error(transparent)]
    Command(#[from] LaneError),
    #[error("{0}")]
    Parse(String),
}

pub(crate) fn main_exit(args: Vec<String>) -> ExitCode {
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

fn target_project() -> Result<TargetProject, ExitCode> {
    current_target_project().map_err(|err| {
        eprintln!("[bench-instruction-counts] cannot resolve target project: {err}");
        exit(LaneExit::Usage)
    })
}

fn runnable_benches(target: &TargetProject, args: Vec<String>) -> Result<Vec<String>, ExitCode> {
    match bench_plan(target, args) {
        Ok(BenchPlan::Run(benches)) => Ok(benches),
        Ok(BenchPlan::NotApplicable(reason)) => Err(not_applicable_exit(&reason)),
        Err(error) => Err(usage_error_exit(&error.to_string())),
    }
}

fn not_applicable_exit(reason: &str) -> ExitCode {
    eprintln!("[bench-instruction-counts] NotApplicable: {reason}");
    exit(LaneExit::NotApplicable)
}

fn usage_error_exit(err: &str) -> ExitCode {
    eprintln!("[bench-instruction-counts] {err}");
    exit(LaneExit::Usage)
}

fn run_bench_plan(target: &TargetProject, benches: &[String]) -> ExitCode {
    let Some((target_dir, evidence_dir)) = prepare_evidence_dirs(target) else {
        return missing_perf_exit();
    };
    if let Err(error) = require_perf(target) {
        eprintln!("[bench-instruction-counts] {error}");
        return LaneExit::Failure.into_exit_code();
    }
    for bench in benches {
        eprintln!("[bench-instruction-counts] running {bench}");
        if let Err(error) = run_compile(target, &target_dir, bench, &[]) {
            eprintln!("[bench-instruction-counts] {} failed: {error}", bench);
            return LaneExit::Violations.into_exit_code();
        }
        if let Err(error) = run_perf_stat(
            target,
            &target_dir,
            bench,
            &evidence_dir.join(format!("{bench}.perf.log")),
        ) {
            eprintln!("[bench-instruction-counts] {} failed: {error}", bench);
            return LaneExit::Violations.into_exit_code();
        }
    }
    LaneExit::Clean.into_exit_code()
}

/// Helper to map LaneExit (the shared exit-code enum) to a process ExitCode
/// for use as `Err(code)` in `Result<_, ExitCode>` flows.
trait IntoExitCode {
    fn into_exit_code(self) -> ExitCode;
}
impl IntoExitCode for LaneExit {
    fn into_exit_code(self) -> ExitCode {
        exit(self)
    }
}

fn require_perf(target: &TargetProject) -> Result<(), BenchError> {
    let mut perf = CommandIn::new(target, "perf")?;
    perf.inherit_env().arg("--version");
    let status = perf.run_status_raw()?;
    if status.success() {
        Ok(())
    } else {
        eprintln!("[bench-instruction-counts] perf check failed");
        Err(BenchError::Command(LaneError::NonZeroExit {
            program: "perf".to_owned(),
            code: status.code(),
            stderr: String::new(),
        }))
    }
}

fn missing_perf_exit() -> ExitCode {
    eprintln!("Missing required instruction counter: perf");
    exit(LaneExit::Failure)
}

fn prepare_evidence_dirs(target: &TargetProject) -> Option<(PathBuf, PathBuf)> {
    let target_dir = target.as_std_path().join("target/perf");
    let evidence_dir = target_dir.join("evidence");
    if let Err(e) = fs::create_dir_all(&evidence_dir) {
        eprintln!("[bench-instruction-counts] could not create evidence dir: {e}");
        return None;
    }
    Some((target_dir, evidence_dir))
}

fn bench_plan(target: &TargetProject, args: Vec<String>) -> Result<BenchPlan, BenchError> {
    if !package_manifest(target).is_file() {
        return Ok(BenchPlan::NotApplicable(
            "benchmark package titania-workspace-tests is absent".to_owned(),
        ));
    }
    let requested = requested_benches(args).map_err(BenchError::Parse)?;
    let available = available_benches(target, requested);
    if available.is_empty() {
        Ok(BenchPlan::NotApplicable(
            "target project has no requested instruction-count benches".to_owned(),
        ))
    } else {
        Ok(BenchPlan::Run(available))
    }
}

fn package_manifest(target: &TargetProject) -> PathBuf {
    target.as_std_path().join("crates/titania-workspace-tests/Cargo.toml")
}

fn requested_benches(args: Vec<String>) -> Result<Vec<String>, String> {
    let requested = if args.is_empty() {
        DEFAULT_BENCHES.iter().map(|bench| (*bench).to_owned()).collect()
    } else {
        args
    };
    if requested.iter().any(|bench| bench.is_empty()) {
        Err("empty benchmark name is not allowed".to_owned())
    } else {
        Ok(requested)
    }
}

fn available_benches(target: &TargetProject, requested: Vec<String>) -> Vec<String> {
    let benches_root = target.as_std_path().join("crates/titania-workspace-tests/benches");
    requested
        .into_iter()
        .filter(|bench| benches_root.join(format!("{bench}.rs")).is_file())
        .collect()
}

/// `cargo bench --bench NAME --all-features --no-run` (via rustup).
fn run_compile(
    target: &TargetProject,
    target_dir: &Path,
    bench: &str,
    extra: &[&str],
) -> Result<(), BenchError> {
    let target_dir_value = target_dir.display().to_string();
    let mut cmd = CommandIn::new(target, "rustup")?;
    cmd.inherit_env();
    append_compile_args(&mut cmd, bench, &target_dir_value, extra);
    run_with_status(&cmd, "cargo bench --no-run")
}

fn append_compile_args<'a>(
    cmd: &mut CommandIn<'a>,
    bench: &'a str,
    target_dir: &'a str,
    extra: &'a [&'a str],
) {
    cmd.arg("run").arg(RUSTUP_TOOLCHAIN).arg("cargo").arg("bench");
    cmd.args(&["-p", BENCH_PACKAGE]).arg("--bench").arg(bench);
    cmd.args(&["--all-features"]).arg("--no-run");
    cmd.env("CARGO_TARGET_DIR", target_dir).args(extra);
}

/// `perf stat -x, -e instructions -- rustup run nightly-… cargo bench -- --bench`
fn run_perf_stat(
    target: &TargetProject,
    target_dir: &Path,
    bench: &str,
    log_file: &Path,
) -> Result<(), BenchError> {
    let target_dir_value = target_dir.display().to_string();
    let log_file_value = log_file.display().to_string();
    let mut cmd = CommandIn::new(target, "perf")?;
    cmd.inherit_env();
    append_perf_args(&mut cmd, bench, &target_dir_value, &log_file_value);
    run_with_status(&cmd, "perf stat cargo bench")
}

fn append_perf_args<'a>(
    cmd: &mut CommandIn<'a>,
    bench: &'a str,
    target_dir: &'a str,
    log_file: &'a str,
) {
    cmd.args(&["stat", "-x,", "-e", "instructions", "-o"]).arg(log_file);
    cmd.arg("--").arg("rustup").arg("run").arg(RUSTUP_TOOLCHAIN);
    cmd.arg("cargo").arg("bench").args(&["-p", BENCH_PACKAGE]);
    cmd.arg("--bench").arg(bench).args(&["--all-features"]);
    cmd.arg("--").arg("--bench").env("CARGO_TARGET_DIR", target_dir);
}

fn run_with_status(cmd: &CommandIn<'_>, label: &str) -> Result<(), BenchError> {
    let status = cmd.run_status_raw()?;
    if status.success() {
        Ok(())
    } else {
        Err(BenchError::Parse(format!("{label} failed with exit {:?}", status.code())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requested_benches_uses_default_when_args_empty() {
        let result = requested_benches(vec![]).expect("default benches");
        assert_eq!(result, DEFAULT_BENCHES.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());
    }

    #[test]
    fn requested_benches_rejects_empty_bench_name() {
        let result = requested_benches(vec!["foo".to_owned(), "".to_owned()]);
        assert!(result.is_err());
    }
}
