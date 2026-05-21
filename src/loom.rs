//! Loom concurrency model test command.
//!
//! Implements `cargo xtask loom --model <name>` for VB-CONC-001..005.
//!
//! Each model is a loom test in vb_runtime/src/models/loom/ that verifies
//! ordering invariants for a specific concurrency seam.

#![allow(clippy::print_stderr)]

use std::path::PathBuf;
use std::process;

use anyhow::Context;

const VB_RUNTIME_PATH: &str = "crates/vb_runtime";

const LOOM_MODELS: &[(&str, &str)] = &[
    ("journal_writer_queue", "tests ordered write before flush"),
    (
        "action_completion_cancel",
        "tests completion vs cancel ordering",
    ),
    ("timer_fired_cancel", "tests timer fired vs cancel ordering"),
    ("shutdown_drain", "tests graceful shutdown drain ordering"),
    ("bounded_queue", "tests enqueue/dequeue invariants"),
];

#[allow(clippy::print_stderr)]
pub fn cmd_loom(model: &str) -> anyhow::Result<()> {
    let model_path = find_model(model)?;

    eprintln!("Running loom model: {}", model);
    eprintln!("Model path: {}", model_path.display());
    eprintln!(
        "Command: RUSTFLAGS=\"--cfg loom\" cargo test -p vb_runtime {}",
        model
    );

    let mut cmd = process::Command::new("cargo");
    cmd.arg("test")
        .arg("-p")
        .arg("vb_runtime")
        .env("RUSTFLAGS", "--cfg loom")
        .arg(model);

    let status = cmd
        .status()
        .context("Failed to run loom model (is loom installed?)")?;

    if status.success() {
        eprintln!("PASS: Loom model '{}' completed successfully", model);
        Ok(())
    } else {
        eprintln!("FAIL: Loom model '{}' failed", model);
        eprintln!("Exit code: {:?}", status.code());
        anyhow::bail!(
            "Loom model '{}' failed with exit code {:?}",
            model,
            status.code()
        );
    }
}

fn find_model(model: &str) -> anyhow::Result<PathBuf> {
    let model_dir = PathBuf::from(VB_RUNTIME_PATH).join("src/models/loom");

    let model_file = model_dir.join(format!("{}.rs", model));
    if model_file.exists() {
        return Ok(model_file);
    }

    // Also check if it's a test name (loom test functions are named)
    let candidates: Vec<PathBuf> = std::fs::read_dir(&model_dir)
        .context("Failed to read loom models directory")?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .collect();

    if candidates.is_empty() {
        anyhow::bail!(
            "Loom model '{}' not found. No models exist in {}",
            model,
            model_dir.display()
        );
    }

    eprintln!("Available loom models:");
    for (name, desc) in LOOM_MODELS {
        eprintln!("  {} — {}", name, desc);
    }

    anyhow::bail!(
        "Loom model '{}' not found. Available models: {:?}",
        model,
        LOOM_MODELS.iter().map(|(n, _)| *n).collect::<Vec<_>>()
    );
}

#[allow(dead_code, clippy::print_stderr)]
pub fn list_models() {
    eprintln!("Available loom models:");
    for (name, desc) in LOOM_MODELS {
        eprintln!("  {} — {}", name, desc);
    }
}
