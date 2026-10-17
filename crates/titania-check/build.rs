#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]

use std::{
    io::Write,
    process::{Command, Stdio},
};

const UNKNOWN_SHA: &str = "unknown";
const MAX_SHA_LEN: usize = 64;
const BUILD_GIT_SHA_ENV: &str = "TITANIA_BUILD_GIT_SHA";
const WORKSPACE_NAME_ENV: &str = "TITANIA_WORKSPACE_NAME";

fn main() {
    let sha =
        resolve_git_sha().map_or_else(|_| String::from(UNKNOWN_SHA), |raw| sanitize_sha(&raw));
    emit_rustc_env(BUILD_GIT_SHA_ENV, &sha);
    emit_rustc_env(WORKSPACE_NAME_ENV, workspace_name_fallback());
}

enum ShaError {
    Git,
    Utf8,
    Empty,
}

fn resolve_git_sha() -> Result<String, ShaError> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|_io| ShaError::Git)?;
    if !output.status.success() {
        return Err(ShaError::Git);
    }
    let raw = String::from_utf8(output.stdout).map_err(|_utf8| ShaError::Utf8)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ShaError::Empty);
    }
    Ok(trimmed.to_owned())
}

fn sanitize_sha(raw: &str) -> String {
    let cleaned: String =
        raw.chars().filter(|c| c.is_ascii_hexdigit() || *c == '-').take(MAX_SHA_LEN).collect();
    if cleaned.is_empty() { String::from(UNKNOWN_SHA) } else { cleaned }
}

fn emit_rustc_env(key: &str, value: &str) {
    let mut stdout = std::io::stdout().lock();
    match writeln!(stdout, "cargo:rustc-env={key}={value}") {
        Ok(()) | Err(_) => (),
    }
}

const DEFAULT_WORKSPACE_NAME: &str = "titania";

#[must_use]
const fn workspace_name_fallback() -> &'static str {
    DEFAULT_WORKSPACE_NAME
}
