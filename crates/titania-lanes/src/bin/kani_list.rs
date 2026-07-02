//! Enumerates cargo kani harnesses for one or more packages, writes per-pkg JSON.
//!
//! Rust re-implementation of the bash lane in
//! `velvet-ballistics/scripts/kani-list.sh`. Run via
//! `cargo run --bin kani_list -- <package>...` from the repository root or via
//! the matching Moon task in `.moon/tasks/all.yml`.
//!
//! Exit codes: 0 = clean, 1 = violations, 2 = usage, 3 = upstream failure.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use serde_json::Value;
use titania_core::TargetProject;

use titania_lanes::{CommandIn, Finding, LaneExit, LaneReport, current_target_project, exit};

/// Usage blurb emitted on `--help`.
const USAGE: &str = "usage: kani_list [<package> ...]\n\
     no package args: write target-workspace kani-list JSON to KANI_LIST_DIR/workspace.json\n\
     package args: validate package names before writing per-package scoped kani-list JSON\n\
     set KANI_FEATURES=feature1,feature2 to activate package features";

fn main() -> std::process::ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!("{USAGE}");
        return exit(LaneExit::Clean);
    }

    let mut report = LaneReport::new();
    let input = parse_lane_input(args);
    match run_lane(&input) {
        Ok(()) => exit(LaneExit::Clean),
        Err(LaneError::Usage(msg)) => {
            eprintln!("[kani_list] {msg}");
            exit(LaneExit::Usage)
        }
        Err(LaneError::Failure(msg)) => {
            let finding = Finding::new("KANI-LIST-001", "<lane>", 0, msg.clone());
            eprintln!("[kani_list] FAIL: {msg}");
            report.push(finding);
            exit(LaneExit::Failure)
        }
        Err(LaneError::Violation(details)) => {
            let finding = Finding::new(
                details.rule,
                details.path.clone(),
                details.line,
                details.message.clone(),
            );
            eprintln!("[kani_list] {}", details.message);
            report.push(finding);
            exit(LaneExit::Violations)
        }
    }
}

/// Lane-local error taxonomy. `Violation` carries a boxed [`Violation`]
/// struct so the variant's stack footprint stays under the 64-byte
/// `result_large_err` threshold.
enum LaneError {
    Usage(String),
    Failure(String),
    Violation(Box<Violation>),
}

/// Per-finding payload carried by `LaneError::Violation`.
struct Violation {
    rule: &'static str,
    path: String,
    line: u32,
    message: String,
}

impl From<io::Error> for LaneError {
    fn from(err: io::Error) -> Self {
        Self::Failure(format!("io error: {err}"))
    }
}

/// Boundary-parsed lane input.
enum LaneInput {
    Workspace,
    Packages(Vec<String>),
}

fn parse_lane_input(args: Vec<String>) -> LaneInput {
    let packages: Vec<String> = args.into_iter().filter(|a| !a.is_empty()).collect();
    if packages.is_empty() { LaneInput::Workspace } else { LaneInput::Packages(packages) }
}

/// Top-level lane dispatcher. Targets the workspace list or one
/// per-package list depending on `input`.
///
/// # Errors
/// Returns `LaneError::Usage` when `current_target_project()` fails.
/// Returns `LaneError::Failure` for I/O or cargo subprocess errors.
/// Returns `LaneError::Violation` for kani-list subprocess failures
/// or missing/invalid produced JSON.
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

/// Runs `cargo kani list` once for the workspace and writes
/// `kani-list.json` into the lane output directory.
///
/// # Errors
/// Returns `LaneError::Failure` on I/O, and `LaneError::Violation`
/// when `cargo kani list` exits non-success or its JSON output is
/// missing/invalid.
fn run_workspace_list(target: &TargetProject, output_dir: &Path) -> Result<(), LaneError> {
    let target_file = output_dir.join("workspace.json");
    let produced = target.as_std_path().join("kani-list.json");
    remove_if_present(&produced)?;

    eprintln!("[kani-list] scope=workspace output={}", target_file.display());
    let kani_status = run_kani_list(target, None)?;
    if !kani_status.success() {
        return Err(LaneError::Violation(Box::new(Violation {
            rule: "KANI-LIST-EXEC",
            path: target.as_std_path().display().to_string(),
            line: 0,
            message: format!("cargo kani list failed (exit {:?}", kani_status.code()),
        })));
    }
    validate_produced_json(&produced)?;
    fs::rename(&produced, &target_file)?;
    eprintln!("KANI_LIST_OK output_dir={} scope=workspace", output_dir.display());
    Ok(())
}

/// Runs `cargo kani list` once per package and writes one JSON file
/// per package into the lane output directory.
///
/// # Errors
/// Returns `LaneError::Failure` for I/O or cargo-metadata errors
/// (missing workspace, duplicate package, bad manifest). Returns
/// `LaneError::Violation` for kani-list subprocess failures or
/// missing/invalid produced JSON.
fn run_package_lists(
    target: &TargetProject,
    output_dir: &Path,
    packages: &[String],
) -> Result<(), LaneError> {
    let metadata_text = run_cargo_metadata(target)?;
    let metadata: Value = serde_json::from_str(&metadata_text)
        .map_err(|e| LaneError::Failure(format!("cargo metadata parse: {e}")))?;

    for package in packages {
        write_package_list(target, output_dir, &metadata, package)?;
    }

    eprintln!("KANI_LIST_OK output_dir={} packages={}", output_dir.display(), packages.join(","));
    Ok(())
}

/// Runs `cargo kani list` for one package, validates the produced
/// JSON, and renames it into the lane output directory.
///
/// # Errors
/// Returns `LaneError::Failure` for I/O (via the `io::Error` `From`
/// impl) and metadata lookups. Returns `LaneError::Violation` for
/// kani-list subprocess failures or missing/invalid produced JSON.
fn write_package_list(
    target: &TargetProject,
    output_dir: &Path,
    metadata: &Value,
    package: &str,
) -> Result<(), LaneError> {
    let manifest = find_manifest(metadata, package)?;
    let package_dir = manifest_dir(&manifest);
    let target_file = output_dir.join(format!("{package}.json"));
    let produced = target.as_std_path().join("kani-list.json");
    remove_if_present(&produced)?;

    eprintln!(
        "[kani-list] package={package} dir={} output={}",
        package_dir.display(),
        target_file.display()
    );
    let kani_status = run_kani_list(target, Some(&manifest))?;
    if !kani_status.success() {
        return Err(LaneError::Violation(Box::new(Violation {
            rule: "KANI-LIST-EXEC",
            path: package_dir.display().to_string(),
            line: 0,
            message: format!("cargo kani list failed (exit {:?}", kani_status.code()),
        })));
    }
    validate_produced_json(&produced)?;
    fs::rename(&produced, &target_file)?;
    eprintln!("[kani-list] wrote {}", target_file.display());
    Ok(())
}

/// Resolves the lane output directory from `KANI_LIST_DIR` (or the
/// default `.evidence/kani-list` under the target root).
fn output_dir(target: &TargetProject) -> PathBuf {
    let raw = match env::var_os("KANI_LIST_DIR") {
        Some(s) if !s.is_empty() => PathBuf::from(s),
        _ => PathBuf::from(".evidence/kani-list"),
    };
    target_root_path(target, raw)
}

/// Runs `cargo metadata --no-deps --format-version 1` and returns
/// its stdout.
///
/// # Errors
/// Returns `LaneError::Failure` when the subprocess cannot be
/// spawned, exits non-success, or returns non-UTF8 stdout.
fn run_cargo_metadata(target: &TargetProject) -> Result<String, LaneError> {
    let manifest = target.manifest_path();
    let mut command = command_in(target, "cargo")?;
    command
        .arg("metadata")
        .arg("--no-deps")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(manifest.as_str());
    let output = command
        .run_capture_raw()
        .map_err(|e| LaneError::Failure(format!("failed to spawn cargo metadata: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LaneError::Failure(format!("cargo metadata failed: {stderr}")));
    }
    String::from_utf8(output.stdout)
        .map_err(|e| LaneError::Failure(format!("cargo metadata non-UTF8: {e}")))
}

/// Looks up `package`'s `manifest_path` in the cargo metadata JSON.
///
/// # Errors
/// Returns `LaneError::Failure` when the metadata is missing the
/// `packages` field, when no package matches, when the matching
/// package has no `manifest_path`, or when multiple packages share
/// the requested name.
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
        1 => {
            let manifest = matches
                .first()
                .and_then(|v| v.get("manifest_path"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    LaneError::Failure(format!("package '{package}' has no manifest_path"))
                })?;
            Ok(PathBuf::from(manifest))
        }
        n => Err(LaneError::Failure(format!(
            "expected exactly one package named '{package}', found {n}"
        ))),
    }
}

fn manifest_dir(manifest: &Path) -> PathBuf {
    manifest
        .parent()
        .map_or_else(|| PathBuf::from("."), std::path::Path::to_path_buf)
}

/// Runs `cargo kani list` and returns the subprocess `ExitStatus`.
///
/// # Errors
/// Returns `LaneError::Failure` for subprocess spawn errors or for
/// non-UTF-8 manifest paths passed via `--manifest-path`.
fn run_kani_list(
    target: &TargetProject,
    manifest: Option<&Path>,
) -> Result<std::process::ExitStatus, LaneError> {
    let features = env::var_os("KANI_FEATURES").map(|value| value.to_string_lossy().into_owned());
    let mut command = command_in(target, "cargo")?;
    command.arg("kani").arg("list").arg("--format").arg("json");
    if let Some(manifest) = manifest {
        let manifest = manifest.to_str().ok_or_else(|| {
            LaneError::Failure(format!("package manifest is not UTF-8: {}", manifest.display()))
        })?;
        command.arg("--manifest-path").arg(manifest);
    }
    if let Some(features) = features.as_deref().filter(|value| !value.is_empty()) {
        command.arg("--features").arg(features);
    }
    command
        .run_status_raw()
        .map_err(|e| LaneError::Failure(format!("failed to spawn cargo kani: {e}")))
}

fn target_root_path(target: &TargetProject, path: PathBuf) -> PathBuf {
    if path.is_absolute() { path } else { target.as_std_path().join(path) }
}

/// Builds a `CommandIn` for `program` against `target` with the
/// inherited environment.
///
/// # Errors
/// Returns `LaneError::Failure` when `CommandIn::new` rejects
/// `program` (e.g. the binary is not on `PATH`).
fn command_in<'a>(target: &'a TargetProject, program: &'a str) -> Result<CommandIn<'a>, LaneError> {
    let mut command = CommandIn::new(target, program)
        .map_err(|e| LaneError::Failure(format!("failed to prepare {program}: {e}")))?;
    command.inherit_env();
    Ok(command)
}

/// Validates the `kani-list.json` produced by `cargo kani list`:
/// non-empty file and parses as JSON.
///
/// # Errors
/// Returns `LaneError::Violation` when the file is missing/empty or
/// contains invalid JSON. Returns `LaneError::Failure` (via the
/// `io::Error` `From` impl) when the file cannot be read.
fn validate_produced_json(produced: &Path) -> Result<(), LaneError> {
    if !is_non_empty(produced) {
        return Err(LaneError::Violation(Box::new(Violation {
            rule: "KANI-LIST-MISSING",
            path: produced.display().to_string(),
            line: 0,
            message: format!("cargo kani list did not produce {}", produced.display()),
        })));
    }

    let raw = fs::read_to_string(produced)?;
    validate_json(&raw).map_err(|e| {
        LaneError::Violation(Box::new(Violation {
            rule: "KANI-LIST-INVALID-JSON",
            path: produced.display().to_string(),
            line: 0,
            message: format!("invalid JSON in {}: {e}", produced.display()),
        }))
    })
}

/// Removes a file if it exists, treating a missing file as success.
///
/// # Errors
/// Returns the underlying `io::Error` when `fs::remove_file` fails
/// for any reason other than the file already being absent.
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

/// Parses `raw` as JSON and returns the parse error (if any) as a
/// `String` for inclusion in the lane report.
///
/// # Errors
/// Returns `Err(String)` carrying the `serde_json::Error` text when
/// `raw` is not valid JSON.
fn validate_json(raw: &str) -> Result<(), String> {
    serde_json::from_str::<Value>(raw).map(|_| ()).map_err(|e| e.to_string())
}
