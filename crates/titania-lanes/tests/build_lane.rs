//! Build lane behavior tests for `run-cargo build`.
//!
//! These tests prove the run-cargo binary writes typed v1 LaneOutcome artifacts
//! under `.titania/out/<scope>/<lane>.json` via the atomic artifact writer contract.
//!
//! Beads: tn-uia.4
//!
//! CONTRACT (v1-spec §11):
//! - Build lane scope: `release` → artifact at `.titania/out/release/build.json`
//! - Command: `cargo build --workspace --release --frozen`
//! - Clean outcome: `variant:"clean"` with `CommandEvidence.argv`
//! - Findings outcome: `variant:"findings"` with `CARGO_BUILD_001` rule
//! - Infra failure: `variant:"failed"` with `failure.variant:"infra_failure"` tool `"cargo"`
//!
//! These tests should RED because production does not write typed artifacts yet,
//! not because of syntax/import mistakes.

use std::{
    error::Error,
    fs,
    path::Path,
    process::{Command, Output},
};

use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

fn run_cargo(cwd: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_run-cargo")).args(args).current_dir(cwd).output()
}

fn stderr_text(output: &Output) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(output.stderr.clone())
}

/// Build a minimal single-crate Cargo project in a temp directory.
fn make_target(name: &str, lib_rs: &str, main_rs: &str) -> Result<TempDir, std::io::Error> {
    let temp = tempfile::tempdir()?;
    fs::write(
        temp.path().join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",),
    )?;
    fs::create_dir_all(temp.path().join("src"))?;
    fs::write(temp.path().join("src/lib.rs"), lib_rs)?;
    fs::write(temp.path().join("src/main.rs"), main_rs)?;
    Ok(temp)
}

/// Generate a lockfile for the target project so `cargo build` is deterministic.
fn ensure_lockfile(temp: &TempDir) -> Result<(), std::io::Error> {
    let status = Command::new("cargo")
        .arg("generate-lockfile")
        .arg("--manifest-path")
        .arg(temp.path().join("Cargo.toml"))
        .status()?;
    if !status.success() {
        return Err(std::io::Error::other("cargo generate-lockfile failed"));
    }
    Ok(())
}

/// Expected artifact path for the build lane.
fn build_artifact_path(target: &Path) -> std::path::PathBuf {
    target.join(".titania").join("out").join("release").join("build.json")
}

// ---------------------------------------------------------------------------
// Test 1: release-build fixture writes Clean
// ---------------------------------------------------------------------------

/// **Contract:** A clean project running `run-cargo build` exits 0 with empty
/// stderr, and the artifact at `.titania/out/release/build.json` deserialises
/// to a v1 `LaneOutcome::Clean` with `lane:"Build"`, `variant:"clean"`, and
/// exact argv `["cargo","build","--workspace","--release","--frozen"]`.
#[test]
fn build_lane_clean_project_writes_clean_artifact() -> TestResult {
    let temp = make_target("build_clean_project", "pub fn value() -> u8 { 1 }", "fn main() {}")?;
    ensure_lockfile(&temp)?;

    let output = run_cargo(temp.path(), &["build"])?;
    assert_eq!(
        output.status.code(),
        Some(0_i32),
        "build must exit 0 for a clean project; stderr was: {}",
        stderr_text(&output).unwrap_or_default()
    );
    assert_eq!(stderr_text(&output)?, "", "clean build must produce empty stderr");

    // --- Artifact assertions ---
    let artifact_path = build_artifact_path(temp.path());
    assert!(
        artifact_path.exists(),
        "artifact must exist at {}; got: {:?}",
        artifact_path.display(),
        output.status
    );

    let content = fs::read_to_string(&artifact_path)?;

    // lane field
    assert!(
        content.contains("\"lane\": \"Build\""),
        "artifact must contain lane Build; got: {}",
        content
    );

    // variant
    assert!(
        content.contains("\"variant\": \"clean\""),
        "artifact must contain variant clean; got: {}",
        content
    );

    // exact argv
    assert!(
        content.contains("\"cargo\"")
            && content.contains("\"build\"")
            && content.contains("\"--workspace\"")
            && content.contains("\"--release\"")
            && content.contains("\"--frozen\""),
        "artifact must contain exact argv [cargo, build, --workspace, --release, --frozen]; got: {}",
        content
    );

    // evidence.command section must contain the argv
    assert!(
        content.contains("\"command\""),
        "clean artifact must contain command evidence; got: {}",
        content
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 2: build failure fixture records gate failure (findings)
// ---------------------------------------------------------------------------

/// **Contract:** A project with a compile error running `run-cargo build` exits
/// 1, emits `CARGO_BUILD_001` in stderr, and the artifact at
/// `.titania/out/release/build.json` contains `variant:"findings"` with the
/// `CARGO_BUILD_001` rule id.
#[test]
fn build_lane_failure_records_findings() -> TestResult {
    let temp =
        make_target("build_failure_project", "pub fn value() -> String { 1 }", "fn main() {}")?;
    ensure_lockfile(&temp)?;

    let output = run_cargo(temp.path(), &["build"])?;
    let stderr = stderr_text(&output)?;
    assert_eq!(
        output.status.code(),
        Some(1_i32),
        "build must exit 1 for a failing project; stderr was: {}",
        stderr
    );
    assert!(
        stderr.contains("CARGO_BUILD_001"),
        "stderr must contain rule CARGO_BUILD_001; got: {}",
        stderr
    );

    // --- Artifact assertions ---
    let artifact_path = build_artifact_path(temp.path());
    assert!(
        artifact_path.exists(),
        "artifact must exist at {} for findings",
        artifact_path.display()
    );

    let content = fs::read_to_string(&artifact_path)?;

    // lane field
    assert!(
        content.contains("\"lane\": \"Build\""),
        "artifact must contain lane Build; got: {}",
        content
    );

    // variant must be "findings" (not "clean")
    assert!(
        content.contains("\"variant\": \"findings\""),
        "artifact must contain variant findings for compile error; got: {}",
        content
    );

    // rule id must be present in findings
    assert!(
        content.contains("CARGO_BUILD_001"),
        "findings artifact must contain rule CARGO_BUILD_001; got: {}",
        content
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Test 3: PATH without cargo records InfraFailure
// ---------------------------------------------------------------------------

/// **Contract:** When `cargo` is not on PATH, `run-cargo build` exits 3 and
/// writes an artifact at `.titania/out/release/build.json` with
/// `variant:"failed"` and `failure.variant:"infra_failure"` with tool `"cargo"`.
#[test]
fn build_lane_path_without_cargo_records_infra_failure() -> TestResult {
    // Completely remove PATH to guarantee cargo is not found.
    let temp = make_target("build_no_cargo_project", "pub fn value() -> u8 { 1 }", "fn main() {}")?;

    let output = Command::new(env!("CARGO_BIN_EXE_run-cargo"))
        .arg("build")
        .current_dir(temp.path())
        .env("PATH", "")
        .output()?;

    let stderr = stderr_text(&output)?;
    assert_eq!(
        output.status.code(),
        Some(3_i32),
        "build with missing cargo must exit 3 (infra failure); stderr was: {}",
        stderr
    );

    // --- Artifact assertions ---
    let artifact_path = build_artifact_path(temp.path());
    assert!(
        artifact_path.exists(),
        "artifact must exist at {} for infra failure",
        artifact_path.display()
    );

    let content = fs::read_to_string(&artifact_path)?;

    // lane field
    assert!(
        content.contains("\"lane\": \"Build\""),
        "artifact must contain lane Build; got: {}",
        content
    );

    // variant must be "failed"
    assert!(
        content.contains("\"variant\": \"failed\""),
        "artifact must contain variant failed; got: {}",
        content
    );

    // failure must be infra_failure with tool "cargo"
    assert!(
        content.contains("\"infra_failure\""),
        "failed artifact must contain infra_failure; got: {}",
        content
    );
    assert!(
        content.contains("\"tool\": \"cargo\""),
        "infra_failure must name tool as cargo; got: {}",
        content
    );

    Ok(())
}
