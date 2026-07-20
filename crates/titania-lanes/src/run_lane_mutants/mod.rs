//! Cargo-mutants test-survivor lane (v1.5, Full scope).
//!
//! Per `.evidence/v1.5/spec.md §4.3`, the lane runs the locked-spec
//! **workspace-level full test-mode command** — a single invocation of
//! `cargo mutants --no-shuffle --output <dir> --workspace` — instead of
//! fanning out per package with synthetic multi-package evidence. The
//! invocation is wrapped in a
//! `systemd-run --user --scope -p MemoryHigh=20G -p MemoryMax=24G
//! -p MemorySwapMax=0 --` cgroup scope when `systemd-run` is on PATH and
//! the binary responds to `--version`; when `systemd-run` is absent the
//! lane falls back to the bare cargo-mutants invocation and records the
//! fallback policy in the `LaneEvidence::command` argv (per the safe
//! fallback policy).
//!
//! Spec-mandated JSON/output flags for cargo-mutants 27 are preserved
//! on every invocation: `-o` / `--output <dir>` routes the JSON
//! artifacts (always emitted by cargo-mutants 27 regardless of any
//! `--json` flag) and `--no-shuffle` orders mutants deterministically.
//! The `--workspace` flag confirms full-workspace scope (cargo-mutants
//! 27 also defaults to workspace scope when run from a workspace root;
//! we pass it explicitly for evidence clarity).
//!
//! The lane is split across cohesive submodules:
//!
//! - `constants` — locked-spec string literals and rule ids.
//! - `state` — `LaneRunState`, `MutantsReport`, `NewSurvivor` data
//!   types.
//! - `version` — `cargo mutants --version` probe and major parser.
//! - `command` — cgroup-wrapped and bare `Command` builders plus
//!   `run_workspace_command`.
//! - `baseline` — baseline path resolution and typed load.
//! - `report` — workspace-level artifact resolution and typed
//!   `MutantsOutcomes` / `MutantsRecords` decoding.
//! - `operators` — v1.5 closed-set operator classifier and
//!   `relative_mutant_path`.
//! - `survivors` — per-survivor classifier and baseline diff.
//! - `outcomes` — clean, baseline-missing, and survivor findings
//!   construction.
//!
//! Output flow (encoded in the lane's `outcome` orchestrator):
//!
//! 1. Probe `cargo mutants --version`; on missing binary, no output,
//!    or an unparseable version line the lane returns
//!    `LaneOutcome::Skipped { reason:
//!    SkipReason::ToolUnavailable(ToolKind::CargoMutants) }`.
//! 2. Probe the reported major version against the v1.5 spec floor
//!    of `25`; sub-floor majors (e.g. `cargo-mutants 24.5.0`) also map
//!    to `Skipped { reason: ToolUnavailable(CargoMutants) }` so old
//!    binaries cannot silently downgrade evidence.
//! 3. Probe `systemd-run --version`; record whether the cgroup wrapper
//!    will be applied.
//! 4. Load `.titania/profiles/strict-ai/mutants.baseline.json`. A
//!    missing file produces the spec-typed `MUTANT_BASELINE_MISSING`
//!    reject finding (catalog row 80) instead of a dead catalog text;
//!    this surfaces the operator reminder to run
//!    `scripts/dev/mutants-bootstrap.sh` directly in the lane outcome.
//! 5. Spawn the single workspace-level `cargo mutants ...` invocation,
//!    wait up to the lane's wallclock cap (`MUTANTS_WALLCLOCK_TIMEOUT_SECS`)
//!    for completion, and reap the child on timeout.
//! 6. Parse `<dir>/mutants.out/outcomes.json` and `.../mutants.json`
//!    via the typed core parsers. Every
//!    `summary == "MissedMutant"` whose `Mutant.name` is not in the
//!    baseline becomes one typed `MUTANT_SURVIVED` reject finding
//!    carrying the real `<file:line:col>` location and a `MutantId`
//!    built via `MutantId::new`.
//! 7. Cargo-mutants genre ↔ `MutantOperator` mapping fails *closed*:
//!    any `BinaryOperator` whose textual name does not match a
//!    recognised pattern produces
//!    `MutantsLaneError::UnknownOperator` rather than coercing to
//!    `ArithmeticOpFlip`. The lane surfaces this as a typed
//!    `LaneFailure::Infra { tool: "cargo-mutants", reason: ... }` so
//!    the gate fails loud instead of silently swallowing a new
//!    operator the spec closed set has not been amended for.
//! 8. Zero new survivors produces `LaneOutcome::Clean { evidence }`
//!    whose `argv` reflects the actual command that ran (cgroup
//!    wrapper or bare cargo-mutants).

mod baseline;
mod command;
mod constants;
pub mod error;
mod operators;
mod outcomes;
mod report;
mod state;
mod survivors;
mod version;

use std::time::{SystemTime, UNIX_EPOCH};

use titania_core::{Finding, LaneOutcome, SkipReason, TargetProject, ToolKind};

pub(super) use error::MutantsLaneError;
use state::LaneRunState;

use self::{
    baseline::{baseline_path, load_baseline},
    command::run_workspace_command,
    outcomes::{
        baseline_missing_outcome, build_clean_outcome, map_outcome_error, survivor_finding,
    },
    report::{read_workspace_report, validate_report_exit},
    survivors::build_new_survivors,
    version::{
        detect_cargo_mutants_version, major_meets_floor, parse_cargo_mutants_major,
        probe_systemd_run,
    },
};

/// Run the cargo-mutants test-survivor lane.
///
/// # Errors
///
/// Returns [`MutantsLaneError`] for every typed failure the lane can
/// surface: tool presence/version, baseline read/parse, cargo-mutants
/// spawn/wait/kill/reap/artifact/parse, and typed operator /
/// geometry / id-classification failures. A missing baseline is **not**
/// propagated to the caller — the lane emits
/// `LaneOutcome::Findings { [MUTANT_BASELINE_MISSING] }` directly so
/// the operator reminder reaches the receipt. A missing or below-floor
/// cargo-mutants binary is converted into
/// `LaneOutcome::Skipped { reason:
/// SkipReason::ToolUnavailable(ToolKind::CargoMutants) }` so the
/// aggregate gate does not flag the host's tool absence as a
/// release-blocking internal error.
pub(super) fn outcome(target: &TargetProject) -> Result<LaneOutcome, MutantsLaneError> {
    let workspace_root = target.as_std_path();

    let Some(tool_version) = detect_cargo_mutants_version() else {
        return Ok(tool_unavailable_outcome());
    };
    let Some(major_version) = parse_cargo_mutants_major(&tool_version) else {
        return Ok(tool_unavailable_outcome());
    };
    if !major_meets_floor(major_version) {
        return Ok(tool_unavailable_outcome());
    }

    let baseline_path_value = baseline_path(workspace_root);
    if let Err(MutantsLaneError::BaselineMissing(label)) = load_baseline(&baseline_path_value) {
        return Ok(baseline_missing_outcome(&label));
    }

    let state = LaneRunState { tool_version, cgroup_used: probe_systemd_run() };

    let exit_code = run_workspace_command(workspace_root, state.cgroup_used)?;
    let report = read_workspace_report(workspace_root, exit_code)?;
    validate_report_exit(&report)?;

    let now_unix = current_unix();
    let new_survivors = build_new_survivors(&report.survivor_names, workspace_root, now_unix)?;
    if new_survivors.is_empty() {
        return build_clean_outcome(&state).map_err(|error| map_outcome_error(&error));
    }
    let mut findings: Vec<Finding> = Vec::with_capacity(new_survivors.len());
    for survivor in &new_survivors {
        findings.push(survivor_finding(survivor)?);
    }
    Ok(LaneOutcome::Findings { findings: findings.into_boxed_slice() })
}

/// Fold the absence-of-binary / version-parse-failure paths into a
/// typed [`SkipReason::ToolUnavailable`] outcome so the lane never
/// propagates a `MutantsLaneError` for "no cargo-mutants on the host".
#[must_use]
pub(super) const fn tool_unavailable_outcome() -> LaneOutcome {
    LaneOutcome::Skipped { reason: SkipReason::ToolUnavailable(ToolKind::CargoMutants) }
}

/// Resolve the current unix-seconds timestamp without panicking on
/// clocks that pre-date the epoch.
#[must_use]
pub(super) fn current_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use titania_core::{LaneOutcome, SkipReason};

    use super::{current_unix, tool_unavailable_outcome};

    #[test]
    fn tool_unavailable_outcome_records_skip_reason() {
        let outcome = tool_unavailable_outcome();
        let LaneOutcome::Skipped { reason } = outcome else {
            panic!("tool-unavailable must surface as Skipped");
        };
        assert!(matches!(reason, SkipReason::ToolUnavailable(_)));
    }

    #[test]
    fn current_unix_returns_non_zero_on_real_clock() {
        // Sanity guard: `SystemTime::now()` must return post-epoch on
        // any normal host, so the saturating path is never the path
        // exercised in production.
        assert!(current_unix() > 1_700_000_000);
    }
}
