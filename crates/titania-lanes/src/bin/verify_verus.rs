//! Runs verus over the production proof registry and emits trust-boundary reports.
//!
//! `contracts/proof_obligations.yaml` is authoritative only when it contains at
//! least one production proof obligation. The `formal_setup_smoke.rs` fixture may
//! remain in-tree as a Verus installation probe, but a smoke-only registry exits
//! with usage status instead of reporting production proof success.
//!
//! Strict Holzman Rust: no `unwrap`, no `expect`, no `panic`, no `unsafe`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::string_slice)]
#![deny(clippy::get_unwrap)]
#![deny(clippy::arithmetic_side_effects)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::as_conversions)]
#![forbid(unsafe_code)]

#[path = "verify_verus/diagnostics.rs"]
/// Diagnostic helpers for lane exits and stderr reporting.
pub mod diagnostics;
#[path = "verify_verus/evidence.rs"]
/// Evidence-file helpers for the Verus lane.
pub mod evidence;
#[path = "verify_verus/outcome.rs"]
/// Verus lane outcome orchestration.
pub mod outcome;
#[path = "verify_verus/registry.rs"]
/// Proof-obligation registry parsing.
pub mod registry;
#[path = "verify_verus/target_outcome.rs"]
/// Verus target execution and failure recording.
pub mod target_outcome;
#[path = "verify_verus/trust.rs"]
/// Trust-boundary scans for Verus artifacts.
pub mod trust;
#[path = "verify_verus/verus_tool.rs"]
/// Verus process execution helpers.
pub mod verus_tool;
#[path = "verify_verus/walk.rs"]
/// Recursive Rust source walking helpers.
pub mod walk;

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use diagnostics::{err_after_stderr, lane_after_stderr, write_stderr_line};
use outcome::run_production_targets;
use registry::ProofTarget;
use titania_core::TargetProject;
use titania_lanes::{LaneExit, LaneReport, current_target_project, exit};

const REGISTRY_DEFAULT: &str = "contracts/proof_obligations.yaml";
const EVIDENCE_DIR_DEFAULT: &str = ".evidence/verus";
const SUMMARY_FILE: &str = "summary.txt";

struct LanePaths {
    registry: PathBuf,
    evidence_dir: PathBuf,
}

struct VerificationInputs {
    targets: Vec<ProofTarget>,
    summary_path: PathBuf,
}

fn main() -> std::process::ExitCode {
    let mut report = LaneReport::new();
    match run(&mut report) {
        LaneExit::Clean => exit(LaneExit::Clean),
        LaneExit::Violations => match write_stderr_line(format_args!("{}", report.render())) {
            Ok(()) => exit(LaneExit::Violations),
            Err(_) => exit(LaneExit::Failure),
        },
        other => exit(other),
    }
}

fn run(report: &mut LaneReport) -> LaneExit {
    match run_checked(report) {
        Ok(exit_code) | Err(exit_code) => exit_code,
    }
}

/// Run the lane after converting setup failures into lane exits.
///
/// # Errors
///
/// Returns a lane exit when target discovery, tool discovery, registry input,
/// or evidence setup fails.
fn run_checked(report: &mut LaneReport) -> Result<LaneExit, LaneExit> {
    let target = resolve_target()?;
    ensure_verus_on_path(&target)?;
    let paths = lane_paths(&target);
    let inputs = prepare_inputs(&target, &paths)?;
    if registry::contains_only_fixture_smoke(&inputs.targets) {
        return Ok(smoke_only_usage(&inputs.summary_path));
    }
    Ok(run_production_targets(report, &target, &paths.evidence_dir, &inputs))
}

/// Resolve the current target project.
///
/// # Errors
///
/// Returns usage or failure when the target project cannot be discovered or
/// the diagnostic cannot be written.
fn resolve_target() -> Result<TargetProject, LaneExit> {
    match current_target_project() {
        Ok(target) => Ok(target),
        Err(error) => err_after_stderr(
            format_args!("[verify-verus] target discovery failed: {error}"),
            LaneExit::Usage,
        ),
    }
}

/// Ensure the Verus binary is runnable from the target environment.
///
/// # Errors
///
/// Returns failure when Verus is unavailable or the diagnostic cannot be
/// written.
fn ensure_verus_on_path(target: &TargetProject) -> Result<(), LaneExit> {
    if verus_tool::verus_on_path(target) {
        return Ok(());
    }
    err_after_stderr(
        format_args!("[verify-verus] verus not on PATH; formal verification cannot run"),
        LaneExit::Failure,
    )
}

/// Load and validate lane inputs.
///
/// # Errors
///
/// Returns a lane exit when the registry, evidence directory, registry
/// targets, or initial summary cannot be prepared.
fn prepare_inputs(
    target: &TargetProject,
    paths: &LanePaths,
) -> Result<VerificationInputs, LaneExit> {
    ensure_registry_has_content(&paths.registry)?;
    ensure_evidence_dir(&paths.evidence_dir)?;
    let targets = load_registry_targets(target, &paths.registry)?;
    ensure_targets_present(&targets, &paths.registry)?;
    let summary_path = paths.evidence_dir.join(SUMMARY_FILE);
    write_initial_summary(&summary_path, targets.len())?;
    Ok(VerificationInputs { targets, summary_path })
}

/// Ensure the registry path exists and is non-empty.
///
/// # Errors
///
/// Returns usage when the registry is missing/empty, or failure when the
/// diagnostic cannot be written.
fn ensure_registry_has_content(registry: &Path) -> Result<(), LaneExit> {
    if registry::registry_path_is_nonempty(registry) {
        return Ok(());
    }
    err_after_stderr(
        format_args!(
            "[verify-verus] registry missing or empty: {}; formal obligations are required",
            registry.display()
        ),
        LaneExit::Usage,
    )
}

/// Create the evidence directory.
///
/// # Errors
///
/// Returns failure when the directory cannot be created or the diagnostic
/// cannot be written.
fn ensure_evidence_dir(evidence_dir: &Path) -> Result<(), LaneExit> {
    fs::create_dir_all(evidence_dir).map_err(|e| {
        lane_after_stderr(
            format_args!("[verify-verus] cannot create evidence dir: {e}"),
            LaneExit::Failure,
        )
    })
}

/// Parse proof targets from the registry.
///
/// # Errors
///
/// Returns failure when the registry cannot be parsed or the diagnostic
/// cannot be written.
fn load_registry_targets(
    target: &TargetProject,
    registry: &Path,
) -> Result<Vec<ProofTarget>, LaneExit> {
    registry::parse_registry_targets(registry, target).map_err(|e| {
        lane_after_stderr(
            format_args!("[verify-verus] registry parse failed: {e}"),
            LaneExit::Failure,
        )
    })
}

/// Ensure the registry produced at least one target.
///
/// # Errors
///
/// Returns usage when no targets are present, or failure when the diagnostic
/// cannot be written.
fn ensure_targets_present(targets: &[ProofTarget], registry: &Path) -> Result<(), LaneExit> {
    if !targets.is_empty() {
        return Ok(());
    }
    err_after_stderr(
        format_args!(
            "[verify-verus] no targets discovered in {}; formal obligations are required",
            registry.display()
        ),
        LaneExit::Usage,
    )
}

/// Write the initial evidence summary.
///
/// # Errors
///
/// Returns failure when the summary cannot be written or the diagnostic cannot
/// be emitted.
fn write_initial_summary(summary_path: &Path, target_count: usize) -> Result<(), LaneExit> {
    evidence::write_summary_header(summary_path, target_count).map_err(|e| {
        lane_after_stderr(
            format_args!("[verify-verus] cannot write summary: {e}"),
            LaneExit::Failure,
        )
    })
}

fn lane_paths(target: &TargetProject) -> LanePaths {
    LanePaths {
        registry: target_path(target, &env_value("VERUS_PROOF_REGISTRY", REGISTRY_DEFAULT)),
        evidence_dir: target_path(target, &env_value("VERUS_EVIDENCE_DIR", EVIDENCE_DIR_DEFAULT)),
    }
}

fn env_value(key: &str, default: &str) -> String {
    let Ok(value) = env::var(key) else {
        return default.to_owned();
    };
    value
}

fn target_path(target: &TargetProject, raw: &str) -> PathBuf {
    let path = Path::new(raw);
    if path.is_absolute() { path.to_path_buf() } else { target.as_std_path().join(path) }
}

fn smoke_only_usage(summary_path: &Path) -> LaneExit {
    if write_stderr_line(format_args!(
        "[verify-verus] only fixture smoke obligations discovered; production Verus obligations are required"
    ))
    .is_err()
    {
        return LaneExit::Failure;
    }
    if let Err(e) = evidence::append_not_applicable(summary_path, "fixture-smoke-only") {
        return lane_after_stderr(
            format_args!("[verify-verus] cannot append smoke-only summary: {e}"),
            LaneExit::Failure,
        );
    }
    LaneExit::Usage
}
