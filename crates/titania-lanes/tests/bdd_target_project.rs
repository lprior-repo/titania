//! Behavior (BDD) tests for target-project discovery.

use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use tempfile::TempDir;
use titania_core::{
    Digest, LaneDigest, LaneName, ReceiptDigests, ReceiptEnvelope, ReceiptLaneExit, ReceiptPeriod,
    TargetProject, TargetProjectError, discover_target,
};

fn run_cargo(cwd: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_run-cargo")).args(args).current_dir(cwd).output()
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

fn single_crate(name: &str, lib_rs: &str) -> Result<TempDir, std::io::Error> {
    let temp = tempfile::tempdir()?;
    write_package(temp.path(), name, lib_rs)?;
    Ok(temp)
}

fn workspace_with_member(
    member_lib_rs: &str,
) -> Result<(TempDir, std::path::PathBuf), std::io::Error> {
    let temp = tempfile::tempdir()?;
    fs::write(
        temp.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/foo\"]\nresolver = \"3\"\n",
    )?;
    let member = temp.path().join("crates/foo");
    write_package(&member, "foo", member_lib_rs)?;
    Ok((temp, member))
}

fn write_package(root: &Path, name: &str, lib_rs: &str) -> Result<(), std::io::Error> {
    fs::create_dir_all(root.join("src"))?;
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )?;
    fs::write(root.join("src/lib.rs"), lib_rs)?;
    Ok(())
}

fn stderr_text(output: &Output) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(output.stderr.clone())
}

fn stable_digest(label: &'static [u8]) -> Digest {
    Digest::from_bytes(label)
}

#[test]
fn scenario_workspace_discovery_from_subcrate_reports_member_diff() {
    // Given: cwd is a sub-crate of a workspace with badly formatted Rust.
    let (_workspace, member) =
        must!(workspace_with_member("pub fn value()->u8{1}\n"), "create workspace fixture");

    // When: run-cargo fmt is invoked from the member directory.
    let output = must!(run_cargo(&member, &["fmt"]), "run cargo fmt from member");
    let stderr = must!(stderr_text(&output), "decode stderr");

    // Then: the lane discovers the workspace target and reports the member diff.
    assert_eq!(output.status.code(), Some(1_i32));
    assert!(stderr.contains("CARGO_FMT_001"));
    assert!(stderr.contains("crates/foo/src/lib.rs"));
}

#[test]
fn scenario_single_crate_root_uses_cwd_as_target() {
    // Given: cwd is a standalone Cargo package root.
    let target = must!(
        single_crate("single_crate_target", "pub fn value()->u8{1}\n"),
        "create single-crate fixture"
    );

    // When: run-cargo fmt is invoked from that root.
    let output = must!(run_cargo(target.path(), &["fmt"]), "run cargo fmt from target");
    let stderr = must!(stderr_text(&output), "decode stderr");

    // Then: the lane reports the file inside that single-crate target.
    assert_eq!(output.status.code(), Some(1_i32));
    assert!(stderr.contains("CARGO_FMT_001"));
    assert!(stderr.contains("src/lib.rs"));
}

#[test]
fn scenario_missing_cargo_toml_returns_usage_with_typed_error() {
    // Given: cwd has no Cargo.toml in itself or its temporary ancestors.
    let target = must!(tempfile::tempdir(), "create empty target fixture");

    // When: run-cargo fmt is invoked there.
    let output = must!(run_cargo(target.path(), &["fmt"]), "run cargo fmt without manifest");
    let stderr = must!(stderr_text(&output), "decode stderr");

    // Then: discovery fails closed as a usage/config error with the typed message.
    assert_eq!(output.status.code(), Some(2_i32));
    assert!(stderr.contains("target discovery failed"));
    assert!(stderr.contains("target project directory does not contain a Cargo.toml file"));
}

#[test]
fn scenario_completed_lane_receipt_records_resolved_target_root() {
    // Given: a clean standalone Cargo package and a successful lane run.
    let target_dir = must!(
        single_crate("receipt_target", "pub fn value() -> u8 {\n    1\n}\n"),
        "create receipt target fixture"
    );
    let output = must!(run_cargo(target_dir.path(), &["fmt"]), "run clean cargo fmt");
    assert_eq!(output.status.code(), Some(0_i32));

    // When: a receipt is built for the completed lane.
    let target = must!(discover_target(target_dir.path()), "discover target");
    let receipt = ReceiptEnvelope::new(
        &target,
        must!(ReceiptPeriod::new(1, 2), "build receipt period"),
        vec![must!(
            LaneDigest::new(
                must!(LaneName::new("fmt"), "build lane name"),
                ReceiptLaneExit::Clean,
                1,
                1,
                0,
            ),
            "build lane digest"
        )],
        ReceiptDigests::new(
            stable_digest(b"source"),
            stable_digest(b"lock"),
            stable_digest(b"policy"),
            stable_digest(b"toolchain"),
        ),
    );
    let receipt = must!(receipt, "build quality receipt");
    let json = must!(serde_json::to_string(&receipt), "serialize receipt");

    // Then: the serialized receipt includes the resolved target_root.
    assert!(json.contains("\"target_root\""));
    assert!(json.contains(&target_dir.path().display().to_string()));
}

#[test]
fn scenario_empty_target_input_returns_typed_error_without_panic() {
    // Given: an empty target path.
    let empty = Path::new("");

    // When: the TargetProject constructor validates it.
    let result = TargetProject::try_from_path(empty);

    // Then: construction returns the exact typed error.
    assert_eq!(result, Err(TargetProjectError::Empty));
}
