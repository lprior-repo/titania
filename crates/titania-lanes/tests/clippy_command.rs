//! Clippy command behavior tests — failing-first, exact argv and infra failure.
//!
//! These tests prove `run-cargo clippy` writes typed v1 lane artifacts with the
//! exact Clippy command contract from `v1-spec.md` §9.2 plus JSON message output.
//!
//! Beads: tn-d2l.1

use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

const EXPECTED_CLIPPY_ARGV: &[&str] = &[
    "cargo",
    "clippy",
    "--workspace",
    "--lib",
    "--bins",
    "--examples",
    "--frozen",
    "--message-format=json",
    "--",
    "-F",
    "clippy::unwrap_used",
    "-F",
    "clippy::expect_used",
    "-F",
    "clippy::panic",
    "-F",
    "clippy::panic_in_result_fn",
    "-F",
    "clippy::todo",
    "-F",
    "clippy::unimplemented",
    "-F",
    "clippy::indexing_slicing",
    "-F",
    "clippy::string_slice",
    "-F",
    "clippy::get_unwrap",
    "-F",
    "clippy::arithmetic_side_effects",
    "-F",
    "clippy::dbg_macro",
    "-F",
    "clippy::as_conversions",
    "-F",
    "clippy::let_underscore_must_use",
    "-F",
    "clippy::await_holding_lock",
    "-D",
    "warnings",
];

fn run_cargo_binary() -> Result<PathBuf, std::io::Error> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let status = Command::new("cargo")
        .arg("build")
        .arg("--bin")
        .arg("run-cargo")
        .current_dir(manifest_dir)
        .status()?;
    if status.success() {
        Ok(PathBuf::from(env!("CARGO_BIN_EXE_run-cargo")))
    } else {
        Err(std::io::Error::other("cargo build --bin run-cargo failed"))
    }
}

fn run_clippy(cwd: &Path) -> Result<Output, std::io::Error> {
    Command::new(run_cargo_binary()?).arg("clippy").current_dir(cwd).output()
}

fn clean_package(name: &str) -> Result<TempDir, std::io::Error> {
    let temp = tempfile::tempdir()?;
    write_manifest(temp.path(), name)?;
    fs::create_dir_all(temp.path().join("src"))?;
    fs::write(temp.path().join("src/lib.rs"), "pub fn value() -> u8 { 1 }\n")?;
    generate_lockfile(temp.path())?;
    Ok(temp)
}

fn write_manifest(root: &Path, name: &str) -> Result<(), std::io::Error> {
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
}

fn generate_lockfile(root: &Path) -> Result<(), std::io::Error> {
    let status = Command::new("cargo")
        .arg("generate-lockfile")
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("cargo generate-lockfile failed"))
    }
}

fn clippy_artifact_path(root: &Path) -> PathBuf {
    root.join(".titania/out/prepush/clippy.json")
}

fn read_artifact(root: &Path) -> Result<serde_json::Value, Box<dyn Error>> {
    let payload = fs::read_to_string(clippy_artifact_path(root))?;
    serde_json::from_str(&payload).map_err(Into::into)
}

#[test]
fn clippy_command_clean_artifact_records_exact_v1_command() -> TestResult {
    let target = clean_package("clippy_exact_command")?;

    let output = run_clippy(target.path())?;
    assert_eq!(output.status.code(), Some(0_i32));

    let artifact = read_artifact(target.path())?;
    assert_eq!(artifact["lane"].as_str(), Some("Clippy"));
    assert_eq!(artifact["outcome"]["variant"].as_str(), Some("clean"));

    let argv = artifact["outcome"]["evidence"]["command"]["argv"]
        .as_array()
        .ok_or_else(|| std::io::Error::other("command argv must be an array"))?
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>();
    assert_eq!(argv, EXPECTED_CLIPPY_ARGV);
    Ok(())
}

#[test]
fn clippy_command_without_cargo_records_infra_failure() -> TestResult {
    let target = clean_package("clippy_no_cargo")?;

    let output = Command::new(run_cargo_binary()?)
        .arg("clippy")
        .current_dir(target.path())
        .env("PATH", "")
        .output()?;
    assert_ne!(output.status.code(), Some(0_i32));

    let artifact = read_artifact(target.path())?;
    assert_eq!(artifact["lane"].as_str(), Some("Clippy"));
    assert_eq!(artifact["outcome"]["variant"].as_str(), Some("failed"));
    assert_eq!(artifact["outcome"]["failure"]["infra_failure"]["tool"].as_str(), Some("cargo"));
    assert!(artifact["outcome"]["failure"]["infra_failure"]["reason"].as_str().is_some());
    Ok(())
}
