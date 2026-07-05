//! Diffs `cargo public-api` for every `vb_*` package against `origin/main`.
//!
//! Rust re-implementation of the bash lane `scripts/check-public-api-diff.sh`. Run via
//! `cargo run --bin check-public-api-diff --` from the repository root or via
//! the matching Moon task in `.moon/tasks/all.yml`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

#[path = "check_public_api_diff/package_json.rs"]
/// Cargo metadata package-name extraction helpers.
pub mod package_json;

use titania_core::TargetProject;
use titania_lanes::{
    CommandIn, Finding, LaneError, LaneExit, LaneReport, RuleId, RuleIdError,
    current_target_project, exit,
};

use std::io::{self, Write};
use thiserror::Error;

use package_json::extract_package_names;

const RULE_CARGO_MISSING: &str = "PUBAPI_CARGO_MISSING_001";
const RULE_METADATA: &str = "PUBAPI_METADATA_001";
const RULE_PUBLIC_API_DIFF: &str = "PUBAPI_DIFF_001";
const RULE_PUBLIC_API_TOOL: &str = "PUBAPI_TOOL_001";
const RULE_TARGET: &str = "PUBAPI_TARGET_001";
const TOOLCHAIN: &str = "nightly-2026-04-27";

struct PubApiRules {
    cargo_missing: RuleId,
    metadata: RuleId,
    public_api_diff: RuleId,
    public_api_tool: RuleId,
    target: RuleId,
}

impl PubApiRules {
    /// Builds typed rule IDs used by the public API lane.
    ///
    /// # Errors
    ///
    /// Returns a [`RuleIdError`] when a configured rule ID literal is invalid.
    fn new() -> Result<Self, RuleIdError> {
        Ok(Self {
            cargo_missing: RuleId::new(RULE_CARGO_MISSING)?,
            metadata: RuleId::new(RULE_METADATA)?,
            public_api_diff: RuleId::new(RULE_PUBLIC_API_DIFF)?,
            public_api_tool: RuleId::new(RULE_PUBLIC_API_TOOL)?,
            target: RuleId::new(RULE_TARGET)?,
        })
    }
}

fn filter_packages(discovered: Vec<String>) -> Vec<String> {
    let mut selected: Vec<String> =
        discovered.into_iter().filter(|package| package.starts_with("vb_")).collect();
    selected.sort();
    selected.dedup();
    selected
}

#[derive(Debug, Error)]
enum PackageDiscoveryError {
    #[error("cargo metadata failed to start: {source}")]
    Command { source: LaneError },
    #[error("cargo metadata failed: {stderr}")]
    Status { stderr: String },
    #[error("cargo metadata returned non-UTF8 JSON: {source}")]
    NonUtf8 { source: std::string::FromUtf8Error },
}

impl PackageDiscoveryError {
    const fn is_missing_command(&self) -> bool {
        matches!(self, Self::Command { .. })
    }
}

/// Discovers public API checked package names from Cargo metadata.
///
/// # Errors
///
/// Returns a typed error when `cargo metadata` cannot start, exits with a
/// failure status, or emits non-UTF-8 JSON.
fn discover_packages(target: &TargetProject) -> Result<Vec<String>, PackageDiscoveryError> {
    let manifest = target.manifest_path();
    let mut command = CommandIn::new(target, "cargo")
        .map_err(|source| PackageDiscoveryError::Command { source })?;
    let _ = command.inherit_env();
    let _ = command
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .arg("--manifest-path")
        .arg(manifest.as_str());
    let output =
        command.run_capture_raw().map_err(|source| PackageDiscoveryError::Command { source })?;
    if !output.status().success() {
        let stderr = String::from_utf8_lossy(output.stderr()).into_owned();
        return Err(PackageDiscoveryError::Status { stderr });
    }
    let text = String::from_utf8(output.stdout().to_vec())
        .map_err(|source| PackageDiscoveryError::NonUtf8 { source })?;
    Ok(filter_packages(extract_package_names(&text)))
}

enum PublicApiDiff {
    Clean,
    Violation(String),
    Failure(String),
}

struct DiffContext<'a> {
    target: &'a TargetProject,
    rules: &'a PubApiRules,
}

fn run_public_api_diff(target: &TargetProject, package: &str) -> PublicApiDiff {
    let mut command = match CommandIn::new(target, "rustup") {
        Ok(command) => command,
        Err(error) => return PublicApiDiff::Failure(format!("rustup command invalid: {error}")),
    };
    let _ = command.inherit_env();
    let _ = command.env_remove("RUSTC_WRAPPER");
    let _ = command.env("SCCACHE_DISABLE", "1");
    add_public_api_args(&mut command, package);
    classify_public_api_output(package, command.run_capture_raw())
}

fn add_public_api_args<'a>(command: &mut CommandIn<'a>, package: &'a str) {
    let _ = command
        .arg("run")
        .arg(TOOLCHAIN)
        .arg("cargo")
        .arg("public-api")
        .arg("-p")
        .arg(package)
        .arg("diff")
        .arg("origin/main..HEAD")
        .arg("--all-features")
        .arg("--deny")
        .arg("removed")
        .arg("--deny")
        .arg("changed");
}

fn classify_public_api_output(
    package: &str,
    output: Result<titania_lanes::CommandOutput, titania_lanes::LaneError>,
) -> PublicApiDiff {
    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return PublicApiDiff::Failure(format!(
                "cargo public-api failed for {package}: {error}"
            ));
        }
    };
    if output.status().success() {
        return PublicApiDiff::Clean;
    }
    let stderr = String::from_utf8_lossy(output.stderr());
    let stdout = String::from_utf8_lossy(output.stdout());
    let message = format!(
        "cargo public-api diff failed for {package} with code {:?}\n{stdout}{stderr}",
        output.status().code()
    );
    if is_public_api_missing(&message) {
        PublicApiDiff::Failure(message)
    } else {
        PublicApiDiff::Violation(message)
    }
}

fn is_public_api_missing(message: &str) -> bool {
    message.contains("no such command") || message.contains("cargo-public-api")
}

fn run_package_diffs(
    target: &TargetProject,
    packages: &[String],
    rules: &PubApiRules,
    report: &mut LaneReport,
) -> LaneExit {
    let context = DiffContext { target, rules };
    packages.iter().fold(LaneExit::Clean, |exit_code, package| {
        apply_public_api_diff(&context, package, report, exit_code)
    })
}

fn apply_public_api_diff(
    context: &DiffContext<'_>,
    package: &str,
    report: &mut LaneReport,
    current_exit: LaneExit,
) -> LaneExit {
    match run_public_api_diff(context.target, package) {
        PublicApiDiff::Clean => current_exit,
        PublicApiDiff::Violation(message) => {
            report.push(Finding::new(context.rules.public_api_diff.clone(), package, 0, message));
            violation_exit(current_exit)
        }
        PublicApiDiff::Failure(message) => {
            report.push(Finding::new(context.rules.public_api_tool.clone(), package, 0, message));
            LaneExit::Failure
        }
    }
}

const fn violation_exit(current: LaneExit) -> LaneExit {
    match current {
        LaneExit::Failure => LaneExit::Failure,
        _ => LaneExit::Violations,
    }
}

include!("check_public_api_diff/output.rs");

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if usage_requested(&args) {
        return usage_exit();
    }
    let rules = match PubApiRules::new() {
        Ok(rules) => rules,
        Err(error) => {
            return exit_after_stderr_line(
                &format!("[check-public-api-diff] rule id configuration error: {error}"),
                LaneExit::Failure,
            );
        }
    };
    let mut report = LaneReport::new();
    let target = match resolve_target(&mut report, &rules) {
        Ok(target) => target,
        Err(code) => return exit(code),
    };
    let packages = match resolve_package_list(&target, &mut report, &rules) {
        Ok(packages) => packages,
        Err(code) => return exit(code),
    };
    if packages.is_empty() {
        return exit(report_no_packages());
    }
    exit(run_diffs_and_emit(&target, &packages, &mut report, &rules))
}
