//! Diffs `cargo public-api` for every `vb_*` package against `origin/main`.
//!
//! Rust re-implementation of the bash lane in
//! `velvet-ballistics/scripts/check-public-api-diff.sh`. Run via
//! `cargo run --bin check-public-api-diff --` from the repository root or via
//! the matching Moon task in `.moon/tasks/all.yml`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

#[path = "check_public_api_diff/package_json.rs"]
mod package_json;

use titania_core::TargetProject;
use titania_lanes::{CommandIn, CommandOutput, Finding, LaneError, LaneExit, LaneReport, current_target_project, exit};

use package_json::extract_package_names;

// (Rule constants that may be reintroduced when toolchains are missing:
//  see `RULE_PUBLIC_API_DIFF` and `RULE_PUBLIC_API_TOOL` below.)
const RULE_METADATA: &str = "PUBAPI-METADATA-001";
const RULE_PUBLIC_API_DIFF: &str = "PUBAPI-DIFF-001";
const RULE_PUBLIC_API_TOOL: &str = "PUBAPI-TOOL-001";
const RULE_TARGET: &str = "PUBAPI-TARGET-001";
const TOOLCHAIN: &str = "nightly-2026-04-28";

fn filter_packages(discovered: Vec<String>) -> Vec<String> {
    discovered.into_iter().filter(|name| name.starts_with("vb_")).collect()
}

/// Runs `cargo metadata --format-version 1 --no-deps` and extracts the
/// `vb_*` package names from its JSON output.
///
/// # Errors
/// Returns the formatted `cargo metadata` failure when the subprocess
/// cannot be started, returns non-success, or returns non-UTF8 JSON.
fn discover_packages(target: &TargetProject) -> Result<Vec<String>, String> {
    let manifest = target.manifest_path();
    let mut command = CommandIn::new(target, "cargo")
        .map_err(|error| format!("cargo metadata failed to start: {error}"))?;
    command.inherit_env();
    command
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .arg("--manifest-path")
        .arg(manifest.as_str());
    let output = command
        .run_capture_raw()
        .map_err(|error| format!("cargo metadata failed to start: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo metadata failed: {stderr}"));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|error| format!("cargo metadata returned non-UTF8 JSON: {error}"))?;
    Ok(filter_packages(extract_package_names(&text)))
}

enum PublicApiDiff {
    Clean,
    Violation(String),
    Failure(String),
}

fn run_public_api_diff<'a>(
    target: &'a TargetProject,
    package: &'a str,
    manifest: &'a str,
) -> PublicApiDiff {
    let mut command = match CommandIn::new(target, "rustup") {
        Ok(command) => command,
        Err(error) => return PublicApiDiff::Failure(format!("rustup command invalid: {error}")),
    };
    command.inherit_env();
    add_public_api_args(&mut command, package, manifest);
    classify_public_api_output(package, command.run_capture_raw())
}

fn add_public_api_args<'a>(command: &mut CommandIn<'a>, package: &'a str, manifest: &'a str) {
    command
        .arg("run")
        .arg(TOOLCHAIN)
        .arg("cargo")
        .arg("public-api")
        .arg("--manifest-path")
        .arg(manifest)
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
    output: Result<CommandOutput, LaneError>,
) -> PublicApiDiff {
    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return PublicApiDiff::Failure(format!(
                "cargo public-api failed for {package}: {error}"
            ));
        }
    };
    if output.status.success() {
        return PublicApiDiff::Clean;
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let message = format!(
        "cargo public-api diff failed for {package} with code {:?}\n{stdout}{stderr}",
        output.status.code()
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
    manifest: &str,
    report: &mut LaneReport,
) -> LaneExit {
    let mut exit_code = LaneExit::Clean;
    for package in packages {
        update_exit_code(&mut exit_code, run_public_api_diff(target, package, manifest), package, report);
    }
    exit_code
}

/// Records a single `PublicApiDiff` result into the lane report and
/// folds it into the running `LaneExit` (extracted to keep
/// `run_package_diffs` at one level of nesting per arm).
fn update_exit_code(
    exit_code: &mut LaneExit,
    diff: PublicApiDiff,
    package: &str,
    report: &mut LaneReport,
) {
    match diff {
        PublicApiDiff::Clean => {}
        PublicApiDiff::Violation(message) => {
            report.push(Finding::new(RULE_PUBLIC_API_DIFF, package, 0, message));
            if *exit_code == LaneExit::Clean {
                *exit_code = LaneExit::Violations;
            }
        }
        PublicApiDiff::Failure(message) => {
            report.push(Finding::new(RULE_PUBLIC_API_TOOL, package, 0, message));
            *exit_code = LaneExit::Failure;
        }
    }
}

fn usage_requested(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

fn emit_usage() {
    eprintln!(
        "usage: check-public-api-diff\n\
         Discovers vb_* workspace packages and runs\n\
         `cargo public-api diff origin/main..HEAD` through CommandIn."
    );
}

/// Resolves the current target project, recording a typed finding on
/// failure and returning the `LaneExit` the lane should use.
///
/// # Errors
/// Returns the `LaneExit` to use when `current_target_project()` fails
/// after recording a `PUBAPI-TARGET-001` finding in `report`.
fn resolve_target(report: &mut LaneReport) -> Result<TargetProject, LaneExit> {
    match current_target_project() {
        Ok(target) => Ok(target),
        Err(error) => {
            report.push(Finding::new(RULE_TARGET, ".", 0, format!("target discovery failed: {error}")));
            Err(LaneExit::Usage)
        }
    }
}

/// Discovers the `vb_*` package list, recording a typed finding on
/// failure.
///
/// # Errors
/// Returns `LaneExit::Usage` after recording a `PUBAPI-METADATA-001`
/// finding when package discovery fails.
fn resolve_package_list(
    target: &TargetProject,
    report: &mut LaneReport,
) -> Result<Vec<String>, LaneExit> {
    match discover_packages(target) {
        Ok(packages) => Ok(packages),
        Err(error) => {
            report.push(Finding::new(RULE_METADATA, ".", 0, error));
            Err(LaneExit::Usage)
        }
    }
}

fn report_no_packages() -> LaneExit {
    eprintln!("[check-public-api-diff] no vb_* packages to diff against origin/main");
    LaneExit::NotApplicable
}

fn run_diffs_and_emit(
    target: &TargetProject,
    packages: &[String],
    report: &mut LaneReport,
) -> LaneExit {
    if packages.is_empty() {
        return report_no_packages();
    }
    let manifest = target.manifest_path();
    run_package_diffs(target, packages, manifest.as_str(), report)
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if usage_requested(&args) {
        emit_usage();
        return exit(LaneExit::Clean);
    }
    let mut report = LaneReport::new();
    let target = match resolve_target(&mut report) {
        Ok(target) => target,
        Err(code) => return exit(code),
    };
    let packages = match resolve_package_list(&target, &mut report) {
        Ok(packages) => packages,
        Err(code) => return exit(code),
    };
    let code = run_diffs_and_emit(&target, &packages, &mut report);
    eprint!("{}", report.render());
    if matches!(code, LaneExit::Clean) {
        eprintln!("[check-public-api-diff] no public-api diff violations");
    }
    exit(code)
}
