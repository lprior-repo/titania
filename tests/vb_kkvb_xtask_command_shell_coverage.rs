#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::error::Error;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

const WORKSPACE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../velvet-ballistics");
const TOOLING_MANIFEST: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
const TOKENS_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../velvet-ballistics/design/tokens/velvet_ui_tokens.toml"
);

#[test]
fn xtask_help_lists_required_and_legacy_commands_when_requested() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;

    // When
    let output = run_xtask(workspace.path(), &["--help"])?;

    // Then
    if output.status.code() != Some(0) {
        return Err(format!("expected status code 0, got {:?}", output.status.code()).into());
    }
    let stdout = stdout_text(&output)?;
    if !stdout.contains("Required command families:") {
        return Err(format!(
            "expected stdout to contain 'Required command families:', got: {}",
            stdout
        )
        .into());
    }
    if !stdout.contains("  ai-context") {
        return Err(format!("expected stdout to contain '  ai-context', got: {}", stdout).into());
    }
    if !stdout.contains("Legacy commands:") {
        return Err(format!(
            "expected stdout to contain 'Legacy commands:', got: {}",
            stdout
        )
        .into());
    }
    if !stdout.contains("  ui-snapshot") {
        return Err(format!(
            "expected stdout to contain '  ui-snapshot', got: {}",
            stdout
        )
        .into());
    }
    Ok(())
}

#[test]
fn xtask_version_prints_package_version_when_requested() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;

    // When
    let output = run_xtask(workspace.path(), &["--version"])?;

    // Then
    if output.status.code() != Some(0) {
        return Err(format!("expected status code 0, got {:?}", output.status.code()).into());
    }
    let stdout = stdout_text(&output)?;
    if stdout != "xtask 0.1.0\n" {
        return Err(format!("expected version output 'xtask 0.1.0\\n', got: {}", stdout).into());
    }
    Ok(())
}

#[test]
fn xtask_legacy_separator_routes_ui_overlap_check_and_reports_missing_screen()
-> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;

    // When
    let output = run_xtask(
        workspace.path(),
        &[
            "--",
            "ui-overlap-check",
            "--screen",
            "missing_screen",
            "--input-dir",
            "missing_snapshots",
        ],
    )?;

    // Then
    if output.status.code() != Some(1) {
        return Err(format!("expected status code 1, got {:?}", output.status.code()).into());
    }
    let stdout = stdout_text(&output)?;
    if !stdout.contains("FAIL: missing_snapshots/missing_screen.png does not exist") {
        return Err(format!("expected stdout to contain 'FAIL: missing_snapshots/missing_screen.png does not exist', got: {}", stdout).into());
    }
    let stderr = stderr_text(&output)?;
    if !stderr.contains("UI overlap check failed") {
        return Err(format!(
            "expected stderr to contain 'UI overlap check failed', got: {}",
            stderr
        )
        .into());
    }
    Ok(())
}

#[test]
fn xtask_ui_tokens_writes_rust_constants_when_tokens_are_valid() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;
    let output_path = workspace.path().join("generated").join("tokens.rs");
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
    if output.status.code() != Some(0) {
        return Err(format!("expected status code 0, got {:?}", output.status.code()).into());
    }
    if !output_path.exists() {
        return Err(format!("expected output path {:?} to exist", output_path).into());
    }
    let stdout = stdout_text(&output)?;
    if !stdout.contains("background_board") {
        return Err(format!(
            "expected stdout to contain 'background_board', got: {}",
            stdout
        )
        .into());
    }
    let file_content = std::fs::read_to_string(output_path)?;
    if !file_content.contains("pub const TOKENS") {
        return Err(format!(
            "expected file content to contain 'pub const TOKENS', got: {}",
            file_content
        )
        .into());
    }
    Ok(())
}

#[test]
fn xtask_ui_tokens_check_confirms_generated_tokens_when_file_matches() -> Result<(), Box<dyn Error>>
{
    // Given
    let workspace = TempDir::new()?;
    let output_path = workspace.path().join("generated_tokens.rs");
    let output_arg = output_path.to_string_lossy().to_string();
    let write_output = run_xtask(
        workspace.path(),
        &["ui-tokens", "--input", TOKENS_FILE, "--output", &output_arg],
    )?;
    if write_output.status.code() != Some(0) {
        return Err(format!(
            "expected write status code 0, got {:?}",
            write_output.status.code()
        )
        .into());
    }

    // When
    let check_output = run_xtask(
        workspace.path(),
        &[
            "ui-tokens",
            "--input",
            TOKENS_FILE,
            "--output",
            &output_arg,
            "--check",
        ],
    )?;

    // Then
    if check_output.status.code() != Some(0) {
        return Err(format!(
            "expected check status code 0, got {:?}",
            check_output.status.code()
        )
        .into());
    }
    let stdout = stdout_text(&check_output)?;
    if !stdout.contains("Generated UI tokens are current") {
        return Err(format!(
            "expected stdout to contain 'Generated UI tokens are current', got: {}",
            stdout
        )
        .into());
    }
    Ok(())
}

#[test]
fn xtask_ui_tokens_check_rejects_stale_generated_tokens() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;
    let output_path = workspace.path().join("stale_tokens.rs");
    std::fs::write(&output_path, "stale")?;
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
            "--check",
        ],
    )?;

    // Then
    if output.status.code() != Some(1) {
        return Err(format!("expected status code 1, got {:?}", output.status.code()).into());
    }
    let stderr = stderr_text(&output)?;
    if !stderr.contains("Generated UI tokens are stale") {
        return Err(format!(
            "expected stderr to contain 'Generated UI tokens are stale', got: {}",
            stderr
        )
        .into());
    }
    Ok(())
}

#[test]
fn xtask_ui_snapshot_captures_named_fixture_and_writes_report() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;
    let output_dir = workspace.path().join("snapshots");
    let output_arg = output_dir.to_string_lossy().to_string();

    // When
    let output = run_xtask(
        Path::new(WORKSPACE_ROOT),
        &[
            "ui-snapshot",
            "--fixture",
            "execution_overview",
            "--output-dir",
            &output_arg,
            "--emit",
            "yaml",
        ],
    )?;

    // Then
    if output.status.code() != Some(0) {
        return Err(format!("expected status code 0, got {:?}", output.status.code()).into());
    }
    if !output_dir.join("execution_overview.png").exists() {
        return Err(format!(
            "expected {:?} to exist",
            output_dir.join("execution_overview.png")
        )
        .into());
    }
    if !output_dir.join("ui_snapshot_report.yaml").exists() {
        return Err(format!(
            "expected {:?} to exist",
            output_dir.join("ui_snapshot_report.yaml")
        )
        .into());
    }
    let stdout = stdout_text(&output)?;
    if !stdout.contains("Snapshot report written to:") {
        return Err(format!(
            "expected stdout to contain 'Snapshot report written to:', got: {}",
            stdout
        )
        .into());
    }
    Ok(())
}

#[test]
fn xtask_ui_snapshot_rejects_invocation_without_all_or_fixture() -> Result<(), Box<dyn Error>> {
    // Given
    let workspace = TempDir::new()?;

    // When
    let output = run_xtask(workspace.path(), &["ui-snapshot"])?;

    // Then
    if output.status.code() != Some(1) {
        return Err(format!("expected status code 1, got {:?}", output.status.code()).into());
    }
    let stderr = stderr_text(&output)?;
    if !stderr.contains("Must specify --all or --fixture <name>") {
        return Err(format!(
            "expected stderr to contain 'Must specify --all or --fixture <name>', got: {}",
            stderr
        )
        .into());
    }
    Ok(())
}

fn run_xtask(current_dir: &Path, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Command::new("cargo")
        .current_dir(current_dir)
        .args(["run", "--locked", "--manifest-path", TOOLING_MANIFEST, "--"])
        .args(args)
        .output()
        .map_err(Into::into)
}

fn stdout_text(output: &Output) -> Result<String, Box<dyn Error>> {
    String::from_utf8(output.stdout.clone()).map_err(Into::into)
}

fn stderr_text(output: &Output) -> Result<String, Box<dyn Error>> {
    String::from_utf8(output.stderr.clone()).map_err(Into::into)
}
