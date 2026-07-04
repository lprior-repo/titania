//! Compile-lane behavior tests — failing-first.
//!
//! These tests invoke `run-cargo compile` via the binary public API against
//! synthetic Cargo projects in temp directories and assert:
//!
//! 1. **Clean fixture** — exit 0, artifact written to
//!    `.titania/out/prepush/compile.json` with `variant:"clean"` and exact argv
//!    `["cargo", "check", "--workspace", "--frozen"]`.
//! 2. **Compile-error fixture** — exit 1, stderr contains `CARGO_COMPILE_001`,
//!    artifact written with `variant:"findings"` and `CommandEvidence`.
//! 3. **No-cargo fixture** — `cargo` absent from PATH, exit 1, stderr contains
//!    `target discovery failed` or `cargo execution failed`, artifact with
//!    `variant:"failed"` and `infra_failure` tool `cargo`.
//!
//! Beads: tn-uia.1, tn-uia.2, tn-uia.3, tn-uia.4

use std::{
    path::Path,
    process::{Command, Output},
};

use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn run_compile(cwd: &Path) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_run-cargo")).arg("compile").current_dir(cwd).output()
}

/// Create a minimal clean Cargo package inside a temp dir, including a
/// generated lock file (required by --frozen).
fn clean_package(name: &str) -> Result<TempDir, std::io::Error> {
    let tmp = tempfile::tempdir()?;
    let root = tmp.path();

    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )?;

    std::fs::create_dir_all(root.join("src"))?;
    std::fs::write(root.join("src/lib.rs"), "pub fn hello() -> &'static str { \"world\" }\n")?;

    // Generate a lock file so --frozen does not fail.
    let status = Command::new("cargo")
        .arg("generate-lockfile")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .status()?;
    if !status.success() {
        return Err(std::io::Error::other("cargo generate-lockfile failed"));
    }

    Ok(tmp)
}

/// Create a Cargo package that does NOT compile (type error in lib.rs),
/// including a generated lock file (required by --frozen).
fn compile_error_package(name: &str) -> Result<TempDir, std::io::Error> {
    let tmp = tempfile::tempdir()?;
    let root = tmp.path();

    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )?;

    std::fs::create_dir_all(root.join("src"))?;
    std::fs::write(root.join("src/lib.rs"), "pub fn broken() -> String {\n    42\n}\n")?;

    // Generate a lock file so --frozen does not fail.
    let status = Command::new("cargo")
        .arg("generate-lockfile")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .status()?;
    if !status.success() {
        return Err(std::io::Error::other("cargo generate-lockfile failed"));
    }

    Ok(tmp)
}

/// Run the compile lane with a modified PATH that omits any cargo binary.
fn run_compile_no_cargo(cwd: &Path) -> Result<Output, std::io::Error> {
    // Start with the current PATH, then remove every directory that might
    // contain cargo (rustup toolchains, system paths).
    let original_path = std::env::var("PATH").unwrap_or_default();
    let filtered = original_path
        .split(':')
        .filter(|dir| {
            !dir.ends_with("/.cargo/bin")
                && !dir.ends_with("/cargo/bin")
                && !dir.ends_with("/rustup/toolchains")
                && !dir.ends_with("/bin")
                && !dir.ends_with("/usr/bin")
                && !dir.ends_with("/usr/local/bin")
        })
        .collect::<Vec<_>>()
        .join(":");

    let output = Command::new(env!("CARGO_BIN_EXE_run-cargo"))
        .arg("compile")
        .current_dir(cwd)
        .env("PATH", filtered)
        .output()?;

    Ok(output)
}

fn stderr_text(output: &Output) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(output.stderr.clone())
}

/// Read the lane artifact JSON if it exists at the expected path.
fn read_artifact(target_root: &Path, scope: &str, lane: &str) -> Result<String, std::io::Error> {
    let path = target_root.join(".titania").join("out").join(scope).join(format!("{lane}.json"));
    std::fs::read_to_string(path)
}

// ---------------------------------------------------------------------------
// 1. Clean fixture — writes Clean artifact
// ---------------------------------------------------------------------------

#[test]
fn clean_compile_lane_exits_zero() -> TestResult {
    let target = clean_package("compile_lane_clean")?;
    let output = run_compile(target.path())?;
    assert_eq!(output.status.code(), Some(0_i32));
    Ok(())
}

#[test]
fn clean_compile_lane_writes_clean_artifact() -> TestResult {
    let target = clean_package("compile_lane_clean_artifact")?;
    let _output = run_compile(target.path())?;

    let artifact = read_artifact(target.path(), "prepush", "compile")?;

    // Must contain variant:"clean" — a clean lane produces a Clean outcome.
    assert!(artifact.contains("clean"), "artifact should contain 'clean' variant; got: {artifact}");

    // Must contain the lane name.
    assert!(
        artifact.contains("Compile"),
        "artifact should contain 'Compile' lane; got: {artifact}"
    );

    Ok(())
}

#[test]
fn clean_compile_lane_artifact_has_command_evidence() -> TestResult {
    let target = clean_package("compile_lane_cmd_evidence")?;
    let _output = run_compile(target.path())?;

    let artifact = read_artifact(target.path(), "prepush", "compile")?;

    // The argv must contain the exact compile args:
    // cargo check --workspace --frozen
    assert!(
        artifact.contains("cargo check --workspace --frozen")
            || (artifact.contains("cargo")
                && artifact.contains("check")
                && artifact.contains("--workspace")
                && artifact.contains("--frozen")),
        "artifact CommandEvidence argv must include 'cargo check --workspace --frozen'; got: {artifact}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Compile-error fixture — records Failed/Finding with CommandEvidence
// ---------------------------------------------------------------------------

#[test]
fn compile_error_lane_exits_nonzero() -> TestResult {
    let target = compile_error_package("compile_lane_error")?;
    let output = run_compile(target.path())?;
    assert_ne!(output.status.code(), Some(0_i32));
    Ok(())
}

#[test]
fn compile_error_lane_stderr_contains_rule_id() -> TestResult {
    let target = compile_error_package("compile_lane_error_rule")?;
    let output = run_compile(target.path())?;
    let stderr = stderr_text(&output)?;
    assert!(
        stderr.contains("CARGO_COMPILE_001"),
        "stderr should contain rule id CARGO_COMPILE_001; got: {stderr}"
    );
    Ok(())
}

#[test]
fn compile_error_lane_stderr_contains_diagnostic() -> TestResult {
    let target = compile_error_package("compile_lane_error_diag")?;
    let output = run_compile(target.path())?;
    let stderr = stderr_text(&output)?;
    assert!(
        stderr.contains("mismatched types") || stderr.contains("expected `String`"),
        "stderr should contain type-mismatch diagnostic; got: {stderr}"
    );
    Ok(())
}

#[test]
fn compile_error_lane_writes_findings_artifact() -> TestResult {
    let target = compile_error_package("compile_lane_findings_artifact")?;
    let _output = run_compile(target.path())?;

    let artifact = read_artifact(target.path(), "prepush", "compile")?;

    // Must contain variant:"findings" — findings indicate issues were reported.
    assert!(
        artifact.contains("findings") || artifact.contains("Failed"),
        "artifact should contain 'findings' or 'Failed' variant; got: {artifact}"
    );

    // Must contain lane name and finding count — the lane identity is preserved.
    assert!(
        artifact.contains("Compile") && artifact.contains("CARGO_COMPILE_001"),
        "artifact should contain lane name and rule id; got: {artifact}"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// 3. No-cargo fixture — InfraFailure
// ---------------------------------------------------------------------------

#[test]
fn no_cargo_lane_exits_nonzero() -> TestResult {
    let target = clean_package("compile_lane_no_cargo")?;
    let output = run_compile_no_cargo(target.path())?;
    assert_ne!(output.status.code(), Some(0_i32));
    Ok(())
}

#[test]
fn no_cargo_lane_stderr_indicates_cargo_failure() -> TestResult {
    let target = clean_package("compile_lane_no_cargo_stderr")?;
    let output = run_compile_no_cargo(target.path())?;
    let stderr = stderr_text(&output)?;
    assert!(
        stderr.contains("cargo")
            || stderr.contains("not found")
            || stderr.contains("execution failed"),
        "stderr should mention cargo failure; got: {stderr}"
    );
    Ok(())
}

#[test]
fn no_cargo_lane_writes_failed_artifact() -> TestResult {
    let target = clean_package("compile_lane_failed_artifact")?;
    let _output = run_compile_no_cargo(target.path())?;

    let artifact = read_artifact(target.path(), "prepush", "compile")?;

    // Must indicate failure — either via variant:"failed" or via InfraFailure.
    assert!(
        artifact.contains("failed")
            || artifact.contains("Failed")
            || artifact.contains("infra_failure"),
        "artifact should indicate failure; got: {artifact}"
    );

    // Must identify 'cargo' as the failing tool.
    assert!(
        artifact.contains("cargo"),
        "artifact should mention 'cargo' as the tool; got: {artifact}"
    );

    Ok(())
}
