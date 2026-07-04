//! Test-lane behavior tests: verifying the `test` lane writes typed v1 LaneOutcome
//! artifacts under `.titania/out/prepush/test.json` with the correct `CommandEvidence`.
//!
//! These tests exercise the binary public interface (`run-cargo test`) against
//! two fixture projects (passing / failing-test) and assert the exact shape of
//! the written artifacts.

use std::{
    error::Error,
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

/// Run `run-cargo` with the given subcommands from the given working directory.
fn run_cargo(cwd: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_run-cargo")).args(args).current_dir(cwd).output()
}

/// Helper: read stderr bytes as text.
fn stderr_text(output: &Output) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(output.stderr.clone())
}

/// Path to the test-lane artifact produced for a project root.
fn test_artifact_path(project: &Path) -> std::path::PathBuf {
    project.join(".titania").join("out").join("prepush").join("test.json")
}

/// Build the expected argv vector for the `test` lane.
const fn expected_test_argv() -> [&'static str; 6] {
    ["cargo", "test", "--workspace", "--frozen", "--", "--test-threads=1"]
}

fn fixture_source(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("test").join(name)
}

fn materialize_fixture(name: &str) -> Result<TempDir, Box<dyn Error>> {
    let temp = tempfile::tempdir()?;
    let source = fixture_source(name);
    let _manifest_bytes = fs::copy(source.join("Cargo.toml"), temp.path().join("Cargo.toml"))?;
    let _lock_bytes = fs::copy(source.join("Cargo.lock"), temp.path().join("Cargo.lock"))?;
    fs::create_dir_all(temp.path().join("src"))?;
    let _lib_bytes =
        fs::copy(source.join("src").join("lib.rs"), temp.path().join("src").join("lib.rs"))?;
    let _main_bytes =
        fs::copy(source.join("src").join("main.rs"), temp.path().join("src").join("main.rs"))?;
    Ok(temp)
}

// ── Passing fixture ──────────────────────────────────────────────────────────

#[test]
fn test_lane_passing_writes_clean_artifact() -> TestResult {
    // Fixture lives in tests/fixtures/test/passing alongside this file.
    let fixture = materialize_fixture("passing")?;

    // Run the test lane.
    let output = run_cargo(fixture.path(), &["test"])?;
    assert_eq!(output.status.code(), Some(0_i32));
    assert_eq!(stderr_text(&output)?, "");

    // The artifact must exist.
    let artifact_path = test_artifact_path(fixture.path());
    assert!(artifact_path.exists(), "artifact must be written at {artifact_path:?}");

    // Parse and assert the JSON shape.
    let payload = fs::read_to_string(&artifact_path)?;
    let json: serde_json::Value = serde_json::from_str(&payload)?;

    // Top-level lane field.
    assert_eq!(json.get("lane").and_then(|v| v.as_str()), Some("Test"), "lane must be Test");

    // Outcome variant is "clean".
    assert_eq!(
        json.get("outcome").and_then(|v| v.get("variant")).and_then(|v| v.as_str()),
        Some("clean"),
        "variant must be \"clean\" for a passing test lane"
    );

    // CommandEvidence: exact argv.
    let argv = json
        .get("outcome")
        .and_then(|o| o.get("evidence"))
        .and_then(|e| e.get("command"))
        .and_then(|c| c.get("argv"))
        .and_then(|a| a.as_array())
        .expect("clean outcome must contain evidence.command.argv");

    let expected = expected_test_argv();
    assert_eq!(
        argv.len(),
        expected.len(),
        "argv length mismatch: got {} items, expected {}",
        argv.len(),
        expected.len()
    );
    for (i, (got, want)) in argv.iter().zip(expected.iter()).enumerate() {
        assert_eq!(got.as_str(), Some(*want), "argv[{i}] must be {want:?}, got {got:?}");
    }

    // Executable must be "cargo".
    let executable = json
        .get("outcome")
        .and_then(|o| o.get("evidence"))
        .and_then(|e| e.get("command"))
        .and_then(|c| c.get("executable"))
        .and_then(|v| v.as_str())
        .expect("command evidence must have executable");
    assert_eq!(executable, "cargo");

    // Exit status must be code 0.
    let code = json
        .get("outcome")
        .and_then(|o| o.get("evidence"))
        .and_then(|e| e.get("exit_status"))
        .and_then(|t| t.get("exited"))
        .and_then(|e| e.get("code"))
        .and_then(|v| v.as_i64())
        .expect("exit_status must contain exited.code");
    assert_eq!(code, 0, "exit code must be 0 for a clean test lane");

    Ok(())
}

// ── Failing-test fixture ─────────────────────────────────────────────────────

#[test]
fn test_lane_failing_writes_findings_artifact() -> TestResult {
    let fixture = materialize_fixture("failing-test")?;

    // Run the test lane; cargo exits 1 because the test fails.
    let output = run_cargo(fixture.path(), &["test"])?;
    assert_eq!(output.status.code(), Some(1_i32));

    // The artifact must still be written.
    let artifact_path = test_artifact_path(fixture.path());
    assert!(
        artifact_path.exists(),
        "artifact must be written at {artifact_path:?} even when tests fail"
    );

    // Parse and assert the JSON shape.
    let payload = fs::read_to_string(&artifact_path)?;
    let json: serde_json::Value = serde_json::from_str(&payload)?;

    // Top-level lane field.
    assert_eq!(json.get("lane").and_then(|v| v.as_str()), Some("Test"), "lane must be Test");

    // Outcome variant is "findings" (gate failure expressed via findings).
    assert_eq!(
        json.get("outcome").and_then(|v| v.get("variant")).and_then(|v| v.as_str()),
        Some("findings"),
        "variant must be \"findings\" for a failing test lane"
    );

    // Must contain at least one finding with rule CARGO_TEST_001.
    let findings = json
        .get("outcome")
        .and_then(|o| o.get("findings"))
        .and_then(|f| f.as_array())
        .expect("findings outcome must contain findings array");
    assert!(!findings.is_empty(), "findings array must not be empty for a failing test lane");

    // At least one finding must have rule_id CARGO_TEST_001.
    let has_test_rule = findings.iter().any(|f| {
        f.get("rule_id").and_then(|v| v.as_str()).map_or(false, |r| r == "CARGO_TEST_001")
    });
    assert!(has_test_rule, "at least one finding must have rule_id CARGO_TEST_001");

    // The finding message must reference the failed test.
    let finding = findings.get(0).expect("findings must have at least one entry");
    let message =
        finding.get("message").and_then(|v| v.as_str()).expect("finding must have a message");
    assert!(
        message.contains("test failed: tests::fails"),
        "finding message must reference the failing test: {message}"
    );

    Ok(())
}
