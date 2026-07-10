//! Failing-first CLI emit-restriction tests for bead tn-ulrv.
//!
//! Verifies the v1 §12 grammar requirement: primary `check` and `aggregate`
//! commands accept `--emit json` only (and default to JSON). They must reject
//! `--emit human` as an `InputError`. Doctor retains both human and JSON
//! modes with human as its documented default.
//!
//! Uses `std::process::Command` and the `CARGO_BIN_EXE_titania-check`
//! environment variable set by `cargo test`.

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
    let mut cmd = Command::new(binary());
    let _ = cmd.current_dir(cwd);
    let _ = cmd.args(args);
    let _ = cmd.stdout(Stdio::piped());
    let _ = cmd.stderr(Stdio::piped());

    // Real binary needs no extra env setup; tests rely on default behavior.
    #[cfg(unix)]
    let _ = cmd.env("TITANIA_MOON_BIN", "/bin/true");

    let output = cmd.output().expect("failed to spawn titania-check");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

fn assert_input_error(code: i32, stderr: &str, expected_substring: &str) {
    assert_eq!(code, 3, "input error must exit 3; stderr: {stderr}");
    assert!(
        stderr.contains(expected_substring),
        "stderr must contain {expected_substring:?}: {stderr}"
    );
}

#[test]
fn check_rejects_emit_human() {
    let (code, _stdout, stderr) = run(&["check", "--emit", "human"]);
    assert_input_error(code, &stderr, "human");
}

#[test]
fn default_check_command_rejects_emit_human() {
    // `titania-check` with no subcommand dispatches to `check`. The same
    // restriction must apply.
    let (code, _stdout, stderr) = run(&["--emit", "human"]);
    assert_input_error(code, &stderr, "human");
}

#[test]
fn aggregate_rejects_emit_human() {
    let (code, _stdout, stderr) = run(&["aggregate", "--scope", "edit", "--emit", "human"]);
    assert_input_error(code, &stderr, "human");
}

// `titania-check check` dispatches through Moon; on Windows the production
// PATH lookup for `moon` (or the test stub of `/bin/true`) has no equivalent,
// so this scenario is exercised on Unix only. The portable scenarios above
// (check/aggregate --emit human rejection, aggregate --scope edit, and the
// doctor defaults) still run everywhere.
#[cfg(unix)]
#[test]
fn check_default_emits_json_without_explicit_flag() {
    // v1 §12 grammar: check defaults to JSON. We can't easily run a real
    // check here, but a workspace lacking lanes must surface a JSON report
    // matching the aggregate report schema. Failing this test means the
    // default drifted to a human renderer.
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (code, stdout, stderr) = run_in(workspace.path(), &["check"]);
    assert!(stderr.is_empty(), "check default JSON path must not write stderr: {stderr}");
    assert_eq!(code, 1, "empty workspace check must reject (exit 1): {stderr}");
    // The empty report shape is produced by the aggregate layer.
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("check default stdout must be JSON: {stdout}");
    assert_eq!(parsed["variant"], "Reject", "default JSON must include report variant: {parsed}");
    assert!(parsed.get("per_lane").is_some(), "default JSON must include per_lane: {parsed}");
}

#[test]
fn aggregate_default_emits_json_without_explicit_flag() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (code, stdout, stderr) = run_in(workspace.path(), &["aggregate", "--scope", "edit"]);
    assert!(stderr.is_empty(), "aggregate default JSON path must not write stderr: {stderr}");
    assert_eq!(code, 1, "empty workspace aggregate must reject (exit 1): {stderr}");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("aggregate default stdout must be JSON: {stdout}");
    assert_eq!(parsed["variant"], "Reject", "default JSON must include report variant: {parsed}");
    assert!(parsed.get("per_lane").is_some(), "default JSON must include per_lane: {parsed}");
}

#[test]
fn doctor_retains_emit_human_default() {
    // Doctor must NOT regress: human output stays its documented default.
    let (code, stdout, stderr) = run(&["doctor", "--scope", "edit"]);
    assert!(stderr.is_empty(), "doctor default human must not write stderr: {stderr}");
    assert!(code == 0 || code == 3, "doctor must exit 0 or 3, got: {code}");
    assert!(
        stdout.contains("titania-check doctor — scope: edit"),
        "doctor human header missing: {stdout}"
    );
}

#[test]
fn doctor_accepts_explicit_emit_json() {
    let (code, stdout, stderr) = run(&["doctor", "--scope", "edit", "--emit", "json"]);
    assert!(stderr.is_empty(), "doctor JSON must not write stderr: {stderr}");
    assert!(code == 0 || code == 3, "doctor must exit 0 or 3, got: {code}");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor JSON stdout must parse: {stdout}");
    assert_eq!(parsed["scope"], "edit", "doctor JSON must include scope=edit");
    assert!(parsed["tools"].is_array(), "doctor JSON must include tools array");
}
