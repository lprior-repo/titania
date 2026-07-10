#![cfg(unix)]
//! End-to-end tests for the external target evaluation path.
//!
//! Exercises the full chain: `TargetProject::try_from_path` on a temp Cargo
//! project, `CommandIn` execution rooted at that project, `PolicyDefaults::embedded()`
//! validation, and `write_lane_artifact` writing a `LaneOutcome` to the expected
//! `.titania/out/<scope>/<lane>.json` path.

use std::{error::Error, fs, process::Command};

use tempfile::TempDir;
use titania_core::{
    CommandEvidence, Digest, GateScope, Lane, LaneEvidence, LaneOutcome, ProcessTermination,
    TargetProject,
};
use titania_lanes::{CommandIn, artifact_writer::write_lane_artifact};
use titania_policy::PolicyDefaults;

type TestResult = Result<(), Box<dyn Error>>;

// ── Helpers ────────────────────────────────────────────────────────────────

/// Build a minimal single-crate Cargo project in a temp directory.
fn make_target(name: &str, lib_rs: &str) -> Result<(TempDir, TargetProject), Box<dyn Error>> {
    let tmp = tempfile::tempdir()?;
    let cargo_toml =
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fs::create_dir_all(tmp.path().join("src"))?;
    fs::write(tmp.path().join("Cargo.toml"), cargo_toml)?;
    fs::write(tmp.path().join("src/lib.rs"), lib_rs)?;
    let target = titania_lanes::try_from_path(tmp.path())?;
    Ok((tmp, target))
}

/// Build a `LaneEvidence` for a clean command execution.
fn stub_evidence(program: &str, argv: &[&str]) -> Result<LaneEvidence, Box<dyn Error>> {
    let executable = program.to_owned();
    let argv_vec: Vec<String> = argv.iter().map(|s| (*s).to_owned()).collect();
    let mut argv_final: Vec<String> = Vec::new();
    if argv_vec.is_empty() || argv_vec[0] != executable {
        argv_final.push(executable.clone());
        for a in argv_vec {
            argv_final.push(a);
        }
    } else {
        argv_final.extend(argv_vec);
    }
    let cmd_evidence = CommandEvidence::new(executable, argv_final.into_boxed_slice())?;
    let exit_status = ProcessTermination::Exited { code: 0 };
    let digest = Digest::from_bytes(b"stub-lane-result-digest");
    Ok(LaneEvidence::new(cmd_evidence, "titania-lanes/0.1.0".into(), exit_status, digest)?)
}

// ── Tests ──────────────────────────────────────────────────────────────────

/// **Contract:** `TargetProject::try_from_path` accepts a valid temp Cargo project
/// and rejects paths that are missing Cargo.toml.
#[test]
fn external_target_policy_validates_temp_project() -> TestResult {
    // Given: a minimal Cargo project in a temp directory.
    let (tmp, _target) =
        make_target("external_test_project", "pub fn hello() -> &'static str { \"world\" }\n")?;

    // When: we validate the target project.
    let result = titania_lanes::try_from_path(tmp.path());

    // Then: construction succeeds and the path is absolute.
    let validated = result.expect("valid temp project must construct");
    assert!(validated.as_path().as_str().starts_with('/'), "target path must be absolute");

    // And: manifest_path returns the correct Cargo.toml.
    assert!(
        validated.manifest_path().ends_with("Cargo.toml"),
        "manifest_path must end with Cargo.toml"
    );

    // And: the original TempDir is still alive, so the filesystem hasn't been
    // consumed by construction.
    assert!(tmp.path().exists(), "temp dir must still exist after construction");

    Ok(())
}

/// **Contract:** `TargetProject::try_from_path` returns `NoCargoToml` when
/// the directory has no `Cargo.toml`.
#[test]
fn external_target_policy_rejects_missing_cargo_toml() -> TestResult {
    // Given: an empty temp directory (no Cargo.toml).
    let tmp = tempfile::tempdir()?;

    // When: we try to construct a TargetProject from it.
    let result = titania_lanes::try_from_path(tmp.path());

    // Then: we get the exact typed error.
    assert!(
        matches!(result, Err(titania_core::TargetProjectError::NoCargoToml)),
        "expected NoCargoToml, got {:?}",
        result
    );

    Ok(())
}

/// **Contract:** `TargetProject::try_from_path` returns `NotADirectory` for a
/// regular file rather than collapsing it into `NotFound`.
#[test]
fn external_target_policy_rejects_regular_file() -> TestResult {
    // Given: a regular file path.
    let tmp = tempfile::tempdir()?;
    let file = tmp.path().join("not-a-directory");
    fs::write(&file, b"not a project directory")?;

    // When: we try to construct a target from the file.
    let result = titania_lanes::try_from_path(&file);

    // Then: the typed error preserves the filesystem distinction.
    assert!(
        matches!(&result, Err(titania_core::TargetProjectError::NotADirectory)),
        "expected NotADirectory, got {:?}",
        result
    );
    Ok(())
}

/// **Contract:** `TargetProject::try_from_path` returns `NotFound` for a path
/// that does not exist.
#[test]
fn external_target_policy_rejects_nonexistent_path() -> TestResult {
    // Given: a path that is absent from an existing temporary directory.
    let tmp = tempfile::tempdir()?;
    let missing = tmp.path().join("missing-target");

    // When: we try to construct a target from the missing path.
    let result = titania_lanes::try_from_path(&missing);

    // Then: the typed error preserves the missing-path distinction.
    assert!(
        matches!(&result, Err(titania_core::TargetProjectError::NotFound)),
        "expected NotFound, got {:?}",
        result
    );
    Ok(())
}

/// **Contract:** `CommandIn` executes with `current_dir` rooted at the target
/// project. Proved by running `pwd` and asserting the path matches the target.
#[test]
fn external_target_policy_command_inherits_target_cwd() -> TestResult {
    // Given: a temp Cargo project.
    let (_tmp, target) = make_target("cwd_target", "pub fn value() -> u32 { 42 }\n")?;

    // When: we run `pwd` via CommandIn.
    let mut cmd = CommandIn::new(&target, "/bin/sh")?;
    let out = cmd.arg("-c").arg("pwd").run()?;

    // Then: stdout is the absolute path of the target root.
    let stdout = out.stdout_str()?;
    let expected = target.as_std_path().display().to_string();
    let actual = stdout.trim();
    assert_eq!(
        actual, expected,
        "CommandIn cwd must match target root; expected={}, got={}",
        expected, actual
    );

    Ok(())
}

/// **Contract:** `CommandIn` passes environment variables into the subprocess,
/// proving it does not silently strip the environment.
#[test]
fn external_target_policy_command_env_is_passed() -> TestResult {
    // Given: a temp Cargo project.
    let (_tmp, target) = make_target("env_target", "pub fn answer() -> u32 { 42 }\n")?;

    // When: we set a custom env var and run a shell that echoes it.
    let mut cmd = CommandIn::new(&target, "/bin/sh")?;
    let _ = cmd.arg("-c").arg("echo \"$TITANIA_TEST_VAR\"");
    let _ = cmd.env("TITANIA_TEST_VAR", "policed-value");

    let out = cmd.run()?;
    let stdout = out.stdout_str()?.trim().to_owned();

    // Then: the variable was visible inside the subprocess.
    assert_eq!(stdout, "policed-value", "env var must be visible to subprocess");

    Ok(())
}

/// **Contract:** `CommandIn` returns `LaneError::NonZeroExit` when the
/// subprocess exits non-zero, preserving exit code and stderr.
#[test]
fn external_target_policy_command_nonzero_exit_is_reported() -> TestResult {
    // Given: a temp Cargo project.
    let (_tmp, target) = make_target("exit_target", "pub fn nothing() {}")?;

    // When: we run a shell script that exits with code 42.
    let mut cmd = CommandIn::new(&target, "/bin/sh")?;
    let _ = cmd.arg("-c").arg("echo error >&2; exit 42");

    let err = cmd.run();
    match err {
        Err(titania_lanes::LaneError::NonZeroExit { program, code, stderr }) => {
            assert_eq!(program, "/bin/sh");
            assert_eq!(code, Some(42));
            assert_eq!(stderr.trim(), "error");
        }
        other => panic!("expected NonZeroExit, got: {:?}", other),
    }

    Ok(())
}

/// **Contract:** `PolicyDefaults::embedded()` sources include both
/// `AGENTS.md` and `v1-spec.md` as required by v1-spec §9.
#[test]
fn external_target_policy_embedded_sources_include_required_files() -> TestResult {
    // When: we obtain embedded policy defaults.
    let defaults = PolicyDefaults::embedded();

    // Then: the sources contain both required policy files.
    let sources: Vec<&str> = defaults.sources().iter().map(|s| s.as_str()).collect();
    assert!(sources.contains(&"AGENTS.md"), "embedded sources must include AGENTS.md (v1-spec §9)");
    assert!(
        sources.contains(&"v1-spec.md"),
        "embedded sources must include v1-spec.md (v1-spec §9)"
    );

    // And: other embedded fields match v1-spec defaults.
    assert_eq!(defaults.schema_version, 1, "schema_version must be 1");
    assert_eq!(defaults.profile_name, "strict-ai", "profile must be strict-ai");
    assert!(defaults.embedded, "embedded flag must be true");

    Ok(())
}

/// **Contract:** `write_lane_artifact` writes to the correct `.titania/out/<scope>/<lane>.json`
/// path inside the target project root.
#[test]
fn external_target_policy_artifact_lands_in_target() -> TestResult {
    // Given: a temp Cargo project.
    let (tmp, _target) =
        make_target("artifact_target", "pub fn artifact_test() -> bool { true }\n")?;

    // When: we write a LaneOutcome::Clean for the Fmt lane under Edit scope.
    let evidence = stub_evidence("/bin/sh", &["sh", "-c", "cargo fmt --check"])?;
    let outcome = LaneOutcome::Clean { evidence };

    let artifact_path = write_lane_artifact(tmp.path(), GateScope::Edit, Lane::Fmt, &outcome)?;

    // Then: the artifact lands under .titania/out/edit/fmt.json.
    let expected_path = tmp.path().join(".titania").join("out").join("edit").join("fmt.json");
    assert_eq!(
        artifact_path,
        expected_path,
        "artifact must be at .titania/out/edit/fmt.json; got {}",
        artifact_path.display()
    );

    // And: the file actually exists on disk.
    assert!(artifact_path.exists(), "artifact file must exist at {}", artifact_path.display());

    Ok(())
}

/// **Contract:** The JSON artifact contains the correct `lane` field ("Fmt")
/// and externally tagged `outcome.Clean` field.
#[test]
fn external_target_policy_artifact_json_has_correct_fields() -> TestResult {
    // Given: a temp Cargo project.
    let (tmp, _target) = make_target("json_target", "pub fn json_test() {}")?;

    // When: we write a clean LaneOutcome.
    let evidence = stub_evidence("/bin/sh", &["sh", "-c", "cargo fmt --check"])?;
    let outcome = LaneOutcome::Clean { evidence };

    let _unused = write_lane_artifact(tmp.path(), GateScope::Edit, Lane::Fmt, &outcome)?;

    // Then: the JSON artifact has the expected fields.
    let artifact_path = tmp.path().join(".titania").join("out").join("edit").join("fmt.json");
    let content = fs::read_to_string(&artifact_path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;

    assert_eq!(json["lane"].as_str(), Some("Fmt"), "artifact JSON must contain lane Fmt");
    assert!(json["outcome"].get("Clean").is_some(), "artifact JSON must contain Clean tag");

    Ok(())
}

/// **Contract:** The artifact path is deterministic — same inputs always
/// produce the same path under `.titania/out/edit/fmt.json`.
#[test]
fn external_target_policy_artifact_path_is_deterministic() -> TestResult {
    // Given: a temp Cargo project.
    let (tmp, _target) = make_target("deterministic_target", "pub fn det() {}")?;

    // When: we write two artifacts with the same lane/scope/outcome.
    let evidence_a = stub_evidence("/bin/sh", &["sh", "-c", "cargo fmt --check"])?;
    let outcome_a = LaneOutcome::Clean { evidence: evidence_a };

    let evidence_b = stub_evidence("/bin/sh", &["sh", "-c", "cargo fmt --check"])?;
    let outcome_b = LaneOutcome::Clean { evidence: evidence_b };

    let path_a = write_lane_artifact(tmp.path(), GateScope::Edit, Lane::Fmt, &outcome_a)?;
    let path_b = write_lane_artifact(tmp.path(), GateScope::Edit, Lane::Fmt, &outcome_b)?;

    // Then: both paths are identical.
    assert_eq!(path_a, path_b, "same scope+lane must produce the same artifact path");
    assert_eq!(
        path_a.file_name().map(|n| n.to_str()),
        Some(Some("fmt.json")),
        "artifact file must be named fmt.json"
    );

    Ok(())
}

/// **Contract:** `write_lane_artifact` fails when the target root does not exist.
#[test]
fn external_target_policy_artifact_requires_valid_target_root() -> TestResult {
    // Given: a temp directory whose subdirectory we delete before calling the writer.
    let tmp = tempfile::tempdir()?;
    let gone = tmp.path().join("deleted");
    fs::create_dir_all(&gone)?;
    fs::remove_dir_all(&gone)?;

    // When: we write an artifact to a non-existent target root.
    let evidence = stub_evidence("/bin/sh", &["sh", "-c", "cargo fmt --check"])?;
    let outcome = LaneOutcome::Clean { evidence };

    let result = write_lane_artifact(&gone, GateScope::Edit, Lane::Fmt, &outcome);

    // Then: we get an error.
    assert!(result.is_err(), "write_lane_artifact must fail when target root does not exist");

    Ok(())
}

/// **Contract:** `CommandIn` proves `current_dir` is the target root by running
/// `pwd` via the standard library `Command` (outside `CommandIn`) and comparing.
/// This cross-validates that `CommandIn` actually roots at the target.
#[test]
fn external_target_policy_command_cwd_matches_target_path() -> TestResult {
    // Given: a temp Cargo project.
    let (_tmp, target) = make_target("cwd_cross_target", "pub fn cross() -> u8 { 1 }\n")?;
    let target_path = target.as_std_path().to_path_buf();

    // When: we run `pwd` both through CommandIn and directly via std::process::Command.
    let mut cmd_in = CommandIn::new(&target, "/bin/sh")?;
    let cmd_in_output = cmd_in.arg("-c").arg("pwd").run()?.stdout_str()?.trim().to_owned();

    let std_output =
        Command::new("/bin/sh").arg("-c").arg("pwd").current_dir(&target_path).output()?;
    let std_stdout = String::from_utf8(std_output.stdout)?.trim().to_owned();

    // Then: both methods produce the same cwd, which equals the target path.
    assert_eq!(
        cmd_in_output,
        target_path.display().to_string(),
        "CommandIn cwd must match target path"
    );
    assert_eq!(
        std_stdout,
        target_path.display().to_string(),
        "std::process::Command cwd must match target path"
    );
    assert_eq!(cmd_in_output, std_stdout, "CommandIn and std::process::Command must agree on cwd");

    Ok(())
}
