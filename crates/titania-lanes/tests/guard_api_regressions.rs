#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    clippy::missing_panics_doc,
    clippy::panic_in_result_fn,
    clippy::cognitive_complexity,
    clippy::doc_markdown,
    clippy::excessive_nesting,
    clippy::many_single_char_names,
    clippy::integer_division,
    clippy::integer_division_remainder_used,
    clippy::missing_errors_doc,
    clippy::type_complexity,
    clippy::needless_borrow,
    clippy::useless_vec,
    clippy::map_identity,
    reason = "Tests are exempt from the strict production deny list per project doctrine."
)]
//! guard_api_regressions: integration test.
use std::{
    error::Error,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

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

fn write_executable(dir: &Path, name: &str, script: &str) -> Result<PathBuf, std::io::Error> {
    let path = dir.join(name);
    fs::write(&path, script)?;
    let mut permissions = fs::metadata(&path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions)?;
    Ok(path)
}

fn stderr_text(output: &Output) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(output.stderr.clone())
}

#[test]
fn guard_zero_tests_reports_zero_applicable_tests_as_violations() -> TestResult {
    let workspace = fixture_workspace()?;
    let output = Command::new(env!("CARGO_BIN_EXE_guard-zero-tests"))
        .args([
            "--",
            "/bin/sh",
            "-c",
            "printf 'running 0 tests\\n\\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s\\n'",
        ])
        .current_dir(workspace.path())
        .output()?;

    assert_eq!(output.status.code(), Some(1_i32));
    assert!(stderr_text(&output)?.contains("zero applicable tests"));
    Ok(())
}

#[test]
fn check_public_api_diff_runs_cargo_public_api_diff_command() -> TestResult {
    let workspace = fixture_workspace()?;
    let fake_bin = fake_bin_dir()?;
    let log = fake_bin.path().join("rustup.log");
    write_executable(
        fake_bin.path(),
        "cargo",
        "#!/bin/sh\nprintf '{\"packages\":[{\"name\":\"vb_alpha\"}]}'\n",
    )?;
    write_executable(
        fake_bin.path(),
        "rustup",
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" > '{}'\nprintf 'public api diff failed\\n' >&2\nexit 17\n",
            log.display()
        ),
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_check-public-api-diff"))
        .current_dir(workspace.path())
        .env("PATH", fake_bin.path())
        .output()?;

    assert_eq!(output.status.code(), Some(1_i32));
    assert!(stderr_text(&output)?.contains("public api diff failed"));
    let invoked = fs::read_to_string(log)?;
    assert!(invoked.contains("run nightly-2026-04-28 cargo public-api"));
    assert!(invoked.contains("-p vb_alpha diff origin/main..HEAD"));
    Ok(())
}

#[test]
fn check_public_api_diff_reports_missing_public_api_as_failure() -> TestResult {
    let workspace = fixture_workspace()?;
    let fake_bin = fake_bin_dir()?;
    write_executable(
        fake_bin.path(),
        "cargo",
        "#!/bin/sh\nprintf '{\"packages\":[{\"name\":\"vb_alpha\"}]}'\n",
    )?;
    write_executable(
        fake_bin.path(),
        "rustup",
        "#!/bin/sh\nprintf 'error: no such command: public-api\\n' >&2\nexit 1\n",
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_check-public-api-diff"))
        .current_dir(workspace.path())
        .env("PATH", fake_bin.path())
        .output()?;

    let stderr = stderr_text(&output)?;
    assert_eq!(output.status.code(), Some(3_i32));
    assert!(stderr.contains("PUBAPI-TOOL-001"));
    assert!(stderr.contains("no such command: public-api"));
    Ok(())
}

#[test]
fn check_public_api_diff_does_not_fallback_to_legacy_packages() -> TestResult {
    let workspace = fixture_workspace()?;
    let fake_bin = fake_bin_dir()?;
    let log = fake_bin.path().join("rustup.log");
    write_executable(
        fake_bin.path(),
        "cargo",
        "#!/bin/sh\nprintf '{\"packages\":[{\"name\":\"plain\"}]}'\n",
    )?;
    write_executable(
        fake_bin.path(),
        "rustup",
        &format!("#!/bin/sh\nprintf '%s\\n' \"$*\" > '{}'\n", log.display()),
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_check-public-api-diff"))
        .current_dir(workspace.path())
        .env("PATH", fake_bin.path())
        .output()?;
    assert_eq!(output.status.code(), Some(0_i32));
    assert!(stderr_text(&output)?.contains("NotApplicable: no vb_* or velvet-ballistics packages"));
    assert!(!log.exists());
    Ok(())
}
