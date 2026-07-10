//! Failing-first explain CLI tests for bead tn-ja8.1.
//!
//! Covers: known-rule metadata, unknown-rule exit-3, parser-invalid lowercase.
//!
//! Selective acceptance filter:
//! `cargo test -p titania-check explain`

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

// ---------------------------------------------------------------------------
// Known rule: FUNC_LOOPS_FOR must exit 0 and print metadata on stdout.
// ---------------------------------------------------------------------------

#[test]
fn explain_known_rule_exits_zero() {
    let (code, _stdout, stderr) = run(&["explain", "FUNC_LOOPS_FOR"]);
    assert_eq!(
        code, 0,
        "explain for known rule FUNC_LOOPS_FOR must exit 0, got {code}. stderr: {stderr}"
    );
}

#[test]
fn explain_known_rule_prints_metadata_on_stdout() {
    let (code, stdout, stderr) = run(&["explain", "FUNC_LOOPS_FOR"]);
    assert_eq!(code, 0, "must exit 0 to read stdout, got {code}. stderr: {stderr}");
    let output = stdout.trim();
    assert!(output.starts_with("FUNC_LOOPS_FOR"), "stdout must start with rule id: {output}");
    assert!(
        output.contains("Pattern: for $LOOP in $ITER { ... }"),
        "stdout must include spec pattern: {output}"
    );
    assert!(output.contains("Effect: Reject"), "stdout must include effect: {output}");
    assert!(output.contains("Repair: UseIteratorPipeline"), "stdout must include repair: {output}");
}

#[test]
fn explain_known_rule_prints_examples() {
    let (code, stdout, stderr) = run(&["explain", "FUNC_LOOPS_FOR"]);
    assert_eq!(code, 0, "must exit 0, got {code}. stderr: {stderr}");
    assert!(stdout.contains("Example violation:"), "missing violation heading: {stdout}");
    assert!(stdout.contains("for item in items { process(item); }"), "missing violation: {stdout}");
    assert!(stdout.contains("Example repair:"), "missing repair heading: {stdout}");
    assert!(
        stdout.contains("items.iter().for_each(|item| process(item));"),
        "missing repair: {stdout}"
    );
}

#[test]
fn explain_catalog_covers_non_clippy_lane_edge_ids() {
    [
        "DENY_MULTIPLE_VERSIONS",
        "DENY_UNKNOWN_REGISTRY",
        "DENY_UNKNOWN_GIT",
        "DENY_UNKNOWN",
        "DENY_INFRA_FAILURE",
        "DYLINT_INFRA_FAILURE",
        "FUNC_UNWRAP_USED",
        "FUNC_EXPECT_USED",
        "FUNC_UNWRAP_OR",
        "HOLZMAN_PANIC_PANIC",
        "HOLZMAN_PANIC_ASSERT",
        "HOLZMAN_PANIC_ASSERT_EQ",
        "HOLZMAN_PANIC_ASSERT_NE",
        "HOLZMAN_PANIC_TODO",
        "HOLZMAN_PANIC_UNIMPLEMENTED",
        "HOLZMAN_PANIC_UNREACHABLE",
        "HOLZMAN_PANIC_DBG",
        "CARGO_FMT_001",
        "CARGO_COMPILE_001",
        "CARGO_CLIPPY_001",
        "CARGO_TEST_001",
        "CARGO_BUILD_001",
        "COMPILE_SPLIT",
        "FN_LINE_LIMIT",
        "MUTANTS_RESIDUE",
        "SRC_LEN_LEDGER",
        "SRC_LINE_LIMIT",
        "BYPASS_CARGO_CONFIG_PARENT",
        "BYPASS_CARGO_CONFIG_PARSE_ERROR",
        "BYPASS_CARGO_CONFIG_READ_ERROR",
        "BYPASS_ENV_CARGO_ENCODED_RUSTFLAGS",
        "POLICY_EXCEPTION_EXPIRED",
        "POLICY_EXCEPTION_INVALID_FIELD",
        "POLICY_EXCEPTION_PARSE_ERROR",
        "POLICY_EXCEPTION_MISSING_FIELD",
        "PANIC_SURFACE_001",
        "FORBIDDEN_001",
    ]
    .iter()
    .for_each(|rule| {
        let (code, stdout, stderr) = run(&["explain", rule]);
        assert_eq!(code, 0, "{rule} must be explained. stderr: {stderr}");
        assert!(stderr.is_empty(), "{rule} must not emit stderr: {stderr}");
        assert!(stdout.contains(rule), "{rule} stdout must name rule: {stdout}");
    });
}

#[test]
fn explain_dynamic_clippy_rule_is_explainable() {
    let (code, stdout, stderr) = run(&["explain", "CLIPPY_NEEDLESS_BOOL"]);
    assert_eq!(code, 0, "dynamic clippy rule must exit 0. stderr: {stderr}");
    assert!(stderr.is_empty(), "dynamic clippy rule must not write stderr: {stderr}");
    assert!(stdout.contains("CLIPPY_NEEDLESS_BOOL"), "stdout must name rule: {stdout}");
    assert!(stdout.contains("clippy::needless_bool"), "stdout must name lint: {stdout}");
}

// ---------------------------------------------------------------------------
// Unknown syntactically valid rule: must exit 3 with "unknown rule ID".
// ---------------------------------------------------------------------------

#[test]
fn explain_unknown_rule_exits_three() {
    let (code, stdout, stderr) = run(&["explain", "DOES_NOT_EXIST"]);
    assert_eq!(code, 3, "unknown rule DOES_NOT_EXIST must exit 3, got {code}. stderr: {stderr}");
    assert!(stdout.is_empty(), "unknown-rule path must not write stdout, got: {stdout}");
}

#[test]
fn explain_unknown_rule_stderr_contains_unknown_rule_id() {
    let (code, _stdout, stderr) = run(&["explain", "DOES_NOT_EXIST"]);
    assert_eq!(code, 3, "must exit 3 to read stderr: {stderr}");
    assert_stderr_contains(&stderr, "unknown rule ID");
}

// ---------------------------------------------------------------------------
// Lowercase rule id is parser-invalid (RuleId requires uppercase).
// Must exit 3 with parser-error text (not the catalog unknown path).
// ---------------------------------------------------------------------------

#[test]
fn explain_lowercase_rule_id_is_parser_invalid() {
    let (code, stdout, stderr) = run(&["explain", "does_not_exist"]);
    assert_eq!(code, 3, "lowercase rule id must exit 3, got {code}. stderr: {stderr}");
    assert!(stdout.is_empty(), "parser-invalid path must not write stdout, got: {stdout}");
    // Should report the RuleId validation error, not "unknown rule ID".
    assert_stderr_contains(&stderr, "InputError:");
}

#[test]
fn explain_lowercase_rule_id_rejects_lowercase_characters() {
    let (code, _stdout, stderr) = run(&["explain", "func_loops_for"]);
    assert_eq!(code, 3, "must exit 3 for lowercase input: {stderr}");
    assert_stderr_contains(&stderr, "unknown rule ID");
}
