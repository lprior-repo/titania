//! Enumerates cargo kani harnesses for one or more packages, writes per-pkg JSON.
//!
//! Rust re-implementation of the bash lane `scripts/kani-list.sh`. Run via
//! `cargo run --bin kani_list -- <package>...` from the repository root or via
//! the matching Moon task in `.moon/tasks/all.yml`.
//!
//! Exit codes: 0 = clean, 1 = violations, 2 = usage, 3 = upstream failure.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde_json::Value;
use titania_core::TargetProject;

use titania_lanes::{
    CommandIn, Finding, LaneExit, LaneReport, RuleId, RuleIdError, current_target_project, exit,
};

const KANI_LIST_RULE: &str = "KANI_LIST_001";
const KANI_LIST_EXEC_RULE: &str = "KANI_LIST_EXEC";
const KANI_LIST_MISSING_RULE: &str = "KANI_LIST_MISSING";
const KANI_LIST_INVALID_JSON_RULE: &str = "KANI_LIST_INVALID_JSON";

/// Usage blurb emitted on `--help`.
const USAGE: &str = "usage: kani_list [<package> ...]\n\
     no package args: write target-workspace kani-list JSON to KANI_LIST_DIR/workspace.json\n\
     package args: validate package names before writing per-package scoped kani-list JSON\n\
     set KANI_FEATURES=feature1,feature2 to activate package features";

fn main() -> std::process::ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return exit_after_stderr_line(USAGE, LaneExit::Clean);
    }

    let mut report = LaneReport::new();
    let input = parse_lane_input(args);
    render_lane_result(run_lane(&input), &mut report)
}

fn render_lane_result(
    result: Result<(), LaneError>,
    report: &mut LaneReport,
) -> std::process::ExitCode {
    match result {
        Ok(()) => exit(LaneExit::Clean),
        Err(LaneError::Usage(msg)) => {
            exit_after_stderr_line(&format!("[kani_list] {msg}"), LaneExit::Usage)
        }
        Err(LaneError::Failure(msg)) => render_failure(&msg, report),
        Err(LaneError::Violation(violation)) => render_violation(*violation, report),
    }
}

fn render_failure(msg: &str, report: &mut LaneReport) -> std::process::ExitCode {
    let rule = match RuleId::new(KANI_LIST_RULE) {
        Ok(rule) => rule,
        Err(error) => {
            return exit_after_stderr_line(
                &format!("[kani_list] rule id configuration error: {error}"),
                LaneExit::Failure,
            );
        }
    };
    if write_stderr_line(&format!("[kani_list] FAIL: {msg}")).is_err() {
        return exit(LaneExit::Failure);
    }
    report.push(Finding::new(rule, "<lane>", 0, msg.to_owned()));
    exit(LaneExit::Failure)
}

fn render_violation(violation: ViolationError, report: &mut LaneReport) -> std::process::ExitCode {
    if write_stderr_line(&format!("[kani_list] {}", violation.message)).is_err() {
        return exit(LaneExit::Failure);
    }
    report.push(Finding::new(violation.rule, violation.path, violation.line, violation.message));
    exit(LaneExit::Violations)
}

/// Writes one newline-terminated line to stderr.
///
/// # Errors
///
/// Returns an I/O error when writing either the text or trailing newline fails.
fn write_stderr_line(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())?;
    stderr.write_all(b"\n")
}

fn exit_after_stderr_line(text: &str, code: LaneExit) -> std::process::ExitCode {
    match write_stderr_line(text) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}

/// Lane-local error taxonomy.
enum LaneError {
    Usage(String),
    Failure(String),
    Violation(Box<ViolationError>),
}

struct ViolationError {
    rule: RuleId,
    path: String,
    line: u32,
    message: String,
}

impl LaneError {
    fn violation(
        rule: RuleId,
        path: impl Into<String>,
        line: u32,
        message: impl Into<String>,
    ) -> Self {
        Self::Violation(Box::new(ViolationError {
            rule,
            path: path.into(),
            line,
            message: message.into(),
        }))
    }
}

/// Boundary-parsed lane input.
enum LaneInput {
    Workspace,
    Packages(Vec<String>),
}

impl From<io::Error> for LaneError {
    fn from(err: io::Error) -> Self {
        Self::Failure(format!("io error: {err}"))
    }
}

impl From<RuleIdError> for LaneError {
    fn from(err: RuleIdError) -> Self {
        Self::Failure(format!("rule id configuration error: {err}"))
    }
}

fn parse_lane_input(args: Vec<String>) -> LaneInput {
    let packages: Vec<String> = args.into_iter().filter(|a| !a.is_empty()).collect();
    if packages.is_empty() { LaneInput::Workspace } else { LaneInput::Packages(packages) }
}

/// Runs the Kani list lane for the parsed lane input.
///
/// # Errors
///
/// Returns a lane error when target discovery, output-directory creation, Kani
/// invocation, JSON validation, or output movement fails.
fn run_lane(input: &LaneInput) -> Result<(), LaneError> {
    let target = current_target_project()
        .map_err(|e| LaneError::Usage(format!("target discovery failed: {e}")))?;
    let output_dir = output_dir(&target);
    fs::create_dir_all(&output_dir)?;

    match input {
        LaneInput::Workspace => run_workspace_list(&target, &output_dir),
        LaneInput::Packages(packages) => run_package_lists(&target, &output_dir, packages),
    }
}

/// Runs `cargo kani list` for the whole target workspace.
///
/// # Errors
///
/// Returns a lane error when stale output removal, Kani execution, produced JSON
/// validation, output renaming, or status reporting fails.
fn run_workspace_list(target: &TargetProject, output_dir: &Path) -> Result<(), LaneError> {
    let target_file = output_dir.join("workspace.json");
    let produced = target.as_std_path().join("kani-list.json");
    remove_if_present(&produced)?;

    write_stderr_line(&format!("[kani-list] scope=workspace output={}", target_file.display()))?;
    let kani_status = run_kani_list(target)?;
    if !kani_status.success() {
        return Err(LaneError::violation(
            RuleId::new(KANI_LIST_EXEC_RULE)?,
            target.as_std_path().display().to_string(),
            0,
            format!("cargo kani list failed (exit {:?})", kani_status.code()),
        ));
    }
    validate_produced_json(&produced)?;
    fs::rename(&produced, &target_file)?;
    write_stderr_line(&format!(
        "KANI_LIST_OK output_dir={} scope=workspace",
        output_dir.display()
    ))?;
    Ok(())
}

/// Runs `cargo kani list` once for each requested package.
///
/// # Errors
///
/// Returns a lane error when Cargo metadata cannot be read, a package cannot be
/// resolved, Kani execution fails, produced JSON is invalid, output movement
/// fails, or status reporting fails.
fn run_package_lists(
    target: &TargetProject,
    output_dir: &Path,
    packages: &[String],
) -> Result<(), LaneError> {
    let metadata_text = run_cargo_metadata(target)?;
    let metadata: Value = serde_json::from_str(&metadata_text)
        .map_err(|e| LaneError::Failure(format!("cargo metadata parse: {e}")))?;

    packages.iter().try_for_each(|package| run_package_list(output_dir, &metadata, package))?;

    write_stderr_line(&format!(
        "KANI_LIST_OK output_dir={} packages={}",
        output_dir.display(),
        packages.join(",")
    ))?;
    Ok(())
}

/// Runs `cargo kani list` for one package and writes its JSON artifact.
///
/// # Errors
///
/// Returns a lane error when the package directory cannot be validated, stale
/// output removal fails, Kani execution fails, produced JSON is invalid, output
/// renaming fails, or status reporting fails.
fn run_package_list(output_dir: &Path, metadata: &Value, package: &str) -> Result<(), LaneError> {
    let manifest = find_manifest(metadata, package)?;
    let package_dir = manifest_dir(&manifest);
    let package_target = package_target(&package_dir)?;
    let target_file = output_dir.join(format!("{package}.json"));
    let produced = package_dir.join("kani-list.json");
    remove_if_present(&produced)?;

    write_stderr_line(&format!(
        "[kani-list] package={package} dir={} output={}",
        package_dir.display(),
        target_file.display()
    ))?;
    let kani_status = run_kani_list(&package_target)?;
    if !kani_status.success() {
        return Err(LaneError::violation(
            RuleId::new(KANI_LIST_EXEC_RULE)?,
            package_dir.display().to_string(),
            0,
            format!("cargo kani list failed (exit {:?})", kani_status.code()),
        ));
    }
    validate_produced_json(&produced)?;
    fs::rename(&produced, &target_file)?;
    write_stderr_line(&format!("[kani-list] wrote {}", target_file.display()))?;
    Ok(())
}

fn output_dir(target: &TargetProject) -> PathBuf {
    let raw = match env::var_os("KANI_LIST_DIR") {
        Some(s) if !s.is_empty() => PathBuf::from(s),
        _ => PathBuf::from(".evidence/kani-list"),
    };
    target_root_path(target, raw)
}

/// Reads Cargo metadata for the target workspace.
///
/// # Errors
///
/// Returns a lane error when command preparation fails, `cargo metadata` cannot
/// be spawned, Cargo exits unsuccessfully, or stdout is not UTF-8.
fn run_cargo_metadata(target: &TargetProject) -> Result<String, LaneError> {
    let manifest = target.manifest_path();
    let mut command = command_in(target, "cargo")?;
    let _ = command
        .arg("metadata")
        .arg("--no-deps")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(manifest.as_str());
    let output = command
        .run_capture_raw()
        .map_err(|e| LaneError::Failure(format!("failed to spawn cargo metadata: {e}")))?;
    if !output.status().success() {
        let stderr = String::from_utf8_lossy(output.stderr());
        return Err(LaneError::Failure(format!("cargo metadata failed: {stderr}")));
    }
    String::from_utf8(output.stdout().to_vec())
        .map_err(|e| LaneError::Failure(format!("cargo metadata non-UTF8: {e}")))
}

/// Finds the manifest path for a single package in Cargo metadata JSON.
///
/// # Errors
///
/// Returns a lane error when metadata lacks a package list, the package is
/// absent, the package name is duplicated, or the package has no manifest path.
fn find_manifest(metadata: &Value, package: &str) -> Result<PathBuf, LaneError> {
    let packages = metadata
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| LaneError::Failure("cargo metadata: missing 'packages'".to_string()))?;
    let matches: Vec<&Value> = packages
        .iter()
        .filter(|p| p.get("name").and_then(Value::as_str) == Some(package))
        .collect();
    match matches.len() {
        0 => Err(LaneError::Failure(format!("package '{package}' not found in workspace"))),
        1 => manifest_path_from_value(matches.first().copied(), package),
        n => Err(LaneError::Failure(format!(
            "expected exactly one package named '{package}', found {n}"
        ))),
    }
}

/// Converts the selected package metadata object into a manifest path.
///
/// # Errors
///
/// Returns a lane error when the package metadata object is missing or has no
/// string `manifest_path` field.
fn manifest_path_from_value(value: Option<&Value>, package: &str) -> Result<PathBuf, LaneError> {
    let Some(manifest) = value.and_then(|v| v.get("manifest_path")).and_then(Value::as_str) else {
        return Err(LaneError::Failure(format!("package '{package}' has no manifest_path")));
    };
    Ok(PathBuf::from(manifest))
}

fn manifest_dir(manifest: &Path) -> PathBuf {
    manifest.parent().map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// Builds a target project rooted at a selected package directory.
///
/// # Errors
///
/// Returns a lane error when the package directory is not a valid Cargo
/// project root.
fn package_target(package_dir: &Path) -> Result<TargetProject, LaneError> {
    TargetProject::try_from_path(package_dir).map_err(|error| {
        LaneError::Failure(format!("invalid package directory {}: {error}", package_dir.display()))
    })
}

/// Starts `cargo kani list` and returns its exit status.
///
/// # Errors
///
/// Returns a lane error when command preparation fails or `cargo kani` cannot
/// be spawned.
fn run_kani_list(target: &TargetProject) -> Result<std::process::ExitStatus, LaneError> {
    let features = env::var_os("KANI_FEATURES").map(|value| value.to_string_lossy().into_owned());
    let mut command = command_in(target, "cargo")?;
    let _ = command.arg("kani").arg("list").arg("--format").arg("json");
    if let Some(features) = features.as_deref().filter(|value| !value.is_empty()) {
        let _ = command.arg("--features").arg(features);
    }
    command
        .run_status_raw()
        .map_err(|e| LaneError::Failure(format!("failed to spawn cargo kani: {e}")))
}

fn target_root_path(target: &TargetProject, path: PathBuf) -> PathBuf {
    if path.is_absolute() { path } else { target.as_std_path().join(path) }
}

/// Prepares a command within the target project and inherits the shell environment.
///
/// # Errors
///
/// Returns a lane error when the command cannot be prepared for the target.
fn command_in<'a>(target: &'a TargetProject, program: &'a str) -> Result<CommandIn<'a>, LaneError> {
    let mut command = CommandIn::new(target, program)
        .map_err(|e| LaneError::Failure(format!("failed to prepare {program}: {e}")))?;
    let _ = command.inherit_env();
    Ok(command)
}

/// Validates that a produced Kani list JSON artifact exists and parses.
///
/// # Errors
///
/// Returns a lane error when the file is missing or empty, cannot be read, the
/// validation rule ID is invalid, or the file does not contain valid JSON.
fn validate_produced_json(produced: &Path) -> Result<(), LaneError> {
    if !is_non_empty(produced) {
        return Err(LaneError::violation(
            RuleId::new(KANI_LIST_MISSING_RULE)?,
            produced.display().to_string(),
            0,
            format!("cargo kani list did not produce {}", produced.display()),
        ));
    }

    let raw = fs::read_to_string(produced)?;
    let invalid_json_rule = RuleId::new(KANI_LIST_INVALID_JSON_RULE)?;
    validate_json(&raw).map_err(move |e| {
        LaneError::violation(
            invalid_json_rule,
            produced.display().to_string(),
            0,
            format!("invalid JSON in {}: {e}", produced.display()),
        )
    })
}

/// Removes a stale file if it exists.
///
/// # Errors
///
/// Returns a lane error when the file exists but cannot be removed.
fn remove_if_present(path: &Path) -> Result<(), LaneError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(LaneError::Failure(format!("failed to remove {}: {err}", path.display()))),
    }
}

fn is_non_empty(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|m| m.len() > 0)
}

/// Parses raw JSON text enough to prove it is syntactically valid JSON.
///
/// # Errors
///
/// Returns the parse error text when `raw` is not valid JSON.
fn validate_json(raw: &str) -> Result<(), String> {
    serde_json::from_str::<Value>(raw).map(|_| ()).map_err(|e| e.to_string())
}
