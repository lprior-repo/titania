use std::{error::Error, fs, process::Command};

use tempfile::TempDir;
use titania_core::TargetProject;

use super::{TestIntegrityRules, Vcs, check};

type TestError = Box<dyn Error>;
type InitializedRepo = (TempDir, TargetProject);

macro_rules! must {
    ($result:expr, $context:expr) => {
        must($result, $context)
    };
}

fn must<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => {
            let message = format!("{context}: {error}");
            assert_eq!(message, "", "{message}");
            std::process::abort();
        }
    }
}

fn run_git(root: &std::path::Path, args: &[&str]) -> Result<(), TestError> {
    let status = Command::new("git").args(args).current_dir(root).status()?;
    if status.success() { Ok(()) } else { Err(format!("git {args:?} failed with {status}").into()) }
}

fn initialized_repo() -> Result<InitializedRepo, TestError> {
    let temp = TempDir::new()?;
    let root = temp.path();
    fs::write(root.join("Cargo.toml"), "[workspace]\nmembers=[]\n")?;
    run_git(root, &["init", "-q"])?;
    run_git(root, &["config", "user.email", "lane@example.invalid"])?;
    run_git(root, &["config", "user.name", "Lane Test"])?;
    run_git(root, &["add", "Cargo.toml"])?;
    run_git(root, &["commit", "-q", "-m", "base"])?;
    let target = TargetProject::try_from_path(root)?;
    Ok((temp, target))
}

#[test]
fn check_reports_untracked_new_behavior_tests() {
    let (_temp, target) = must!(initialized_repo(), "initialize repository");
    let tests_dir = target.as_std_path().join("tests");
    must!(fs::create_dir_all(&tests_dir), "create tests dir");
    must!(
        fs::write(
            tests_dir.join("new_behavior.rs"),
            "#[test]\n#[ignore]\nfn tracks_behavior() {\n    assert_eq!(2 + 2, 4);\n}\n",
        ),
        "write new behavior test"
    );

    let rules = must!(TestIntegrityRules::new(), "build rules");
    assert_eq!(must!(check(&target, "HEAD", Vcs::Git, &rules), "check integrity"), 1_i32);
}
