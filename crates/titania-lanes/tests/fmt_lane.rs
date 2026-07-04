//! fmt lane behavior tests — failing-first, artifact-focused.
//!
//! These tests invoke the `run-cargo` binary in temp Cargo projects and inspect
//! stdout/stderr/artifact files. They assert that the fmt lane writes typed v1
//! `LaneOutcome` JSON artifacts under `.titania/out/edit/fmt.json`.
//!
//! Beads: tn-uia.1, tn-uia.2, tn-uia.3, tn-uia.4

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

/// Helper: build a minimal Cargo project (lib + bin) and return the TempDir.
fn package(name: &str, lib_rs: &str, main_rs: &str) -> Result<TempDir, std::io::Error> {
    let temp = tempfile::tempdir()?;
    fs::write(
        temp.path().join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )?;
    fs::create_dir_all(temp.path().join("src"))?;
    fs::write(temp.path().join("src/lib.rs"), lib_rs)?;
    fs::write(temp.path().join("src/main.rs"), main_rs)?;
    Ok(temp)
}

fn stderr_text(output: &Output) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(output.stderr.clone())
}

/// Resolve the artifact path for the fmt lane in the edit scope.
fn fmt_artifact_path(root: &Path) -> std::path::PathBuf {
    root.join(".titania/out/edit/fmt.json")
}

// ===========================================================================
// tn-uia.1 — clean fixture: writes Clean to artifact
// ===========================================================================

#[test]
fn fmt_clean_project_writes_clean_artifact() -> TestResult {
    let temp = package("fmt_clean", "pub fn value() -> u8 {\n    1\n}\n", "fn main() {}\n")?;

    let output = run_cargo(temp.path(), &["fmt"])?;
    assert_eq!(output.status.code(), Some(0_i32));

    let artifact = fmt_artifact_path(temp.path());
    assert!(artifact.exists(), "fmt artifact must exist at .titania/out/edit/fmt.json");

    let payload = fs::read_to_string(&artifact)?;
    let json: serde_json::Value = serde_json::from_str(&payload)?;

    // Lane name must be "Fmt" (PascalCase).
    assert_eq!(json["lane"].as_str(), Some("Fmt"));

    // Outcome variant must be "clean".
    assert_eq!(json["outcome"]["variant"].as_str(), Some("clean"));

    // Command evidence must include the exact argv.
    let argv = &json["outcome"]["evidence"]["command"]["argv"];
    assert!(argv.is_array(), "evidence.command.argv must be a JSON array");
    let argv_strs: Vec<&str> = argv.as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
    assert!(
        argv_strs.contains(&"fmt")
            && argv_strs.contains(&"--all")
            && argv_strs.contains(&"--check"),
        "command argv must contain 'fmt', '--all', '--check'; got {argv_strs:?}"
    );

    Ok(())
}

// ===========================================================================
// tn-uia.2 — bad-format fixture: records findings in artifact
// ===========================================================================

#[test]
fn fmt_bad_project_writes_findings_artifact() -> TestResult {
    // Source that fails cargo fmt: missing whitespace around operators.
    let temp = package("fmt_bad", "pub fn value()->u8{1}\n", "fn main(){}\n")?;

    let output = run_cargo(temp.path(), &["fmt"])?;
    let stderr = stderr_text(&output)?;
    assert_eq!(output.status.code(), Some(1_i32));
    assert!(stderr.contains("CARGO_FMT_001"));

    let artifact = fmt_artifact_path(temp.path());
    assert!(artifact.exists(), "fmt artifact must exist even for failing lane");

    let payload = fs::read_to_string(&artifact)?;
    let json: serde_json::Value = serde_json::from_str(&payload)?;

    assert_eq!(json["lane"].as_str(), Some("Fmt"));
    assert_eq!(json["outcome"]["variant"].as_str(), Some("findings"));

    // Must have findings array with at least one entry.
    let findings = &json["outcome"]["findings"];
    assert!(findings.is_array(), "findings must be a JSON array");
    let count = findings.as_array().unwrap().len();
    assert!(count >= 1, "findings array must have >= 1 entry, got {count}");

    Ok(())
}

// ===========================================================================
// tn-uia.3 — PATH without cargo: records InfraFailure
// ===========================================================================

#[test]
fn fmt_without_cargo_records_infra_failure() -> TestResult {
    let temp = package("fmt_no_cargo", "pub fn value() -> u8 {\n    1\n}\n", "fn main() {}\n")?;

    let output = Command::new(env!("CARGO_BIN_EXE_run-cargo"))
        .arg("fmt")
        .current_dir(temp.path())
        .env("PATH", "")
        .output()?;

    // The binary should return non-zero when cargo is missing.
    assert!(
        !output.status.success(),
        "fmt lane should fail (non-zero exit) when cargo is unavailable"
    );

    // Artifact must be written with the failed outcome.
    let artifact = fmt_artifact_path(temp.path());
    assert!(artifact.exists(), "artifact must be written for infra failure");

    let payload = fs::read_to_string(&artifact)?;
    let json: serde_json::Value = serde_json::from_str(&payload)?;

    assert_eq!(json["lane"].as_str(), Some("Fmt"));
    assert_eq!(json["outcome"]["variant"].as_str(), Some("failed"));

    // Must carry infra_failure with tool = "cargo".
    let infra = &json["outcome"]["failure"]["infra_failure"];
    assert!(infra.is_object(), "outcome.failure.infra_failure must be an object");
    assert_eq!(infra["tool"].as_str(), Some("cargo"), "infra_failure.tool must be \"cargo\"");
    assert!(infra["reason"].as_str().is_some(), "infra_failure.reason must be present");

    Ok(())
}
