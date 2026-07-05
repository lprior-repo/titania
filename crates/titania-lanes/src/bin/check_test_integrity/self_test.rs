use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use titania_core::TargetProject;
use titania_lanes::{CommandIn, LaneExit};

use super::{TestIntegrityError, TestIntegrityRules, Vcs, check};

pub(super) fn run() -> LaneExit {
    match run_fixtures() {
        Ok(()) => super::lane_after_stderr(
            &super::write_stderr_line(format_args!("SelfTest:check-test-integrity:PASS")),
            LaneExit::Clean,
        ),
        Err(error) => super::lane_after_stderr(
            &super::write_stderr_line(format_args!("SelfTest:check-test-integrity:FAIL {error}")),
            LaneExit::Violations,
        ),
    }
}

/// Run all in-process self-test fixtures.
///
/// # Errors
///
/// Returns fixture setup, scanner, or cleanup failures with context.
fn run_fixtures() -> Result<(), TestIntegrityError> {
    let scratch = scratch_dir()?;
    let rules = TestIntegrityRules::new().map_err(|error| format!("rule id config: {error}"))?;
    let result = with_initialized_repo(&scratch, |target| {
        assert_clean_fixture(target, &rules)?;
        assert_untracked_ignored_fixture(target, &rules)
    });
    let cleanup = fs::remove_dir_all(&scratch)
        .map_err(|error| TestIntegrityError::from(format!("cleanup failed: {error}")));
    result.and(cleanup)
}

/// Create a scratch git repository and run a fixture inside it.
///
/// # Errors
///
/// Returns filesystem, target construction, git setup, or fixture callback
/// failures.
fn with_initialized_repo<F>(root: &Path, f: F) -> Result<(), TestIntegrityError>
where
    F: FnOnce(&TargetProject) -> Result<(), TestIntegrityError>,
{
    fs::create_dir_all(root).map_err(|error| format!("create scratch repo failed: {error}"))?;
    fs::write(root.join("Cargo.toml"), "[workspace]\nmembers=[]\n")
        .map_err(|error| format!("write Cargo.toml failed: {error}"))?;
    let target = TargetProject::try_from_path(root)
        .map_err(|error| format!("target project construction failed: {error}"))?;
    run_git(&target, &["init", "-q"])?;
    run_git(&target, &["config", "user.email", "lane@example.invalid"])?;
    run_git(&target, &["config", "user.name", "Lane Test"])?;
    run_git(&target, &["add", "Cargo.toml"])?;
    run_git(&target, &["commit", "-q", "-m", "base"])?;
    f(&target)
}

/// Assert the empty fixture repository is clean.
///
/// # Errors
///
/// Returns scanner errors or a non-clean fixture result.
fn assert_clean_fixture(
    target: &TargetProject,
    rules: &TestIntegrityRules,
) -> Result<(), TestIntegrityError> {
    match check(target, "HEAD", Vcs::Git, rules)? {
        0 => Ok(()),
        code => Err(TestIntegrityError::from(format!("clean fixture returned {code}"))),
    }
}

/// Assert untracked ignored tests are reported as integrity violations.
///
/// # Errors
///
/// Returns filesystem errors, scanner errors, or an unexpected scanner result.
fn assert_untracked_ignored_fixture(
    target: &TargetProject,
    rules: &TestIntegrityRules,
) -> Result<(), TestIntegrityError> {
    let tests_dir = target.as_std_path().join("tests");
    fs::create_dir_all(&tests_dir).map_err(|error| format!("create tests dir failed: {error}"))?;
    fs::write(
        tests_dir.join("untracked_ignored.rs"),
        "#[test]\n#[ignore]\nfn covered_behavior() {\n    assert_eq!(2 + 2, 4);\n}\n",
    )
    .map_err(|error| format!("write untracked test failed: {error}"))?;
    match check(target, "HEAD", Vcs::Git, rules)? {
        1 => Ok(()),
        code => Err(TestIntegrityError::from(format!("untracked ignored fixture returned {code}"))),
    }
}

/// Run one git command in the scratch fixture repository.
///
/// # Errors
///
/// Returns command construction, spawn, or non-zero exit failures.
fn run_git(target: &TargetProject, args: &[&str]) -> Result<(), TestIntegrityError> {
    let mut command =
        CommandIn::new(target, "git").map_err(|error| format!("git {args:?} invalid: {error}"))?;
    let _ = command.inherit_env();
    let _ = command.args(args);
    let status = command
        .run_status_raw()
        .map_err(|error| format!("git {args:?} failed to start: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(TestIntegrityError::from(format!("git {args:?} exited with {status}")))
    }
}

/// Build a unique scratch directory path.
///
/// # Errors
///
/// Returns an error if system time is before the Unix epoch.
fn scratch_dir() -> Result<PathBuf, TestIntegrityError> {
    let root = std::env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system time before epoch: {error}"))?
        .as_nanos();
    Ok(root.join(format!("titania-check-test-integrity-{stamp}")))
}
