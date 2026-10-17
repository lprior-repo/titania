#![cfg(unix)]

use std::{error::Error, fs, process::Command};

use tempfile::TempDir;
use titania_core::{
    CommandEvidence, Digest, GateScope, Lane, LaneEvidence, LaneOutcome, ProcessTermination,
    TargetProject,
};
use titania_lanes::{CommandIn, artifact_writer::write_lane_artifact};
use titania_policy::PolicyDefaults;

type TestResult = Result<(), Box<dyn Error>>;

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

#[test]
fn external_target_policy_validates_temp_project() -> TestResult {
    let (tmp, _target) =
        make_target("external_test_project", "pub fn hello() -> &'static str { \"world\" }\n")?;

    let result = titania_lanes::try_from_path(tmp.path());

    let validated = result.expect("valid temp project must construct");
    assert!(validated.as_path().as_str().starts_with('/'), "target path must be absolute");

    assert!(
        validated.manifest_path().ends_with("Cargo.toml"),
        "manifest_path must end with Cargo.toml"
    );

    assert!(tmp.path().exists(), "temp dir must still exist after construction");

    Ok(())
}

#[test]
fn external_target_policy_rejects_missing_cargo_toml() -> TestResult {
    let tmp = tempfile::tempdir()?;

    let result = titania_lanes::try_from_path(tmp.path());

    assert!(
        matches!(result, Err(titania_core::TargetProjectError::NoCargoToml)),
        "expected NoCargoToml, got {:?}",
        result
    );

    Ok(())
}

#[test]
fn external_target_policy_rejects_regular_file() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let file = tmp.path().join("not-a-directory");
    fs::write(&file, b"not a project directory")?;

    let result = titania_lanes::try_from_path(&file);

    assert!(
        matches!(&result, Err(titania_core::TargetProjectError::NotADirectory)),
        "expected NotADirectory, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn external_target_policy_rejects_nonexistent_path() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let missing = tmp.path().join("missing-target");

    let result = titania_lanes::try_from_path(&missing);

    assert!(
        matches!(&result, Err(titania_core::TargetProjectError::NotFound)),
        "expected NotFound, got {:?}",
        result
    );
    Ok(())
}

#[test]
fn external_target_policy_command_inherits_target_cwd() -> TestResult {
    let (_tmp, target) = make_target("cwd_target", "pub fn value() -> u32 { 42 }\n")?;

    let mut cmd = CommandIn::new(&target, "/bin/sh")?;
    let out = cmd.arg("-c").arg("pwd").run()?;

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

#[test]
fn external_target_policy_command_env_is_passed() -> TestResult {
    let (_tmp, target) = make_target("env_target", "pub fn answer() -> u32 { 42 }\n")?;

    let mut cmd = CommandIn::new(&target, "/bin/sh")?;
    let _ = cmd.arg("-c").arg("echo \"$TITANIA_TEST_VAR\"");
    let _ = cmd.env("TITANIA_TEST_VAR", "policed-value");

    let out = cmd.run()?;
    let stdout = out.stdout_str()?.trim().to_owned();

    assert_eq!(stdout, "policed-value", "env var must be visible to subprocess");

    Ok(())
}

#[test]
fn external_target_policy_command_nonzero_exit_is_reported() -> TestResult {
    let (_tmp, target) = make_target("exit_target", "pub fn nothing() {}")?;

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

#[test]
fn external_target_policy_embedded_sources_include_required_files() -> TestResult {
    let defaults = PolicyDefaults::embedded();

    let sources: Vec<&str> = defaults.sources().iter().map(|s| s.as_str()).collect();
    assert!(sources.contains(&"AGENTS.md"), "embedded sources must include AGENTS.md (v1-spec §9)");
    assert!(
        sources.contains(&"v1-spec.md"),
        "embedded sources must include v1-spec.md (v1-spec §9)"
    );

    assert_eq!(defaults.schema_version, 1, "schema_version must be 1");
    assert_eq!(defaults.profile_name, "strict-ai", "profile must be strict-ai");
    assert!(defaults.embedded, "embedded flag must be true");

    Ok(())
}

#[test]
fn external_target_policy_artifact_lands_in_target() -> TestResult {
    let (tmp, _target) =
        make_target("artifact_target", "pub fn artifact_test() -> bool { true }\n")?;

    let evidence = stub_evidence("/bin/sh", &["sh", "-c", "cargo fmt --check"])?;
    let outcome = LaneOutcome::Clean { evidence };

    let artifact_path = write_lane_artifact(tmp.path(), GateScope::Edit, Lane::Fmt, &outcome)?;

    let expected_path = tmp.path().join(".titania").join("out").join("edit").join("fmt.json");
    assert_eq!(
        artifact_path,
        expected_path,
        "artifact must be at .titania/out/edit/fmt.json; got {}",
        artifact_path.display()
    );

    assert!(artifact_path.exists(), "artifact file must exist at {}", artifact_path.display());

    Ok(())
}

#[test]
fn external_target_policy_artifact_json_has_correct_fields() -> TestResult {
    let (tmp, _target) = make_target("json_target", "pub fn json_test() {}")?;

    let evidence = stub_evidence("/bin/sh", &["sh", "-c", "cargo fmt --check"])?;
    let outcome = LaneOutcome::Clean { evidence };

    let _unused = write_lane_artifact(tmp.path(), GateScope::Edit, Lane::Fmt, &outcome)?;

    let artifact_path = tmp.path().join(".titania").join("out").join("edit").join("fmt.json");
    let content = fs::read_to_string(&artifact_path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;

    assert_eq!(json["lane"].as_str(), Some("Fmt"), "artifact JSON must contain lane Fmt");
    assert!(json["outcome"].get("Clean").is_some(), "artifact JSON must contain Clean tag");

    Ok(())
}

#[test]
fn external_target_policy_artifact_path_is_deterministic() -> TestResult {
    let (tmp, _target) = make_target("deterministic_target", "pub fn det() {}")?;

    let evidence_a = stub_evidence("/bin/sh", &["sh", "-c", "cargo fmt --check"])?;
    let outcome_a = LaneOutcome::Clean { evidence: evidence_a };

    let evidence_b = stub_evidence("/bin/sh", &["sh", "-c", "cargo fmt --check"])?;
    let outcome_b = LaneOutcome::Clean { evidence: evidence_b };

    let path_a = write_lane_artifact(tmp.path(), GateScope::Edit, Lane::Fmt, &outcome_a)?;
    let path_b = write_lane_artifact(tmp.path(), GateScope::Edit, Lane::Fmt, &outcome_b)?;

    assert_eq!(path_a, path_b, "same scope+lane must produce the same artifact path");
    assert_eq!(
        path_a.file_name().map(|n| n.to_str()),
        Some(Some("fmt.json")),
        "artifact file must be named fmt.json"
    );

    Ok(())
}

#[test]
fn external_target_policy_artifact_requires_valid_target_root() -> TestResult {
    let tmp = tempfile::tempdir()?;
    let gone = tmp.path().join("deleted");
    fs::create_dir_all(&gone)?;
    fs::remove_dir_all(&gone)?;

    let evidence = stub_evidence("/bin/sh", &["sh", "-c", "cargo fmt --check"])?;
    let outcome = LaneOutcome::Clean { evidence };

    let result = write_lane_artifact(&gone, GateScope::Edit, Lane::Fmt, &outcome);

    assert!(result.is_err(), "write_lane_artifact must fail when target root does not exist");

    Ok(())
}

#[test]
fn external_target_policy_command_cwd_matches_target_path() -> TestResult {
    let (_tmp, target) = make_target("cwd_cross_target", "pub fn cross() -> u8 { 1 }\n")?;
    let target_path = target.as_std_path().to_path_buf();

    let mut cmd_in = CommandIn::new(&target, "/bin/sh")?;
    let cmd_in_output = cmd_in.arg("-c").arg("pwd").run()?.stdout_str()?.trim().to_owned();

    let std_output =
        Command::new("/bin/sh").arg("-c").arg("pwd").current_dir(&target_path).output()?;
    let std_stdout = String::from_utf8(std_output.stdout)?.trim().to_owned();

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
