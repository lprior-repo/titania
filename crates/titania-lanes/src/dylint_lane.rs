//! Dylint lane loader — checks tool availability and reports typed infra failures.
//!
//! This module owns the Dylint lane's pre-flight checks:
//!
//! 1. Is `cargo-dylint` on the PATH? If not, return [`LaneFailure::Infra`] with
//!    `tool = "cargo-dylint"`.
//! 2. Is the `libtitania_dylint` cdylib available and ABI-compatible? If not,
//!    return [`LaneFailure::Infra`] with `tool = "libtitania_dylint"`.
//!
//! No lint logic lives here — only the load / wiring contract.

use std::process::Command;

use thiserror::Error;
use titania_core::LaneFailure;

use crate::{LaneReport, RuleId};

const CARGO_DYLINT: &str = "cargo-dylint";
const LIB_TITANIA_DYLINT: &str = "libtitania_dylint";
const RULE_DYLINT_INFRA: &str = "DYLINT_INFRA_FAILURE";

/// Errors produced while probing the Dylint toolchain.
#[derive(Debug, Error)]
pub enum DylintLaneError {
    /// `cargo-dylint` binary could not be spawned (missing from PATH).
    #[error("cargo-dylint binary is unavailable")]
    CargoDylintMissing,
    /// `libtitania_dylint` cdylib is missing or ABI-mismatched.
    #[error("libtitania_dylint is unavailable or ABI-mismatched")]
    LibraryMissing,
}

/// Result of the Dylint lane pre-flight checks.
#[derive(Debug)]
pub enum DylintProbe {
    /// Both tools are available; the lane can proceed.
    Ready,
    /// Infrastructure probe failed with typed details.
    Infra(LaneFailure, LaneReport),
}

impl DylintProbe {
    /// Whether the probe indicates the lane can proceed.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Borrow failure details when the probe did not succeed.
    #[must_use]
    pub const fn failure(&self) -> Option<&LaneFailure> {
        match self {
            Self::Ready => None,
            Self::Infra(failure, _) => Some(failure),
        }
    }
}

/// Probe the Dylint toolchain for availability.
///
/// Returns [`DylintProbe::Ready`] when both `cargo-dylint` and
/// `libtitania_dylint` are available. Returns [`DylintProbe::Infra`]
/// on the first infrastructure failure encountered.
#[must_use]
pub fn probe_dylint_toolchain() -> DylintProbe {
    // Check 1: is cargo-dylint on PATH?
    if !tool_is_available(CARGO_DYLINT) {
        return unavailable_probe(CARGO_DYLINT, format!("{CARGO_DYLINT} binary is unavailable"));
    }

    // Check 2: is libtitania_dylint available and ABI-compatible?
    if !library_is_available(LIB_TITANIA_DYLINT) {
        return unavailable_probe(
            LIB_TITANIA_DYLINT,
            format!("{LIB_TITANIA_DYLINT} is unavailable or ABI-mismatched"),
        );
    }

    DylintProbe::Ready
}

/// Check whether a binary is available on the PATH.
///
/// Uses `std::process::Command` to run `<tool> --version` to completion. If
/// the process can be started and reaped, the binary exists; the exit status
/// itself is not part of the availability contract.
fn tool_is_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").status().is_ok()
}

/// Check whether the `libtitania_dylint` cdylib is available.
///
/// The Dylint framework expects a library named `libtitania_dylint`
/// (or `titania_dylint.dll` / `titania_dylint.dylib` on Windows / macOS).
/// We probe for the library's presence in `CARGO_TARGET_DIR`, falling back to
/// the workspace `target/` directory used by normal Cargo builds.
fn library_is_available(lib_name: &str) -> bool {
    let target_dir = cargo_target_dir();
    let candidates = [
        format!("debug/{lib_name}.so"),
        format!("debug/{lib_name}.dylib"),
        format!("debug/{lib_name}.dll"),
        format!("release/{lib_name}.so"),
        format!("release/{lib_name}.dylib"),
        format!("release/{lib_name}.dll"),
    ];

    candidates.iter().any(|path| target_dir.join(path).is_file())
}

fn cargo_target_dir() -> std::path::PathBuf {
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(
        || std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"),
        std::path::PathBuf::from,
    )
}

fn unavailable_probe(tool: &str, reason: String) -> DylintProbe {
    let failure = LaneFailure::Infra { tool: tool.to_owned(), reason };

    match infra_report(tool) {
        Ok(report) => DylintProbe::Infra(failure, report),
        Err(report_failure) => DylintProbe::Infra(report_failure, LaneReport::new()),
    }
}

/// Build a minimal [`LaneReport`] for an infrastructure failure.
///
/// # Errors
///
/// Returns [`LaneFailure::Infra`] if the static Dylint infra rule id violates
/// the shared rule-id format.
fn infra_report(tool: &str) -> Result<LaneReport, LaneFailure> {
    let mut report = LaneReport::new();
    let rule = RuleId::new(RULE_DYLINT_INFRA).map_err(|err| LaneFailure::Infra {
        tool: String::from("titania-dylint"),
        reason: format!("invalid Dylint infra rule id {RULE_DYLINT_INFRA}: {err}"),
    })?;

    report.push(crate::Finding::new(rule, tool, 0, format!("{tool} is unavailable")));
    Ok(report)
}
