//! Failing-first CLI dispatch tests for bead tn-cgk.1.
//!
//! Covers: parser/validation, exit-code mapping, dispatch shell.
//! Uses `std::process::Command` and the `CARGO_BIN_EXE_titania-check`
//! environment variable set by `cargo test`.
//!
//! Selective acceptance filter:
//! `cargo test -p titania-check cli_args_dispatch_missing_implementation_exit_codes`

use std::{
    env,
    path::Path,
    process::{Command, Stdio},
};

fn binary() -> std::path::PathBuf {
    env::var("CARGO_BIN_EXE_titania-check")
        .expect("CARGO_BIN_EXE_titania-check not set — run via `cargo test`")
        .into()
}

fn run(args: &[&str]) -> (i32, String, String) {
    let cwd = env::current_dir().expect("test current directory must be available");
    run_in(&cwd, args)
}

fn run_in(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(binary())
        .current_dir(cwd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to spawn titania-check");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

fn assert_stderr_contains(stderr: &str, needle: &str) {
    assert!(stderr.contains(needle), "stderr should contain {needle:?}, got: {stderr}");
}

fn assert_input_error(code: i32, stdout: &str, stderr: &str) {
    assert_eq!(code, 3, "exit code should be InputError(3), stderr: {stderr}");
    assert!(stdout.is_empty(), "InputError paths must not write stdout, got: {stdout}");
    assert_stderr_contains(stderr, "InputError:");
}

fn assert_missing_impl(args: &[&str], command: &str, bead: &str, detail: &str) {
    let (code, stdout, stderr) = run(args);
    assert_input_error(code, &stdout, &stderr);
    assert_stderr_contains(&stderr, "MissingImplementation");
    assert_stderr_contains(&stderr, &format!("command={command}"));
    assert_stderr_contains(&stderr, &format!("bead={bead}"));
    assert_stderr_contains(&stderr, detail);
}

fn assert_empty_workspace_reject(args: &[&str], expected_gate_failures: usize) {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (code, stdout, stderr) = run_in(workspace.path(), args);
    assert_eq!(code, 1, "reject reports must exit 1, stderr: {stderr}");
    assert!(stderr.is_empty(), "aggregate success path must not write stderr: {stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be JSON");
    assert_eq!(report["variant"], "reject");
    assert_eq!(
        report["gate_failures"].as_array().map(|items| items.len()),
        Some(expected_gate_failures),
    );
    assert_eq!(report["code_findings"].as_array().map(|items| items.len()), Some(0));
    assert_eq!(
        report["per_lane"].as_array().map(|items| items.len()),
        Some(expected_gate_failures),
    );
}

#[test]
fn cli_args_default_scope_edit() {
    assert_empty_workspace_reject(&[], 7);
}

#[test]
fn cli_args_scope_prepush() {
    assert_empty_workspace_reject(&["--scope", "prepush"], 9);
}

#[test]
fn cli_args_scope_release() {
    assert_empty_workspace_reject(&["--scope", "release"], 10);
}

#[test]
fn cli_args_emit_json_flag() {
    assert_empty_workspace_reject(&["--emit", "json"], 7);
}

#[test]
fn cli_args_out_path() {
    assert_empty_workspace_reject(&["--out", "/tmp/report.json"], 7);
}

#[test]
fn cli_args_unknown_scope_rejected() {
    let (code, stdout, stderr) = run(&["--scope", "full"]);
    assert_input_error(code, &stdout, &stderr);
    assert_stderr_contains(&stderr, "unknown scope");
    assert_stderr_contains(&stderr, "full");
}

#[test]
fn dispatch_default_check_aggregates_empty_workspace() {
    assert_empty_workspace_reject(&[], 7);
}

#[test]
fn dispatch_missing_implementation_run_lane_fmt() {
    assert_missing_impl(&["run-lane", "fmt"], "run-lane", "tn-uia", "lane 'fmt'");
}

#[test]
fn dispatch_missing_implementation_run_lane_clippy() {
    assert_missing_impl(&["run-lane", "clippy"], "run-lane", "tn-uia", "lane 'clippy'");
}

#[test]
fn dispatch_aggregate_subcommand_reads_empty_workspace() {
    assert_empty_workspace_reject(&["aggregate", "--scope", "edit"], 7);
}

#[test]
fn dispatch_missing_implementation_doctor() {
    assert_missing_impl(&["doctor"], "doctor", "tn-4rq.2", "scope 'edit'");
}

#[test]
fn dispatch_missing_implementation_explain() {
    assert_missing_impl(
        &["explain", "CLIPPY_UNWRAP_USED"],
        "explain",
        "tn-ja8.1",
        "rule 'CLIPPY_UNWRAP_USED'",
    );
}

#[test]
fn dispatch_missing_implementation_unknown_lane() {
    let (code, stdout, stderr) = run(&["run-lane", "nonexistent-lane"]);
    assert_input_error(code, &stdout, &stderr);
    assert_stderr_contains(&stderr, "unknown lane 'nonexistent-lane'");
}

#[test]
fn exit_codes_reject_report_exits_1() {
    assert_empty_workspace_reject(&[], 7);
}

#[test]
fn exit_codes_unknown_subcommand_rejected() {
    let (code, stdout, stderr) = run(&["badopt"]);
    assert_input_error(code, &stdout, &stderr);
    assert_stderr_contains(&stderr, "unknown subcommand 'badopt'");
}

#[test]
fn exit_codes_clippy_explain_rule() {
    assert_missing_impl(
        &["explain", "CLIPPY_UNWRAP_USED"],
        "explain",
        "tn-ja8.1",
        "rule 'CLIPPY_UNWRAP_USED'",
    );
}

#[test]
fn exit_codes_doctor_returns_input_error() {
    assert_missing_impl(&["doctor"], "doctor", "tn-4rq.2", "scope 'edit'");
}

#[test]
fn exit_codes_run_lane_invalid_lane() {
    let (code, stdout, stderr) = run(&["run-lane", "invalid-lane"]);
    assert_input_error(code, &stdout, &stderr);
    assert_stderr_contains(&stderr, "unknown lane 'invalid-lane'");
}

#[test]
fn exit_codes_scope_emit_out_combination() {
    assert_empty_workspace_reject(
        &["--scope", "prepush", "--emit", "json", "--out", "/tmp/test-report.json"],
        9,
    );
}

#[test]
fn exit_codes_scope_prepush_emit_json() {
    assert_empty_workspace_reject(&["--scope", "prepush", "--emit", "json"], 9);
}

#[test]
fn exit_codes_scope_release_emit_json_out() {
    assert_empty_workspace_reject(
        &["--scope", "release", "--emit", "json", "--out", "/tmp/release-report.json"],
        10,
    );
}

#[test]
fn cli_args_dispatch_missing_implementation_exit_codes() {
    assert_empty_workspace_reject(&[], 7);
    assert_missing_impl(&["run-lane", "fmt"], "run-lane", "tn-uia", "lane 'fmt'");
    assert_missing_impl(&["doctor"], "doctor", "tn-4rq.2", "scope 'edit'");
    assert_missing_impl(
        &["explain", "CLIPPY_UNWRAP_USED"],
        "explain",
        "tn-ja8.1",
        "rule 'CLIPPY_UNWRAP_USED'",
    );
    let (code, stdout, stderr) = run(&["run-lane", "invalid-lane"]);
    assert_input_error(code, &stdout, &stderr);
    assert_stderr_contains(&stderr, "unknown lane 'invalid-lane'");
}
