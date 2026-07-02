//! Rust verification gauntlet target-project integration tests.

use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use tempfile::TempDir;

fn plain_target() -> Result<TempDir, std::io::Error> {
    let temp = tempfile::tempdir()?;
    write_package(temp.path(), "plain_target")?;
    Ok(temp)
}

fn write_package(root: &Path, name: &str) -> Result<(), std::io::Error> {
    fs::create_dir_all(root.join("src"))?;
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )?;
    fs::write(root.join("src/lib.rs"), "pub fn value() -> u8 {\n    1\n}\n")?;
    Ok(())
}

fn run_gauntlet(cwd: &Path, mode: &str) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_rust-verification-gauntlet"))
        .arg(mode)
        .current_dir(cwd)
        .output()
}

fn stderr_text(output: &Output) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(output.stderr.clone())
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

#[test]
fn plain_target_without_vb_compile_is_cleanly_not_applicable() {
    let target = must!(plain_target(), "create plain target");

    let output = must!(run_gauntlet(target.path(), "fast"), "run gauntlet");
    let stderr = must!(stderr_text(&output), "decode stderr");

    assert!(!stderr.contains("titania-lanes` not found"), "{stderr}");
    assert_eq!(output.status.code(), Some(0_i32), "{stderr}");
    assert!(stderr.contains("NotApplicable: package vb_compile absent"), "{stderr}");
}
