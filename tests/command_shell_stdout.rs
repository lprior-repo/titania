use std::error::Error;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

const TOKENS_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../velvet-ballistics/design/tokens/velvet_ui_tokens.toml"
);

#[test]
fn ui_tokens_stdout_is_json_when_json_emit_is_requested() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;
    let output_path = workspace.path().join("tokens_generated.rs");
    let output_arg = output_path.to_string_lossy().to_string();

    // When
    let output = run_xtask(
        workspace.path(),
        &[
            "ui-tokens",
            "--input",
            TOKENS_FILE,
            "--output",
            &output_arg,
            "--emit",
            "json",
        ],
    )?;

    // Then
    require(output.status.code() == Some(0), "xtask exited non-zero")?;
    let stdout = stdout_text(&output)?;
    require(
        stdout.contains("\"window_width\""),
        "stdout missing JSON token",
    )?;
    require(
        !stdout.contains("pub const TOKENS"),
        "stdout unexpectedly emitted Rust tokens",
    )?;
    require(
        std::fs::read_to_string(output_path)?.contains("pub const TOKENS"),
        "output file missing Rust tokens",
    )?;
    Ok(())
}

#[test]
fn ui_tokens_stdout_is_rust_when_rust_emit_is_requested() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;
    let output_path = workspace.path().join("tokens_generated.rs");
    let output_arg = output_path.to_string_lossy().to_string();

    // When
    let output = run_xtask(
        workspace.path(),
        &[
            "ui-tokens",
            "--input",
            TOKENS_FILE,
            "--output",
            &output_arg,
            "--emit",
            "rust",
        ],
    )?;

    // Then
    require(output.status.code() == Some(0), "xtask exited non-zero")?;
    let stdout = stdout_text(&output)?;
    require(
        stdout.contains("pub const TOKENS"),
        "stdout missing Rust tokens",
    )?;
    require(
        !stdout.contains("\"window_width\""),
        "stdout unexpectedly emitted JSON token",
    )?;
    require(
        std::fs::read_to_string(output_path)?.contains("pub const TOKENS"),
        "output file missing Rust tokens",
    )?;
    Ok(())
}

fn run_xtask(current_dir: &Path, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(current_dir)
        .args(args)
        .output()
        .map_err(Into::into)
}

fn stdout_text(output: &Output) -> Result<String, Box<dyn Error>> {
    String::from_utf8(output.stdout.clone()).map_err(Into::into)
}

fn require(condition: bool, message: &'static str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
