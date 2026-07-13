//! Failing-first CLI dispatch tests for bead tn-cgk.1.
//!
//! Covers: parser/validation, exit-code mapping, dispatch shell, and lane execution.
//! Uses `std::process::Command` and the `CARGO_BIN_EXE_titania-check`
//! environment variable set by `cargo test`.
//!
//! Selective acceptance filter:
//! `cargo test -p titania-check cli_args_dispatch_missing_implementation_exit_codes`

use serde_json::Value;
use std::{
    env, fs,
    path::Path,
    process::{Command, Stdio},
};
use tempfile::TempDir;

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

    // Stub Moon via TITANIA_MOON_BIN so `Command::Check` does not invoke the
    // real moon binary (which would error on tempdirs without `.moon/`).
    // `/bin/true` exits 0 with any args. Tests that exercise real Moon dispatch
    // (missing-moon handling, moon-invocation proof) override or clear this.
    // `/bin/true` is Unix-only; gate the env override so that Windows
    // compilation never carries the literal. Tests that go through
    // `Command::Check` and depend on this stub exiting 0 are themselves
    // gated to `cfg(unix)` below; on non-Unix the production moon spawn
    // falls back to the `moon` PATH lookup, which those tests avoid.
    #[cfg(unix)]
    let _ = cmd.env("TITANIA_MOON_BIN", "/bin/true");

    // Pass CARGO_TARGET_DIR through as-is so that library_is_available
    // can resolve it relative to the workspace root when walking up
    // from CARGO_MANIFEST_DIR.
    if let Ok(ctd) = env::var("CARGO_TARGET_DIR") {
        let _ = cmd.env("CARGO_TARGET_DIR", ctd);
    }

    let output = cmd.output().expect("failed to spawn titania-check");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

fn run_in_without_policy_env(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(binary());
    let _ = cmd.current_dir(cwd);
    let _ = cmd.args(args);
    let _ = cmd.stdout(Stdio::piped());
    let _ = cmd.env_remove("RUSTFLAGS");
    let _ = cmd.env_remove("CARGO_ENCODED_RUSTFLAGS");
    let _ = cmd.env_remove("RUSTC_WRAPPER");
    let _ = cmd.env_remove("RUSTC_WORKSPACE_WRAPPER");
    let _ = cmd.env_remove("RUSTC_BOOTSTRAP");
    // Stub Moon (see run_in for rationale). Unix-only.
    #[cfg(unix)]
    let _ = cmd.env("TITANIA_MOON_BIN", "/bin/true");

    if let Ok(ctd) = env::var("CARGO_TARGET_DIR") {
        let _ = cmd.env("CARGO_TARGET_DIR", ctd);
    }
    let _ = cmd.env("CARGO_HOME", cwd.join(".titania").join("hermetic").join("cargo-home"));
    let _ = cmd.env("RUSTUP_HOME", cwd.join(".titania").join("hermetic").join("rustup-home"));

    let output = cmd.output().expect("failed to spawn titania-check");
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

fn assert_known_explain(args: &[&str], rule: &str) {
    let (code, stdout, stderr) = run(args);
    assert_eq!(code, 0, "known explain rule must exit 0, stderr: {stderr}");
    assert!(stderr.is_empty(), "known explain rule must not write stderr: {stderr}");
    assert!(stdout.starts_with(rule), "stdout must start with {rule}: {stdout}");
    assert!(stdout.contains("Pattern:"), "stdout must include pattern metadata: {stdout}");
    assert!(stdout.contains("Effect:"), "stdout must include effect metadata: {stdout}");
    assert!(
        stdout.contains("Example violation:"),
        "stdout must include violation sample: {stdout}"
    );
    assert!(stdout.contains("Example repair:"), "stdout must include repair sample: {stdout}");
}

fn assert_empty_workspace_reject(args: &[&str], expected_gate_failures: usize) {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (code, stdout, stderr) = run_in(workspace.path(), args);
    assert_eq!(code, 1, "reject reports must exit 1, stderr: {stderr}");
    assert!(stderr.is_empty(), "aggregate success path must not write stderr: {stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be JSON");
    assert_eq!(report["variant"], "Reject");
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

/// Helper: build a minimal Cargo project (lib + bin) and return the TempDir.
fn package(name: &str, lib_rs: &str, main_rs: &str) -> Result<TempDir, std::io::Error> {
    let temp = tempfile::tempdir()?;
    fs::write(
        temp.path().join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
             description = \"temporary Titania lane test package\"\n\
             license = \"MIT\"\n\
             repository = \"https://example.com/titania-test\"\n\
             readme = \"README.md\"\n\
             keywords = [\"titania\"]\n\
             categories = [\"development-tools\"]\n"
        ),
    )?;
    fs::create_dir_all(temp.path().join("src"))?;
    fs::write(temp.path().join("src/lib.rs"), lib_rs)?;
    fs::write(temp.path().join("README.md"), "# Temporary Titania lane fixture\n")?;
    fs::write(temp.path().join("src/main.rs"), main_rs)?;
    Ok(temp)
}

/// Resolve the artifact path for the fmt lane in the edit scope.
fn fmt_artifact_path(root: &Path) -> std::path::PathBuf {
    root.join(".titania/out/edit/fmt.json")
}

/// Resolve the artifact path for the clippy lane in the edit scope.
fn clippy_artifact_path(root: &Path) -> std::path::PathBuf {
    root.join(".titania/out/edit/clippy.json")
}

/// Resolve the artifact path for the test lane in the prepush scope.
fn prepush_test_artifact_path(root: &Path) -> std::path::PathBuf {
    root.join(".titania/out/prepush/test.json")
}

/// Resolve the artifact path for the policy-scan lane in the release scope.
fn release_policy_artifact_path(root: &Path) -> std::path::PathBuf {
    root.join(".titania/out/release/policy-scan.json")
}

fn generate_lockfile(root: &Path) {
    let output = Command::new("cargo")
        .current_dir(root)
        .arg("generate-lockfile")
        .output()
        .expect("cargo generate-lockfile must run");
    assert!(
        output.status.success(),
        "cargo generate-lockfile must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// Default `--scope edit --emit json` dispatches through `Command::Check`, which
// spawns Moon. Without a Unix stub (`/bin/true`) Moon's PATH lookup fails on
// Windows and the assertion shape (exit 1 reject on empty workspace) breaks.
#[cfg(unix)]
#[test]
fn cli_args_default_scope_edit() {
    assert_empty_workspace_reject(&["--emit", "json"], 7);
}

#[cfg(unix)]
#[test]
fn cli_args_scope_prepush() {
    assert_empty_workspace_reject(&["--emit", "json", "--scope", "prepush"], 9);
}

#[cfg(unix)]
#[test]
fn cli_args_scope_release() {
    assert_empty_workspace_reject(&["--emit", "json", "--scope", "release"], 10);
}

#[cfg(unix)]
#[test]
fn cli_args_emit_json_flag() {
    assert_empty_workspace_reject(&["--emit", "json"], 7);
}

// `--out <path>` with `--emit json` (no subcommand) routes through
// `Command::Check` and Moon; see the rationale on `cli_args_default_scope_edit`.
#[cfg(unix)]
#[test]
fn cli_args_out_path() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let out_path = workspace.path().join("report.json");
    let (code, stdout, _stderr) =
        run_in(workspace.path(), &["--emit", "json", "--out", out_path.to_str().unwrap()]);
    assert_eq!(code, 1, "reject must exit 1");
    assert!(stdout.is_empty(), "--out must suppress stdout");
    let report_text =
        fs::read_to_string(&out_path).expect("--out path must contain the report file");
    let report: serde_json::Value =
        serde_json::from_str(&report_text).expect("written report must be valid JSON");
    assert_eq!(report["variant"], "Reject");
    assert_eq!(report["gate_failures"].as_array().map(|items| items.len()), Some(7),);
}
#[test]
fn cli_args_unknown_scope_rejected() {
    let (code, stdout, stderr) = run(&["--scope", "full"]);
    assert_input_error(code, &stdout, &stderr);
    assert_stderr_contains(&stderr, "unknown scope");
    assert_stderr_contains(&stderr, "full");
}

// Default `--emit json` dispatches to `Command::Check`; see rationale on
// `cli_args_default_scope_edit`.
#[cfg(unix)]
#[test]
fn dispatch_default_check_aggregates_empty_workspace() {
    assert_empty_workspace_reject(&["--emit", "json"], 7);
}

#[test]
fn dispatch_run_lane_fmt_writes_typed_clean_artifact() {
    let temp =
        package("dispatch_fmt_clean", "pub fn value() -> u8 {\n    1\n}\n", "fn main() {}\n")
            .expect("temp package must be created");
    let (code, stdout, stderr) = run_in(temp.path(), &["run-lane", "fmt"]);
    assert_eq!(code, 0, "run-lane fmt on clean project must exit 0, stderr: {stderr}");
    assert!(stdout.is_empty(), "run-lane fmt must not write stdout, got: {stdout}");
    assert!(stderr.is_empty(), "run-lane fmt must not write stderr, got: {stderr}");
    let artifact = fmt_artifact_path(temp.path());
    assert!(artifact.exists(), "fmt artifact must exist at .titania/out/edit/fmt.json");
    let payload = fs::read_to_string(&artifact).expect("must read fmt artifact");
    let json: Value = serde_json::from_str(&payload).expect("artifact must be valid JSON");
    assert_eq!(json["lane"].as_str(), Some("Fmt"), "lane must be Fmt");
    assert!(json["outcome"].get("Clean").is_some(), "outcome must be Clean");
    // Command evidence: executable, argv, tool_version, exit_status.
    let evidence = &json["outcome"]["Clean"]["evidence"];
    let cmd = &evidence["command"];
    assert_eq!(cmd["executable"].as_str(), Some("cargo"), "command executable must be cargo");
    let argv: Vec<&str> = cmd["argv"]
        .as_array()
        .expect("argv must be a JSON array")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(
        argv,
        vec!["cargo", "fmt", "--all", "--check"],
        "command argv must be [cargo, fmt, --all, --check]; got {argv:?}"
    );
    assert!(
        evidence["tool_version"].as_str().is_some(),
        "tool_version must be non-null, got: {:#}",
        evidence["tool_version"]
    );
    assert_eq!(
        evidence["exit_status"]["Exited"]["code"].as_i64(),
        Some(0),
        "exit_status.Exited.code must be 0, got: {:#}",
        evidence["exit_status"]
    );
}

#[test]
fn dispatch_run_lane_clippy_executes_on_clean_project() {
    let temp = package(
        "dispatch_clippy_clean",
        "#[must_use]\npub const fn value() -> u8 {\n    1\n}\n",
        "fn main() {}\n",
    )
    .expect("temp package must be created");
    // Generate a lock file (clippy lane uses --frozen which requires one).
    generate_lockfile(temp.path());
    let (code, stdout, stderr) = run_in(temp.path(), &["run-lane", "clippy"]);
    assert_eq!(code, 0, "run-lane clippy on clean project must exit 0, stderr: {stderr}");
    assert!(stdout.is_empty(), "run-lane clippy must not write stdout, got: {stdout}");
    assert!(stderr.is_empty(), "run-lane clippy must not write stderr, got: {stderr}");
    let artifact = clippy_artifact_path(temp.path());
    assert!(artifact.exists(), "clippy artifact must exist at .titania/out/edit/clippy.json");
    let payload = fs::read_to_string(&artifact).expect("must read clippy artifact");
    let json: Value = serde_json::from_str(&payload).expect("artifact must be valid JSON");
    assert_eq!(json["lane"].as_str(), Some("Clippy"), "lane must be Clippy");
    assert!(json["outcome"].get("Clean").is_some(), "outcome must be Clean");
}

#[test]
fn dispatch_run_lane_test_successful_cargo_output_writes_clean_artifact() {
    let lib_rs = r#"
pub fn value() -> u8 {
    1
}

#[cfg(test)]
mod tests {
    use super::value;

    #[test]
    fn value_is_one() {
        assert_eq!(value(), 1);
    }
}
"#;
    let temp = package("dispatch_test_clean_output", lib_rs, "fn main() {}\n")
        .expect("temp package must be created");
    generate_lockfile(temp.path());

    let (code, stdout, stderr) = run_in(temp.path(), &["run-lane", "test"]);

    assert_eq!(code, 0, "run-lane test on clean project must exit 0, stderr: {stderr}");
    assert!(stdout.is_empty(), "run-lane test must not write stdout, got: {stdout}");
    assert!(stderr.is_empty(), "clean run-lane test must not write stderr, got: {stderr}");

    let artifact = prepush_test_artifact_path(temp.path());
    assert!(artifact.exists(), "test artifact must exist at .titania/out/prepush/test.json");
    let payload = fs::read_to_string(&artifact).expect("must read test artifact");
    let json: Value = serde_json::from_str(&payload).expect("artifact must be valid JSON");

    assert_eq!(json["lane"].as_str(), Some("Test"), "lane must be Test");
    assert!(json["outcome"].get("Clean").is_some(), "successful cargo test output must be Clean");
    assert!(
        json["outcome"].get("Findings").is_none(),
        "successful cargo test output must not produce Findings: {:#}",
        json["outcome"]
    );
}

#[test]
fn dispatch_run_lane_test_nonzero_cargo_output_writes_tool_failure_artifact() {
    let lib_rs = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn value_is_not_two() {
        assert_eq!(1, 2);
    }
}
"#;
    let temp = package("dispatch_test_nonzero_output", lib_rs, "fn main() {}\n")
        .expect("temp package must be created");
    generate_lockfile(temp.path());

    let (code, stdout, stderr) = run_in(temp.path(), &["run-lane", "test"]);

    // Per v1-spec §12, a gate failure (LaneFailure::Tool) is a Reject (exit 1).
    assert_eq!(code, 1, "run-lane test with failing cargo test must reject, stderr: {stderr}");
    assert!(stdout.is_empty(), "run-lane test must not write stdout, got: {stdout}");

    let artifact = prepush_test_artifact_path(temp.path());
    assert!(artifact.exists(), "test artifact must exist at .titania/out/prepush/test.json");
    let payload = fs::read_to_string(&artifact).expect("must read test artifact");
    let json: Value = serde_json::from_str(&payload).expect("artifact must be valid JSON");

    assert_eq!(json["lane"].as_str(), Some("Test"), "lane must be Test");
    assert!(json["outcome"].get("Clean").is_none(), "nonzero cargo test must not be Clean");
    assert!(
        json["outcome"].get("Findings").is_none(),
        "nonzero cargo test must not produce Findings per v1-spec §5"
    );
    // Per v1-spec §11.2 example: Test failures emit Failed { ToolFailure { tool, termination } }.
    let failed = &json["outcome"]["Failed"];
    assert!(failed.get("ToolFailure").is_some(), "must be ToolFailure, got: {failed:#}");
    let tool = failed["ToolFailure"]["tool"].as_str().expect("tool must be a string");
    assert_eq!(tool, "cargo-test", "tool must be cargo-test per v1-spec §11.2");
    let exit_code = failed["ToolFailure"]["termination"]["Exited"]["code"]
        .as_u64()
        .expect("termination must be Exited with code");
    // cargo test exits 101 when the test harness reports failures.
    assert_eq!(exit_code, 101, "cargo test exit code must be 101 (test failure)");
}

#[test]
fn dispatch_run_lane_policy_scan_expired_exception_writes_release_findings() {
    let temp =
        package("dispatch_policy_expired_exception", "pub fn value() -> u8 {\n    1\n}\n", "")
            .expect("temp package must be created");
    let exceptions_dir = temp.path().join(".titania/profiles/strict-ai");
    fs::create_dir_all(&exceptions_dir).expect("exceptions directory must be created");
    fs::write(
        exceptions_dir.join("exceptions.toml"),
        r#"
[[exceptions]]
rule_id = "BYPASS_EXPECT_ATTR"
path = "crates/titania-dylint/src/lib.rs"
owner = "titania-maintainers"
reason = "Dylint ABI exports require audited temporary exception"
expires_on = "2020-01-01"
review = "tn-dylint-abi-expect"
"#,
    )
    .expect("expired exceptions fixture must be written");

    let (code, stdout, stderr) =
        run_in_without_policy_env(temp.path(), &["run-lane", "policy-scan"]);

    assert_eq!(code, 1, "expired exception must reject policy-scan, stderr: {stderr}");
    assert!(stdout.is_empty(), "run-lane policy-scan must not write stdout, got: {stdout}");
    assert_stderr_contains(&stderr, "1 finding(s)");

    let artifact = release_policy_artifact_path(temp.path());
    assert!(artifact.exists(), "policy-scan release artifact must exist");
    let payload = fs::read_to_string(&artifact).expect("must read policy-scan release artifact");
    let json: Value = serde_json::from_str(&payload).expect("artifact must be valid JSON");
    assert_eq!(json["lane"].as_str(), Some("PolicyScan"), "lane must be PolicyScan");
    let findings = json["outcome"]["Findings"].as_array().expect("findings must be an array");
    assert_eq!(findings.len(), 1, "expired exception must emit exactly one finding");
    assert_eq!(findings[0]["rule_id"].as_str(), Some("POLICY_EXCEPTION_EXPIRED"));
    assert_eq!(findings[0]["lane"].as_str(), Some("PolicyScan"));
    assert_eq!(findings[0]["effect"].as_str(), Some("Reject"));
    assert!(
        findings[0]["message"]
            .as_str()
            .is_some_and(|message| message.contains("BYPASS_EXPECT_ATTR")),
        "finding must name expired rule, got: {:#}",
        findings[0]["message"]
    );
}

#[test]
fn dispatch_aggregate_subcommand_reads_empty_workspace() {
    assert_empty_workspace_reject(&["aggregate", "--emit", "json", "--scope", "edit"], 7);
}

#[test]
fn dispatch_doctor_emits_human_table() {
    let (code, stdout, stderr) = run(&["doctor", "--scope", "edit"]);
    assert!(stderr.is_empty(), "doctor must not write stderr: {stderr}");
    assert!(stdout.contains("titania-check doctor"), "stdout must contain header: {stdout}");
    assert!(stdout.contains("Tool"), "stdout must contain Tool column header: {stdout}");
    assert!(stdout.contains("Required"), "stdout must contain Required column header: {stdout}");
    assert!(stdout.contains("Status:"), "stdout must contain Status line: {stdout}");
    // Exit code is 0 if all required tools are installed, 3 if missing
    assert!(code == 0 || code == 3, "doctor exit code must be 0 or 3, got: {code}");
}

#[test]
fn dispatch_explain_known_rule() {
    assert_known_explain(&["explain", "CLIPPY_UNWRAP_USED"], "CLIPPY_UNWRAP_USED");
}

#[test]
fn dispatch_missing_implementation_unknown_lane() {
    let (code, stdout, stderr) = run(&["run-lane", "nonexistent-lane"]);
    assert_input_error(code, &stdout, &stderr);
    assert_stderr_contains(&stderr, "unknown lane 'nonexistent-lane'");
}

// Default `--emit json` dispatches to `Command::Check`; see rationale on
// `cli_args_default_scope_edit`.
#[cfg(unix)]
#[test]
fn exit_codes_reject_report_exits_1() {
    assert_empty_workspace_reject(&["--emit", "json"], 7);
}

#[test]
fn exit_codes_unknown_subcommand_rejected() {
    let (code, stdout, stderr) = run(&["badopt"]);
    assert_input_error(code, &stdout, &stderr);
    assert_stderr_contains(&stderr, "unknown subcommand 'badopt'");
}

#[test]
fn exit_codes_clippy_explain_rule() {
    assert_known_explain(&["explain", "CLIPPY_UNWRAP_USED"], "CLIPPY_UNWRAP_USED");
}
#[test]
fn exit_codes_doctor_emits_json() {
    let (code, stdout, stderr) = run(&["doctor", "--scope", "edit", "--emit", "json"]);
    assert!(stderr.is_empty(), "doctor must not write stderr: {stderr}");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout must be valid JSON");
    assert_eq!(parsed["scope"], "edit", "JSON must contain scope 'edit'");
    assert!(parsed["tools"].is_array(), "JSON must contain tools array");
    assert!(parsed["status"].is_string(), "JSON must contain status string");
    assert!(
        parsed["status"] == "OK" || parsed["status"] == "MissingRequiredTools",
        "status must be OK or MissingRequiredTools"
    );
    assert!(code == 0 || code == 4, "doctor exit code must be 0 or 4, got: {code}");
}
#[test]
fn exit_codes_run_lane_invalid_lane() {
    let (code, stdout, stderr) = run(&["run-lane", "invalid-lane"]);
    assert_input_error(code, &stdout, &stderr);
    assert_stderr_contains(&stderr, "unknown lane 'invalid-lane'");
}

// `--scope prepush --emit json --out ...` (no subcommand) routes through
// `Command::Check`; see rationale on `cli_args_default_scope_edit`.
#[cfg(unix)]
#[test]
fn exit_codes_scope_emit_out_combination() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let out_path = workspace.path().join("test-report.json");
    let (code, stdout, _stderr) = run_in(
        workspace.path(),
        &["--scope", "prepush", "--emit", "json", "--out", out_path.to_str().unwrap()],
    );
    assert_eq!(code, 1, "reject must exit 1");
    assert!(stdout.is_empty(), "--out must suppress stdout");
    let report: serde_json::Value = serde_json::from_str(&fs::read_to_string(&out_path).unwrap())
        .expect("file must contain valid JSON");
    assert_eq!(report["variant"], "Reject");
    assert_eq!(report["gate_failures"].as_array().map(|items| items.len()), Some(9),);
}

// `--scope prepush --emit json` (no subcommand) dispatches to `Command::Check`;
// see rationale on `cli_args_default_scope_edit`.
#[cfg(unix)]
#[test]
fn exit_codes_scope_prepush_emit_json() {
    assert_empty_workspace_reject(&["--scope", "prepush", "--emit", "json"], 9);
}

// `--scope release --emit json --out ...` (no subcommand) routes through
// `Command::Check`; see rationale on `cli_args_default_scope_edit`.
#[cfg(unix)]
#[test]
fn exit_codes_scope_release_emit_json_out() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let out_path = workspace.path().join("release-report.json");
    let (code, stdout, _stderr) = run_in(
        workspace.path(),
        &["--scope", "release", "--emit", "json", "--out", out_path.to_str().unwrap()],
    );
    assert_eq!(code, 1, "reject must exit 1");
    assert!(stdout.is_empty(), "--out must suppress stdout");
    let report: serde_json::Value = serde_json::from_str(&fs::read_to_string(&out_path).unwrap())
        .expect("file must contain valid JSON");
    assert_eq!(report["variant"], "Reject");
    assert_eq!(report["gate_failures"].as_array().map(|items| items.len()), Some(10),);
}

// Aggregates a `Command::Check` scenario with follow-on doctor/explain/run-lane
// probes. The first assertion routes through Moon and depends on the Unix
// `/bin/true` stub, so the whole acceptance-filter check is Unix-only.
#[cfg(unix)]
#[test]
fn cli_args_dispatch_missing_implementation_exit_codes() {
    assert_empty_workspace_reject(&["--emit", "json"], 7);
    // Doctor now uses real implementation; verify JSON output parses
    let (code, stdout, _stderr) = run(&["doctor", "--scope", "edit", "--emit", "json"]);
    assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_ok(), "doctor JSON must parse");
    assert!(code == 0 || code == 3, "doctor exit code must be 0 or 3, got: {code}");
    assert_known_explain(&["explain", "CLIPPY_UNWRAP_USED"], "CLIPPY_UNWRAP_USED");
    let (code, stdout, stderr) = run(&["run-lane", "invalid-lane"]);
    assert_input_error(code, &stdout, &stderr);
    assert_stderr_contains(&stderr, "unknown lane 'invalid-lane'");
}

/// M7: `titania-check check --scope edit` must dispatch to the aggregate check
/// path (exit 1 for reject on empty workspace), not return UnknownSubcommand.
#[cfg(unix)]
#[test]
fn m7_check_subcommand_dispatches_to_aggregate() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (code, stdout, stderr) =
        run_in(workspace.path(), &["check", "--emit", "json", "--scope", "edit"]);
    // If the check subcommand were not recognized, we would get exit code 3
    // with stderr "unknown subcommand 'check'". Instead it should dispatch
    // to aggregate and return a reject report (exit 1).
    assert_ne!(
        code, 3,
        "check subcommand must not return UnknownSubcommand(exit 3); stderr: {stderr}",
    );
    assert_eq!(code, 1, "check on empty workspace must exit 1 (reject), stderr: {stderr}");
    let report: serde_json::Value =
        serde_json::from_str(&stdout).expect("check must emit JSON report to stdout");
    assert_eq!(report["variant"], "Reject", "check report variant must be Reject");
}
/// M7: `titania-check check` (no explicit scope, default edit) must dispatch
/// to aggregate and produce a typed reject report.
#[cfg(unix)]
#[test]
fn m7_check_subcommand_default_scope() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (code, stdout, stderr) = run_in(workspace.path(), &["check", "--emit", "json"]);
    assert_eq!(code, 1, "check default on empty workspace must exit 1 (reject), stderr: {stderr}");
    assert!(stderr.is_empty(), "check aggregate path must not write stderr: {stderr}",);
    let report: serde_json::Value =
        serde_json::from_str(&stdout).expect("check must emit JSON report");
    assert_eq!(report["variant"], "Reject");
}

// ===== M8: --emit must affect output format for check/aggregate commands =====

/// M8: `--emit json` must produce parseable JSON output (not text/table format).
#[cfg(unix)]
#[test]
fn m8_emit_json_produces_valid_json() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (code, stdout, stderr) = run_in(workspace.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(code, 1, "reject on empty workspace must exit 1, stderr: {stderr}");
    // Must parse as JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--emit json stdout must be valid JSON");
    assert_eq!(parsed["variant"], "Reject", "JSON report must have variant field");
}

/// M8: primary report commands reject the human renderer under v1 §12.
#[cfg(unix)]
#[test]
fn m8_emit_human_rejected() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (code, stdout, stderr) = run_in(workspace.path(), &["--scope", "edit", "--emit", "human"]);
    assert_eq!(code, 3, "human emit must be an input error, stderr: {stderr}");
    assert!(stdout.is_empty(), "rejected human emit must not write stdout: {stdout}");
    assert_stderr_contains(&stderr, "unknown emit format");
}

/// M8: primary report commands accept JSON and reject human output.
#[cfg(unix)]
#[test]
fn m8_emit_format_changes_output() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (json_code, json_out, json_stderr) =
        run_in(workspace.path(), &["--scope", "edit", "--emit", "json"]);
    assert_eq!(json_code, 1, "JSON reject must exit 1, stderr: {json_stderr}");
    drop(serde_json::from_str::<serde_json::Value>(&json_out).expect("JSON output must parse"));

    let (human_code, human_out, human_stderr) =
        run_in(workspace.path(), &["--scope", "edit", "--emit", "human"]);
    assert_eq!(human_code, 3, "human emit must be an input error, stderr: {human_stderr}");
    assert!(human_out.is_empty(), "rejected human emit must not write stdout: {human_out}");
    assert_stderr_contains(&human_stderr, "unknown emit format");
}

/// M8: invalid `--emit` value must be rejected.
#[test]
fn m8_emit_invalid_value_rejected() {
    let (code, stdout, stderr) = run(&["--scope", "edit", "--emit", "xml"]);
    assert_input_error(code, &stdout, &stderr);
    assert_stderr_contains(&stderr, "unknown emit format");
}

// ===== M8: --out must write report to file =====

/// M8: `--out <path>` must write the report to the specified file path.
/// On success, stdout must be empty and the file must contain the JSON report.
#[cfg(unix)]
#[test]
fn m8_out_path_writes_report_to_file() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let out_path = workspace.path().join("report.json");
    let (code, stdout, stderr) = run_in(
        workspace.path(),
        &["--emit", "json", "--scope", "edit", "--out", out_path.to_str().unwrap()],
    );
    assert_eq!(code, 1, "reject must exit 1, stderr: {stderr}");
    // When --out is specified, stdout must be empty
    assert!(stdout.is_empty(), "--out must suppress stdout; got: {stdout}");
    // The report file must exist and contain valid JSON
    let report_text =
        fs::read_to_string(&out_path).expect("--out path must contain the report file");
    let parsed: serde_json::Value =
        serde_json::from_str(&report_text).expect("--out file must contain valid JSON");
    assert_eq!(parsed["variant"], "Reject", "written report must have variant field");
}

/// M8: `--out` with `--emit json` writes JSON to file.
#[cfg(unix)]
#[test]
fn m8_out_with_emit_json_writes_json_file() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let out_path = workspace.path().join("json-report.json");
    let (code, stdout, stderr) = run_in(
        workspace.path(),
        &["--scope", "edit", "--emit", "json", "--out", out_path.to_str().unwrap()],
    );
    assert_eq!(code, 1, "reject must exit 1, stderr: {stderr}");
    assert!(stdout.is_empty(), "--out must suppress stdout; got: {stdout}");
    let content = fs::read_to_string(&out_path).expect("file must be written at --out path");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("file must contain JSON");
    assert_eq!(parsed["variant"], "Reject");
}

// ===== H1: run-lane exit codes must distinguish infrastructure vs input vs reject =====

/// H1: `run-lane` on a missing lane must return exit code 3 (InputError),
/// not 0 (pass) or 1 (reject).
#[test]
fn h1_run_lane_missing_lane_exits_3() {
    let (code, stdout, stderr) = run(&["run-lane", "nonexistent-lane"]);
    assert_eq!(code, 3, "missing lane must exit 3 (InputError), got {code}; stderr: {stderr}");
    assert!(stdout.is_empty(), "InputError must not write stdout, got: {stdout}");
    assert_stderr_contains(&stderr, "unknown lane");
}

/// H1: `run-lane fmt` on a clean project must exit 0 (pass).
#[test]
fn h1_run_lane_clean_fmt_exits_0() {
    let temp = package("h1_fmt_clean", "pub fn value() -> u8 {\n    1\n}\n", "fn main() {}\n")
        .expect("temp package must be created");
    let (code, stdout, stderr) = run_in(temp.path(), &["run-lane", "fmt"]);
    assert_eq!(code, 0, "clean fmt lane must exit 0, stderr: {stderr}");
    assert!(stdout.is_empty(), "fmt lane must not write stdout, got: {stdout}");
}

/// H1: `run-lane clippy` on a clean project (with lockfile) must exit 0 (pass).
#[test]
fn h1_run_lane_clean_clippy_exits_0() {
    let temp = package(
        "h1_clippy_clean",
        "#[must_use]\npub const fn value() -> u8 {\n    1\n}\n",
        "fn main() {}\n",
    )
    .expect("temp package must be created");
    // Generate lockfile for clippy's --frozen mode
    drop(
        std::process::Command::new("cargo")
            .current_dir(temp.path())
            .arg("generate-lockfile")
            .output()
            .expect("cargo generate-lockfile must succeed"),
    );
    let (code, stdout, stderr) = run_in(temp.path(), &["run-lane", "clippy"]);
    assert_eq!(code, 0, "clean clippy lane must exit 0, stderr: {stderr}");
    assert!(stdout.is_empty(), "clippy lane must not write stdout, got: {stdout}");
}

/// H1: aggregate/check on empty workspace must exit 1 (reject), not 0 (pass).
/// The exit code must distinguish reject from pass.
#[test]
fn h1_aggregate_empty_workspace_exits_1_not_0() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (code, stdout, stderr) =
        run_in(workspace.path(), &["aggregate", "--emit", "json", "--scope", "edit"]);
    assert_eq!(
        code, 1,
        "empty workspace aggregate must exit 1 (reject), not 0 (pass); stderr: {stderr}",
    );
    let report: serde_json::Value =
        serde_json::from_str(&stdout).expect("aggregate must emit JSON report");
    assert_eq!(report["variant"], "Reject");
    // Verify gate_failures > 0 to confirm it is reject, not pass
    assert!(
        report["gate_failures"].as_array().is_some_and(|arr| !arr.is_empty()),
        "reject report must have gate_failures",
    );
}

/// H1: `titania-check check --scope edit` exit code must equal aggregate's exit code.
/// Both should map the same ReportStatus → exit code.
#[cfg(unix)]
#[test]
fn h1_check_and_aggregate_exit_codes_match() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (check_code, _, check_stderr) = run_in(workspace.path(), &["check", "--scope", "edit"]);
    let (agg_code, _, agg_stderr) = run_in(workspace.path(), &["aggregate", "--scope", "edit"]);
    assert_eq!(
        check_code, agg_code,
        "check and aggregate must produce the same exit code; check={check_code}, aggregate={agg_code}; check_stderr: {check_stderr}; agg_stderr: {agg_stderr}",
    );
}

// ===== H1: lane infrastructure Failure must map to InternalError(4) =====

/// H1: a lane `Failure` disposition must surface as exit code 4 (InternalError),
/// not the raw `LaneExit::Failure` value of 3 (which is `InputError` in the
/// v1-spec §12 exit-code taxonomy). This proves `map_lane_exit` re-maps
/// Failure → 4 at the CLI boundary.
///
/// We trigger a real `LaneExit::Failure` by running `run-lane fmt` in a Cargo
/// project whose `.titania/out/edit/` directory is read-only: cargo fmt itself
/// succeeds (Clean outcome), but `write_lane_artifact` fails when it tries to
/// write the temp artifact file. `execute_lane_checked` then returns Err, and
/// `execute_lane` synthesizes `LaneExit::Failure`. `map_lane_exit` must route
/// that to exit 4.
// chmod 0o555 on the artifact directory is Unix-only (see `make_read_only`).
#[cfg(unix)]
#[test]
fn h1_lane_failure_maps_to_internal_error() {
    let temp = package("h1_lane_failure", "pub fn value() -> u8 {\n    1\n}\n", "fn main() {}\n")
        .expect("temp package must be created");

    // Pre-create the artifact directory and make it read-only so the lane's
    // atomic-write step fails. create_dir_all is a no-op on existing dirs, so
    // the failure surfaces at the temp-file write step inside write_lane_artifact.
    let artifact_dir = temp.path().join(".titania").join("out").join("edit");
    std::fs::create_dir_all(&artifact_dir).expect("artifact dir must be pre-created");
    make_read_only(&artifact_dir);

    let (code, stdout, stderr) = run_in(temp.path(), &["run-lane", "fmt"]);
    assert_eq!(
        code, 4,
        "LaneExit::Failure must map to InternalError(4), got {code}; stderr: {stderr}",
    );
    assert!(
        stdout.is_empty(),
        "Failure path must not write stdout (it goes to stderr as a diagnostic), got: {stdout}",
    );
    assert!(!stderr.is_empty(), "Failure path must write a diagnostic to stderr, got empty stderr",);
}

/// Mark a directory read-only (mode 0555) on Unix so writes inside it fail.
#[cfg(unix)]
fn make_read_only(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms =
        std::fs::metadata(path).expect("readonly dir metadata must be readable").permissions();
    perms.set_mode(0o555);
    std::fs::set_permissions(path, perms).expect("dir must be made read-only");
}

/// No-op marker on non-Unix hosts (the test is Unix-gated in practice).
#[cfg(not(unix))]
fn make_read_only(_path: &Path) {}

// ===== --help / -h / help routing =====

/// `--help` prints usage to stdout and exits 0.
#[test]
fn help_long_flag_prints_usage_and_exits_zero() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (code, stdout, stderr) = run_in(workspace.path(), &["--help"]);
    assert_eq!(code, 0, "--help must exit 0, got {code}; stderr: {stderr}");
    assert!(stderr.is_empty(), "--help must not write stderr, got: {stderr}");
    assert!(stdout.contains("titania-check"), "usage must mention binary name: {stdout}");
    assert!(stdout.contains("check"), "usage must list check subcommand: {stdout}");
    assert!(stdout.contains("run-lane"), "usage must list run-lane subcommand: {stdout}");
    assert!(stdout.contains("aggregate"), "usage must list aggregate subcommand: {stdout}");
    assert!(stdout.contains("doctor"), "usage must list doctor subcommand: {stdout}");
    assert!(stdout.contains("explain"), "usage must list explain subcommand: {stdout}");
    assert!(stdout.contains("--scope"), "usage must mention --scope flag: {stdout}");
    assert!(stdout.contains("--emit"), "usage must mention --emit flag: {stdout}");
    assert!(stdout.contains("--out"), "usage must mention --out flag: {stdout}");
    assert!(
        stdout.contains("Check/aggregate output"),
        "usage must state check/aggregate output format: {stdout}",
    );
}

/// `-h` is an alias for `--help`.
#[test]
fn help_short_flag_prints_usage_and_exits_zero() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (code, stdout, stderr) = run_in(workspace.path(), &["-h"]);
    assert_eq!(code, 0, "-h must exit 0, got {code}; stderr: {stderr}");
    assert!(stderr.is_empty(), "-h must not write stderr, got: {stderr}");
    assert!(stdout.contains("titania-check"), "-h usage must mention binary name: {stdout}");
}

/// `help` (bare subcommand) prints usage and exits 0.
#[test]
fn help_bare_subcommand_prints_usage_and_exits_zero() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (code, stdout, stderr) = run_in(workspace.path(), &["help"]);
    assert_eq!(code, 0, "help must exit 0, got {code}; stderr: {stderr}");
    assert!(stderr.is_empty(), "help must not write stderr, got: {stderr}");
    assert!(stdout.contains("titania-check"), "help usage must mention binary name: {stdout}");
}

/// `<subcommand> --help` prints usage and exits 0 for each subcommand.
#[test]
fn help_after_check_subcommand_prints_usage_and_exits_zero() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (code, stdout, stderr) = run_in(workspace.path(), &["check", "--help"]);
    assert_eq!(code, 0, "check --help must exit 0, got {code}; stderr: {stderr}");
    assert!(stderr.is_empty(), "check --help must not write stderr, got: {stderr}");
    assert!(stdout.contains("titania-check"), "check --help must print usage: {stdout}");
}

/// `aggregate --help` prints usage and exits 0 (and does NOT require --scope).
#[test]
fn help_after_aggregate_subcommand_short_circuits_scope_requirement() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let (code, stdout, stderr) = run_in(workspace.path(), &["aggregate", "--help"]);
    assert_eq!(code, 0, "aggregate --help must exit 0, got {code}; stderr: {stderr}");
    assert!(
        stderr.is_empty(),
        "aggregate --help must not write stderr (no AggregateScopeRequired), got: {stderr}",
    );
    assert!(stdout.contains("titania-check"), "aggregate --help must print usage: {stdout}");
}

// ===== Check → Moon dispatch (spec §12, §13) =====

/// `Command::Check` with no moon binary on PATH (TITANIA_MOON_BIN points at a
/// missing path) must surface InputError(3) with the install hint.
// TITANIA_MOON_BIN pointed at an absent path; `Command::Check` then spawns
// Moon and surfaces `MoonSpawnError::NotFound`. The Unix-only assertion
// shape (exit 3 + install hint) validates the Moon-not-found diagnostic;
// non-Unix hosts have no canonical Moon install path or hint to compare
// against, so the positive scenario stays Unix-gated.
#[cfg(unix)]
#[test]
fn check_with_missing_moon_binary_yields_input_error() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let mut cmd = Command::new(binary());
    let _ = cmd.current_dir(workspace.path());
    let _ = cmd.args(&["--scope", "edit"]);
    let _ = cmd.stdout(Stdio::piped());
    let _ = cmd.stderr(Stdio::piped());
    // Point at a path that does not exist → spawn fails with NotFound →
    // MoonSpawnError::NotFound → InputError(3).
    let _ = cmd.env("TITANIA_MOON_BIN", "/nonexistent/titania-moon-stub-missing");
    let output = cmd.output().expect("failed to spawn titania-check");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert_eq!(
        code, 3,
        "missing moon binary must surface as InputError(3), got {code}; stderr: {stderr}",
    );
    assert!(stdout.is_empty(), "InputError must not write stdout, got: {stdout}");
    assert!(
        stderr.contains("Moon CI/CD is required"),
        "stderr must contain install hint, got: {stderr}",
    );
}

/// `Command::Check` must invoke the moon binary (not skip directly to
/// aggregate). Proven by setting TITANIA_MOON_BIN to a recording stub that
/// writes a marker file when invoked: the marker must exist after `check`
/// returns. `aggregate` must NOT invoke moon (marker stays absent).
// Recording stub is a `/bin/sh` shell script + `chmod 0o755`; Unix-only.
#[cfg(unix)]
#[test]
fn check_invokes_moon_aggregate_does_not() {
    let workspace = tempfile::tempdir().expect("tempdir must be created");
    let marker = workspace.path().join("moon_invoked marker.txt");
    let stub = write_recording_stub(workspace.path(), &marker);

    // Check path: moon must be invoked.
    let mut check_cmd = Command::new(binary());
    let _ = check_cmd.current_dir(workspace.path());
    let _ = check_cmd.args(&["check", "--scope", "edit"]);
    let _ = check_cmd.stdout(Stdio::piped());
    let _ = check_cmd.stderr(Stdio::piped());
    let _ = check_cmd.env("TITANIA_MOON_BIN", &stub);
    drop(check_cmd.output().expect("failed to spawn titania-check"));
    assert!(
        marker.exists(),
        "check must invoke moon (marker file should exist at {})",
        marker.display(),
    );

    // Remove the marker, then run aggregate: moon must NOT be invoked.
    drop(std::fs::remove_file(&marker));
    let mut agg_cmd = Command::new(binary());
    let _ = agg_cmd.current_dir(workspace.path());
    let _ = agg_cmd.args(&["aggregate", "--scope", "edit"]);
    let _ = agg_cmd.stdout(Stdio::piped());
    let _ = agg_cmd.stderr(Stdio::piped());
    let _ = agg_cmd.env("TITANIA_MOON_BIN", &stub);
    drop(agg_cmd.output().expect("failed to spawn titania-check"));
    assert!(
        !marker.exists(),
        "aggregate must NOT invoke moon (marker file should be absent at {})",
        marker.display(),
    );
}

/// Write a tiny POSIX shell stub that touches `marker` on invocation, then
/// exits 0. Used to prove the check→moon spawn path is taken.
// POSIX shell (`/bin/sh`) + chmod 0o755. The only caller is the Unix-gated
// `check_invokes_moon_aggregate_does_not`; gating the helper keeps Windows
// compilation free of the shell-script and `chmod` literals.
#[cfg(unix)]
fn write_recording_stub(dir: &Path, marker: &Path) -> String {
    let stub_path = dir.join("moon-recording-stub.sh");
    let script = format!("#!/bin/sh\ntouch '{}'\nexit 0\n", marker.display());
    std::fs::write(&stub_path, script).expect("recording stub script must be written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms =
            std::fs::metadata(&stub_path).expect("stub metadata must be readable").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&stub_path, perms).expect("stub must be made executable");
    }
    stub_path.to_str().expect("stub path must be valid UTF-8").to_owned()
}

// ---------------------------------------------------------------------------
// setup-hermetic subcommand tests
// ---------------------------------------------------------------------------

/// `setup-hermetic` on a fresh temp dir must:
/// - exit 0
/// - print eval-able export lines on stdout
/// - create both symlinks that resolve to real directories
#[cfg(unix)]
#[test]
fn setup_hermetic_creates_symlinks_and_prints_exports() {
    let temp = TempDir::new().expect("temp dir must be created");
    let hermetic_dir = temp.path().join(".titania").join("hermetic");

    let (code, stdout, stderr) = run_in(temp.path(), &["setup-hermetic"]);
    assert_eq!(code, 0, "setup-hermetic must exit 0, stderr: {stderr}");

    assert!(
        stdout.contains("export CARGO_HOME="),
        "stdout must contain CARGO_HOME export: {stdout}"
    );
    assert!(
        stdout.contains("export RUSTUP_HOME="),
        "stdout must contain RUSTUP_HOME export: {stdout}"
    );

    let cargo_link = hermetic_dir.join("cargo-home");
    let rustup_link = hermetic_dir.join("rustup-home");
    assert!(cargo_link.is_symlink(), "cargo-home must be a symlink");
    assert!(rustup_link.is_symlink(), "rustup-home must be a symlink");
    assert!(cargo_link.exists(), "cargo-home symlink must resolve");
    assert!(rustup_link.exists(), "rustup-home symlink must resolve");
}

/// Re-running `setup-hermetic` must be idempotent — existing correct symlinks
/// are left as-is, and the command still exits 0.
#[cfg(unix)]
#[test]
fn setup_hermetic_is_idempotent() {
    let temp = TempDir::new().expect("temp dir must be created");

    let (code1, _, stderr1) = run_in(temp.path(), &["setup-hermetic"]);
    assert_eq!(code1, 0, "first run must exit 0, stderr: {stderr1}");

    let (code2, _, stderr2) = run_in(temp.path(), &["setup-hermetic"]);
    assert_eq!(code2, 0, "second run must exit 0, stderr: {stderr2}");
}

/// `setup-hermetic extra-arg` must reject with InputError (exit 3).
#[test]
fn setup_hermetic_rejects_extra_arguments() {
    let (code, _, stderr) = run(&["setup-hermetic", "unexpected-arg"]);
    assert_eq!(code, 3, "extra arg must exit 3 (InputError), stderr: {stderr}");
    assert!(
        stderr.contains("unexpected") || stderr.contains("ExtraArgument"),
        "stderr must mention the unexpected argument: {stderr}"
    );
}

/// `link_if_needed` must refuse to overwrite a real (non-symlink) directory.
#[cfg(unix)]
#[test]
fn setup_hermetic_refuses_real_directory_at_link_path() {
    let temp = TempDir::new().expect("temp dir must be created");
    let hermetic_dir = temp.path().join(".titania").join("hermetic");
    std::fs::create_dir_all(hermetic_dir.join("cargo-home")).expect("create real dir");

    let (code, _, stderr) = run_in(temp.path(), &["setup-hermetic"]);
    assert!(
        code >= 4,
        "must exit >=4 (InternalError) when real dir blocks link path, got {code}, stderr: {stderr}"
    );
}
