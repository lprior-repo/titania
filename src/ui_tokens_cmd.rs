use crate::shell::write_stdout;
use anyhow::Context;
use std::path::Path;
use std::path::PathBuf;
use vb_ui_snapshot::tokens::{self, UiTokens};

pub(crate) fn cmd_ui_tokens(
    input_path: &str,
    output_path: &str,
    emit: Option<String>,
    check: bool,
) -> anyhow::Result<()> {
    let ui_tokens = read_ui_tokens(input_path)?;
    let rust_code = tokens::tokens_to_rust_constants(&ui_tokens);
    emit_tokens_if_requested(emit.as_deref(), &ui_tokens, &rust_code)?;
    let output = PathBuf::from(output_path);
    if check {
        return check_generated_tokens(&output, &rust_code, input_path, output_path);
    }
    write_generated_tokens(&output, &rust_code)
}

fn read_ui_tokens(input_path: &str) -> anyhow::Result<UiTokens> {
    let tokens_content = std::fs::read_to_string(input_path)
        .with_context(|| format!("Failed to read tokens file: {input_path}"))?;
    tokens::parse_tokens_from_toml(&tokens_content)
        .with_context(|| format!("Failed to parse tokens from {input_path}"))
}

fn emit_tokens_if_requested(
    emit: Option<&str>,
    ui_tokens: &UiTokens,
    rust_code: &str,
) -> anyhow::Result<()> {
    match emit {
        Some("rust") => write_stdout(format_args!("{}", rust_code)),
        Some("json") => write_tokens_json(ui_tokens),
        _ => Ok(()),
    }
}

fn write_tokens_json(ui_tokens: &UiTokens) -> anyhow::Result<()> {
    let json =
        serde_json::to_string_pretty(ui_tokens).context("Failed to serialize tokens to JSON")?;
    write_stdout(format_args!("{}", json))
}

fn check_generated_tokens(
    output: &Path,
    rust_code: &str,
    input_path: &str,
    output_path: &str,
) -> anyhow::Result<()> {
    let existing = std::fs::read_to_string(output)
        .with_context(|| format!("Failed to read generated tokens at {}", output.display()))?;
    if existing != rust_code {
        anyhow::bail!(
            "Generated UI tokens are stale: run `cargo xtask ui-tokens --input {input_path} --output {output_path}`"
        );
    }
    write_stdout(format_args!(
        "Generated UI tokens are current: {}",
        output.display()
    ))
}

fn write_generated_tokens(output: &Path, rust_code: &str) -> anyhow::Result<()> {
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    std::fs::write(output, rust_code)
        .with_context(|| format!("Failed to write Rust constants to {}", output.display()))?;
    write_stdout(format_args!(
        "Generated Rust tokens at: {}",
        output.display()
    ))
}
