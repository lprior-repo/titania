//! Regression tests guarding lane exit codes and stderr contracts.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

use tempfile::TempDir;

fn fixture_workspace() -> Result<TempDir, std::io::Error> {
    let temp = tempfile::tempdir()?;
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )?;
    fs::create_dir_all(temp.path().join("src"))?;
    fs::write(temp.path().join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n")?;
    Ok(temp)
}

fn fake_bin_dir() -> Result<TempDir, std::io::Error> {
    tempfile::tempdir()
}

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

fn write_executable(dir: &Path, name: &str, script: &str) -> Result<(), std::io::Error> {
    let path = dir.join(name);
    fs::write(&path, script)?;
    let mut permissions = fs::metadata(&path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions)?;
    Ok(())
}

fn stderr_text(output: &Output) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(output.stderr.clone())
}

#[test]
fn guard_zero_tests_reports_zero_applicable_tests_as_violations() {
    let workspace = must!(fixture_workspace(), "create fixture workspace");
    let output = must!(
        Command::new(env!("CARGO_BIN_EXE_guard-zero-tests"))
            .args([
            "--",
            "/bin/sh",
            "-c",
            "printf 'running 0 tests\\n\\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s\\n'",
            ])
            .current_dir(workspace.path())
            .output(),
        "run guard-zero-tests"
    );

    assert_eq!(output.status.code(), Some(1_i32));
    assert!(must!(stderr_text(&output), "decode stderr").contains("zero applicable tests"));
}

#[test]
fn check_public_api_diff_runs_cargo_public_api_diff_command() {
    let workspace = must!(fixture_workspace(), "create fixture workspace");
    let fake_bin = must!(fake_bin_dir(), "create fake bin dir");
    let log = fake_bin.path().join("rustup.log");
    must!(
        write_executable(
            fake_bin.path(),
            "cargo",
            "#!/bin/sh\nprintf '{\"packages\":[{\"name\":\"vb_alpha\"}]}'\n",
        ),
        "write fake cargo"
    );
    must!(
        write_executable(
            fake_bin.path(),
            "rustup",
            &format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" > '{}'\nprintf 'public api diff failed\\n' >&2\nexit 17\n",
                log.display()
            ),
        ),
        "write fake rustup"
    );

    let output = must!(
        Command::new(env!("CARGO_BIN_EXE_check-public-api-diff"))
            .current_dir(workspace.path())
            .env("PATH", fake_bin.path())
            .output(),
        "run check-public-api-diff"
    );

    assert_eq!(output.status.code(), Some(1_i32));
    assert!(must!(stderr_text(&output), "decode stderr").contains("public api diff failed"));
    let invoked = must!(fs::read_to_string(log), "read rustup invocation log");
    assert!(invoked.contains("run nightly-2026-04-28 cargo public-api"));
    assert!(invoked.contains("-p vb_alpha diff origin/main..HEAD"));
}

#[test]
fn check_public_api_diff_reports_missing_public_api_as_failure() {
    let workspace = must!(fixture_workspace(), "create fixture workspace");
    let fake_bin = must!(fake_bin_dir(), "create fake bin dir");
    must!(
        write_executable(
            fake_bin.path(),
            "cargo",
            "#!/bin/sh\nprintf '{\"packages\":[{\"name\":\"vb_alpha\"}]}'\n",
        ),
        "write fake cargo"
    );
    must!(
        write_executable(
            fake_bin.path(),
            "rustup",
            "#!/bin/sh\nprintf 'error: no such command: public-api\\n' >&2\nexit 1\n",
        ),
        "write fake rustup"
    );

    let output = must!(
        Command::new(env!("CARGO_BIN_EXE_check-public-api-diff"))
            .current_dir(workspace.path())
            .env("PATH", fake_bin.path())
            .output(),
        "run check-public-api-diff"
    );

    let stderr = must!(stderr_text(&output), "decode stderr");
    assert_eq!(output.status.code(), Some(3_i32));
    assert!(stderr.contains("PUBAPI_TOOL_001"));
    assert!(stderr.contains("no such command: public-api"));
}

#[test]
fn check_public_api_diff_does_not_fallback_to_legacy_packages() {
    let workspace = must!(fixture_workspace(), "create fixture workspace");
    let fake_bin = must!(fake_bin_dir(), "create fake bin dir");
    let log = fake_bin.path().join("rustup.log");
    must!(
        write_executable(
            fake_bin.path(),
            "cargo",
            "#!/bin/sh\nprintf '{\"packages\":[{\"name\":\"plain\"}]}'\n",
        ),
        "write fake cargo"
    );
    must!(
        write_executable(
            fake_bin.path(),
            "rustup",
            &format!("#!/bin/sh\nprintf '%s\\n' \"$*\" > '{}'\n", log.display()),
        ),
        "write fake rustup"
    );

    let output = must!(
        Command::new(env!("CARGO_BIN_EXE_check-public-api-diff"))
            .current_dir(workspace.path())
            .env("PATH", fake_bin.path())
            .output(),
        "run check-public-api-diff"
    );
    assert_eq!(output.status.code(), Some(0_i32));
    assert!(
        must!(stderr_text(&output), "decode stderr")
            .contains("NotApplicable: no vb_* or velvet-ballistics packages")
    );
    assert!(!log.exists());
}
