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
fn check_public_api_diff_runs_plain_packages_without_product_filter() {
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
    let invoked = must!(fs::read_to_string(log), "read rustup invocation log");
    assert!(invoked.contains("-p plain diff origin/main..HEAD"));
}

/// Scan all production crate source files under `crates/` for legacy
/// `velvet-ballistics` / `velvet_ballistics` strings.  Fails while any
/// `.rs` file (outside `tests/` directories) contains these tokens so that
/// the removal is tracked as a regression guard, not a one-off fix.
#[test]
fn no_velvet_ballistics_legacy_refs_in_crate_source() {
    // CARGO_MANIFEST_DIR is crates/titania-lanes; its parent IS the workspace
    // `crates/` directory.  Do not append another "crates" segment.
    let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("titania-lanes sits inside a `crates/` workspace")
        .to_path_buf();

    fn collect_legacy_refs(dir: &std::path::Path, out: &mut Vec<String>) {
        for entry in match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(_) => return,
        } {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                // Skip test directories — they may contain test fixtures that
                // intentionally reference legacy names.
                if entry.file_name() == "tests" {
                    continue;
                }
                collect_legacy_refs(&path, out);
            } else if path.extension().map_or(false, |ext| ext == "rs") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    for (line_no, line) in content.lines().enumerate() {
                        if line.contains("velvet-ballistics") || line.contains("velvet_ballistics")
                        {
                            out.push(format!(
                                "{}:{}: {}",
                                path.display(),
                                line_no + 1,
                                line.trim()
                            ));
                        }
                    }
                }
            }
        }
    }

    let mut refs: Vec<String> = Vec::new();
    collect_legacy_refs(&crates_dir, &mut refs);

    if !refs.is_empty() {
        panic!(
            "Production crate code still references legacy `velvet-ballistics`\n\
             / `velvet_ballistics` tokens.  Remove all such references so that\n\
             lane artifacts remain project-neutral:\n\n{}\n",
            refs.join("\n")
        );
    }
}
