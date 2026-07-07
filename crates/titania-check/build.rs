//! Build script for `titania-check`.
//!
//! Captures the short git SHA at build time and exposes it to runtime via a
//! `rustc-env` value consumed by `src/version.rs`.
//!
//! If `git` is unavailable or the working tree is not a git repository, the
//! SHA falls back to the literal `"unknown"` so the build remains hermetic.
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

/// Reason `git rev-parse` could not supply a SHA. Unit-only variants keep the
/// strict clippy profile satisfied (no unused fields) while still letting the
/// caller distinguish failure modes in the future.
enum ShaError {
    /// `git` could not be spawned or exited non-zero.
    Git,
    /// git printed non-UTF-8 bytes.
    Utf8,
    /// git exited zero but stdout was empty.
    Empty,
}

/// Resolve the short git SHA via `git rev-parse --short HEAD`.
///
/// # Errors
///
/// Returns [`ShaError::Git`] when `git` cannot be spawned or exits non-zero,
/// [`ShaError::Utf8`] when stdout is not valid UTF-8, and [`ShaError::Empty`]
/// when stdout trims to an empty string. Callers fall back to a sentinel
/// value so the build remains hermetic in CI / sandboxed environments.
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

/// Filter the git SHA to hex/dash characters only, capped at [`MAX_SHA_LEN`].
fn sanitize_sha(raw: &str) -> String {
    let cleaned: String =
        raw.chars().filter(|c| c.is_ascii_hexdigit() || *c == '-').take(MAX_SHA_LEN).collect();
    if cleaned.is_empty() { String::from(UNKNOWN_SHA) } else { cleaned }
}

/// Emit one `cargo:rustc-env` build-script instruction.
fn emit_rustc_env(key: &str, value: &str) {
    let mut stdout = std::io::stdout().lock();
    match writeln!(stdout, "cargo:rustc-env={key}={value}") {
        Ok(()) | Err(_) => (),
    }
}

/// Workspace name fallback used by [`workspace_name_fallback`] when
/// `cargo metadata` cannot be parsed at build time.
const DEFAULT_WORKSPACE_NAME: &str = "titania";

/// Return the workspace name literal fallback.
#[must_use]
const fn workspace_name_fallback() -> &'static str {
    DEFAULT_WORKSPACE_NAME
}
