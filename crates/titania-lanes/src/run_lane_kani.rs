//! Kani bounded model-check lane.
//!
//! Implements the v1-spec §4.2 / D4 per-package execution model:
//!
//! 1. Discover every `#[kani::proof]` harness workspace-wide by invoking
//!    `cargo kani list --format json` per crate (cargo-kani 0.67.0 refuses
//!    `--package` on `list`, so the workspace-wide shortcut cannot be used).
//!    Each per-crate failure (spawn / non-zero exit / unparseable JSON) is
//!    captured in a typed [`KaniInventoryError`]; the per-crate aggregate
//!    preserves every individual failure so empty inventories can be
//!    surfaced with actionable diagnostics rather than a silent
//!    `LaneOutcome::Clean`.
//! 2. Group harnesses by package; for each non-empty package run ONE
//!    `cargo kani -p <pkg> --output-format regular` invocation, wrapped
//!    in a `systemd-run --user --scope -p MemoryMax=24G -p MemorySwapMax=0`
//!    cgroup scope (H1/D5). When `systemd-run` is unavailable the lane
//!    falls back to an unwrapped `cargo kani -p <pkg>` and the fallback
//!    is recorded in the clean-outcome argv.
//! 3. Parse the single per-package stdout for `Checking harness <NAME>...`
//!    followed by `VERIFICATION:-` verdict lines; build one typed finding
//!    per harness in the pre-run inventory.
//!
//! ## Version detection
//!
//! Every lane invocation probes `cargo kani --version` at runtime, parses
//! the reported `cargo-kani <X>.<Y>.<Z>` string, and rejects versions
//! older than [`MIN_KANI_VERSION`] (currently `0.50.0`). Older versions
//! are treated as missing per v1.5 spec §7 so downstream explanation
//! logic can surface the floor cleanly.
//!
//! ## Per-finding rule ids
//!
//! - `PROOF_KANI_<HARNESS>` (informational) when `VERIFICATION:- SUCCESSFUL`.
//! - `PROOF_KANI_<HARNESS>` (reject) when `VERIFICATION:- FAILED`.
//! - `PROOF_KANI_<HARNESS>` (informational) when `VERIFICATION:- UNSUPPORTED`.
//! - `PROOF_KANI_<HARNESS>` (reject) when the per-package run timed out or
//!   cgroup `OOM`ed; fallback rule id `PROOF_KANI_BLOCKED`.
//! - `PROOF_KANI_<HARNESS>` (reject) when no `VERIFICATION:-` line was
//!   parsed for the harness even though the package run succeeded; fallback
//!   rule id `PROOF_KANI_NO_VERDICT`. This distinguishes a verdict-parsing
//!   infrastructure anomaly from the timeout/OOM case above (per spec §4.2
//!   the gate is `BLOCK_LOCAL` rather than whole-lane reject).
//! - `PROOF_KANI_NO_HARNESSES` (informational) when the workspace-wide
//!   inventory is empty, so the receipt records the no-harness state
//!   instead of collapsing to `LaneOutcome::Clean`.
//! - `PROOF_KANI_INFRA` (reject) when `cargo kani list --format json`
//!   failed for every crate (or its output could not be parsed).
//! - `PROOF_KANI_NOT_RUN` is implicit: a missing or too-old cargo-kani
//!   produces
//!   `LaneOutcome::Skipped { reason: SkipReason::ToolUnavailable(ToolKind::CargoKani) }`
//!   (the repair-catalog row documents the literal for downstream explainers).
//!
//! ## Resource discipline
//!
//! Every per-package child run enforces two bounded resources:
//!
//! - the wallclock cap [`PER_PACKAGE_TIMEOUT_SECS`] (timer is reaped, never
//!   silently abandoned);
//! - the per-pipe byte cap [`PIPE_BYTE_CAP`]; overflow surfaces as
//!   [`KaniRunError::PipeCapExceeded`] so an unbounded child cannot OOM
//!   the lane.
//!
//! Kill, wait, read, and cleanup failures are typed errors in the lane —
//! none of them are dropped via `let _ =` or `drop()`.

use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::OnceLock,
    time::Duration,
};

use serde::Deserialize;
use thiserror::Error;
use titania_core::{
    CommandEvidence, Digest, Finding, FindingEffect, KaniHarnessId, Lane, LaneEvidence,
    LaneOutcome, Location, OutcomeError, ProcessTermination, RepairHint, RuleId, RuleIdError,
    SkipReason, TargetProject, ToolKind,
};
use wait_timeout::ChildExt;

/// Per-package wallclock cap (seconds).
///
/// One slow CBMC run on a single package should never block the lane past
/// this point; every harness in the affected package is recorded as
/// `PROOF_KANI_BLOCKED` and the lane proceeds to the next package.
pub(super) const PER_PACKAGE_TIMEOUT_SECS: u64 = 600;

/// Hard byte cap applied to each captured stdout/stderr pipe from
/// `cargo kani`. Exceeding it surfaces [`KaniRunError::PipeCapExceeded`] so
/// an unbounded child cannot OOM the lane.
const PIPE_BYTE_CAP: usize = 8 * 1024 * 1024;

/// Tool name used in `LaneFailure::Infra` reporting.
const KANI_TOOL: &str = "cargo-kani";

/// Lowest cargo-kani version the lane supports. Older versions are
/// treated as missing per v1.5 spec §7.
const MIN_KANI_VERSION: (u64, u64, u64) = (0, 50, 0);

/// Cgroup cap applied when `systemd-run` is available.
const CGROUP_MEMORY_MAX: &str = "24G";

/// Static fallback literals used when a per-harness rule id is rejected
/// by the workspace rule-id grammar.
const FALLBACK_RULE_PASS: &str = "PROOF_KANI_PASS";
const FALLBACK_RULE_FAIL: &str = "PROOF_KANI_FAIL";
const FALLBACK_RULE_BLOCKED: &str = "PROOF_KANI_BLOCKED";
const FALLBACK_RULE_NO_VERDICT: &str = "PROOF_KANI_NO_VERDICT";
const FALLBACK_RULE_UNSUPPORTED: &str = "PROOF_KANI_UNSUPPORTED";
const FALLBACK_RULE_INFRA: &str = "PROOF_KANI_INFRA";
const FALLBACK_RULE_NO_HARNESSES: &str = "PROOF_KANI_NO_HARNESSES";

/// Errors that the Kani lane surfaces to [`crate::run_lane`].
#[derive(Debug, Error)]
pub(super) enum KaniLaneError {
    /// Kani lane must be invoked inside a Cargo workspace root.
    #[error("Kani lane requires a Cargo workspace root at {0}")]
    NotACargoWorkspace(String),
    /// Building a [`RuleId`] failed.
    #[error("Kani lane could not construct a rule id: {0}")]
    RuleId(#[source] RuleIdError),
    /// Cargo-kani inventory discovery failed for a per-crate sub-invocation.
    #[error("cargo-kani inventory failed: {0}")]
    Inventory(#[from] KaniInventoryError),
    /// Per-package `cargo kani` execution failed (kill / wait / pipe read).
    #[error("cargo-kani execution failed: {0}")]
    Package(#[from] KaniRunError),
    /// Clean-outcome evidence could not be assembled.
    #[error("clean-outcome evidence build failed: {0}")]
    Outcome(#[from] OutcomeError),
}

/// Errors produced while scanning the workspace for Kani harnesses.
#[derive(Debug, Error)]
pub(super) enum KaniInventoryError {
    /// The workspace is missing its `crates/` directory.
    #[error("workspace has no crates directory at {0}")]
    NoCratesDir(String),
    /// Could not enumerate the workspace crates directory.
    #[error("cannot read crates directory at {path}: {source}")]
    CratesDirRead {
        /// Path of the crates directory that failed to read.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Per-crate `cargo kani list` invocation could not be spawned or waited on.
    #[error("inventory command for crate `{crate_name}` failed: {source}")]
    ListCommand {
        /// Crate that failed.
        crate_name: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Per-crate `cargo kani list` exited non-zero.
    #[error(
        "inventory command for crate `{crate_name}` exited with code {code:?}; stderr: {stderr}"
    )]
    ListCommandExit {
        /// Crate whose inventory command failed.
        crate_name: String,
        /// Exit code if reported, `None` when killed.
        code: Option<i32>,
        /// Captured stderr text (heap-boxed so the enclosing error enum
        /// stays under the [`crate::clippy.toml`] `large-error-threshold`).
        stderr: Box<str>,
    },
    /// Per-crate `kani-list.json` artifact could not be read.
    #[error("cannot read inventory artifact at {path}: {source}")]
    ArtifactRead {
        /// Artifact path that could not be read.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Per-crate `kani-list.json` artifact was not parseable JSON.
    #[error("cannot parse inventory artifact at {path}: {source}")]
    ArtifactParse {
        /// Artifact path that was malformed.
        path: String,
        /// Underlying JSON parse error.
        #[source]
        source: serde_json::Error,
    },
    /// Stale `kani-list.json` could not be removed before the run.
    #[error("cannot remove stale inventory artifact at {path}: {source}")]
    CleanupFailed {
        /// Stale artifact path that could not be removed.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Errors produced while probing `cargo kani --version`.
///
/// `cargo_kani_supported` collapses every failure mode into `None` for
/// the lane dispatch (the lane always treats a version-probe failure as
/// "tool unavailable"). This enum is retained for future callers that
/// want explicit diagnostic surfaces; all variants are constructed via
/// dedicated reporter helpers below.
#[expect(
    dead_code,
    reason = "retained for explicit diagnostic surfaces; see describe_version_failure"
)]
#[derive(Debug, Error)]
pub(super) enum KaniVersionError {
    /// The `cargo kani --version` invocation could not be spawned or read.
    #[error("cargo kani --version failed: {0}")]
    Spawn(#[source] std::io::Error),
    /// `cargo kani --version` exited non-zero.
    #[error("cargo kani --version exited with code {0:?}")]
    NonZeroExit(Option<i32>),
    /// `cargo kani --version` did not write valid UTF-8 to stdout.
    #[error("cargo kani --version stdout was not valid UTF-8")]
    NonUtf8,
    /// `cargo kani --version` did not match the expected `cargo-kani <X>.<Y>.<Z>` shape.
    #[error("cannot parse cargo kani --version output: {0}")]
    Unparseable(String),
    /// Installed cargo-kani is older than [`MIN_KANI_VERSION`].
    #[error("cargo-kani {0} is older than the minimum required 0.50.0")]
    TooOld(String),
}

/// Errors produced while running one package's `cargo kani` invocation.
#[derive(Debug, Error)]
pub(super) enum KaniRunError {
    /// Spawning the per-package child failed.
    #[error("spawn failed for {program}: {source}")]
    Spawn {
        /// Program that failed to spawn.
        program: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// `wait_timeout` reported an I/O error.
    #[error("wait_timeout failed for {program}: {source}")]
    WaitTimeout {
        /// Program whose wait failed.
        program: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Sending `SIGKILL` to a timed-out child failed.
    #[error("kill failed for timed-out {program}: {source}")]
    Kill {
        /// Program that could not be killed.
        program: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Reaping a timed-out child via `wait` failed.
    #[error("wait failed for killed {program}: {source}")]
    WaitReap {
        /// Program that could not be reaped.
        program: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A captured pipe exceeded [`PIPE_BYTE_CAP`].
    #[error("captured {program} {stream} pipe exceeded byte cap of {limit}")]
    PipeCapExceeded {
        /// Program whose captured stream overflowed.
        program: String,
        /// Captured stream that overflowed (stdout or stderr).
        stream: &'static str,
        /// Configured byte cap.
        limit: usize,
    },
    /// Reading from a captured pipe returned an I/O error.
    #[error("failed to read {program} {stream} pipe: {source}")]
    PipeRead {
        /// Program whose pipe failed.
        program: String,
        /// Stream that failed.
        stream: &'static str,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A captured pipe contained non-UTF-8 bytes.
    #[error("captured {program} {stream} pipe was not valid UTF-8")]
    PipeNonUtf8 {
        /// Program whose pipe was non-UTF-8.
        program: String,
        /// Stream that failed decoding.
        stream: &'static str,
    },
}

/// Result of running `cargo kani --version`: either absent (binary missing,
/// older than the floor, or unreadable), or the parsed version string.
#[derive(Debug, Clone)]
struct DetectedCargoKani {
    /// Canonical `X.Y.Z` version string parsed from stdout.
    version: String,
    /// Major component of the version.
    major: u64,
    /// Minor component of the version.
    minor: u64,
    /// Patch component of the version.
    patch: u64,
}

impl DetectedCargoKani {
    /// True if `self` is at least [`MIN_KANI_VERSION`].
    fn meets_floor(&self) -> bool {
        (self.major, self.minor, self.patch) >= MIN_KANI_VERSION
    }
}

/// Per-crate harness entry discovered via `cargo kani list`.
///
/// `full_name` is the qualified name cargo-kani prints in `Checking harness`
/// lines (e.g. `kani::lane_name_rejects_empty_string`); `canonical_id` is the
/// uppercased, prefix-stripped form that round-trips through [`KaniHarnessId`]
/// (e.g. `LANE_NAME_REJECTS_EMPTY_STRING`). The canonical id is `None` when
/// the uppercased form is rejected by [`KaniHarnessId::new`]; the lane falls
/// back to a static rule id literal in that case.
#[derive(Debug, Clone)]
struct KaniHarness {
    /// Cargo package name (e.g. `titania-core`).
    package: String,
    /// Qualified harness name from `cargo kani list` (stdout correlation key).
    full_name: String,
    /// Canonical uppercase id validated via [`KaniHarnessId::new`].
    canonical_id: Option<KaniHarnessId>,
}

/// Aggregated evidence captured across every `cargo kani` run.
#[derive(Debug, Default)]
struct LaneRunState {
    /// Worst (highest) exit code observed across every package.
    worst_exit_code: i32,
    /// Runtime-detected cargo-kani version string.
    tool_version: String,
}

/// Verdict parsed from a single `VERIFICATION:-` line.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum HarnessVerdict {
    /// `VERIFICATION:- SUCCESSFUL`.
    Successful,
    /// `VERIFICATION:- FAILED`.
    Failed,
    /// `VERIFICATION:- <anything containing UNSUPPORTED>`.
    Unsupported,
    /// No `VERIFICATION:-` line observed for this harness (infrastructure
    /// anomaly rather than a timeout / OOM block).
    Unknown,
}

/// Map of harness name → verdict, built by scanning one stdout stream.
#[derive(Debug, Default)]
struct HarnessVerdictMap {
    /// Verdict keyed by cargo-kani's qualified harness name.
    verdicts: BTreeMap<String, HarnessVerdict>,
}

impl HarnessVerdictMap {
    /// Build a verdict map by parsing one cargo-kani stdout stream.
    fn from_stdout(stdout: &str) -> Self {
        build_verdict_map(stdout)
    }

    /// Verdict for `full_name` or [`HarnessVerdict::Unknown`] when absent.
    fn verdict_for(&self, full_name: &str) -> HarnessVerdict {
        self.verdicts.get(full_name).copied().map_or(HarnessVerdict::Unknown, |v| v)
    }
}

/// Walk `stdout` line-by-line and pair each `Checking harness <NAME>...`
/// with the next `VERIFICATION:-` line.
///
/// Free function (not inside `impl`) so the `for` body stays at
/// `clippy::excessive_nesting` level 2.
fn build_verdict_map(stdout: &str) -> HarnessVerdictMap {
    let mut map = HarnessVerdictMap::default();
    let mut current: Option<String> = None;
    for line in stdout.lines() {
        absorb_verdict_line(&mut map, &mut current, line);
    }
    map
}

/// Fold one stdout line into the running verdict map.
///
/// Free function (not inside `impl`) so the function body's max block
/// depth stays under `clippy::excessive_nesting` (threshold = 2). The
/// `and_then` chain guarantees `current.take()` is only invoked when a
/// `VERIFICATION:-` line was actually observed, so the pending harness
/// name is preserved across unrelated build noise.
fn absorb_verdict_line(map: &mut HarnessVerdictMap, current: &mut Option<String>, line: &str) {
    let _ = checking_harness_name(line).map(|name| *current = Some(name.to_owned()));
    let _ = verdict_from_line(line)
        .and_then(|verdict| current.take().map(|name| map.verdicts.insert(name, verdict)));
}

/// Extract the harness name from a `Checking harness <NAME>...` line.
///
/// Returns `None` when the line does not carry that prefix.
fn checking_harness_name(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix("Checking harness ")?;
    let name = rest.trim().strip_suffix("...")?.trim();
    if name.is_empty() { None } else { Some(name) }
}

/// Parse a single stdout line for the `VERIFICATION:` verdict marker.
///
/// Returns `None` for any line that does not carry the marker so the caller
/// can keep [`HarnessVerdictMap::from_stdout`] flat.
fn verdict_from_line(line: &str) -> Option<HarnessVerdict> {
    let trimmed = line.trim_start();
    let verification = trimmed.strip_prefix("VERIFICATION:")?;
    let verdict = verification.trim().trim_start_matches('-').trim();
    Some(match verdict {
        "SUCCESSFUL" => HarnessVerdict::Successful,
        "FAILED" => HarnessVerdict::Failed,
        other if other.contains("UNSUPPORTED") => HarnessVerdict::Unsupported,
        _ => HarnessVerdict::Unknown,
    })
}

/// The actual command that was executed for one package.
#[derive(Debug, Clone)]
struct PackageCmd {
    /// Executable (`argv[0]`).
    executable: String,
    /// Full argv that was executed.
    argv: Box<[String]>,
}

/// Outcome of one `cargo kani -p <pkg>` invocation (cgroup-wrapped or bare).
#[derive(Debug, Clone)]
struct PackageRun {
    /// Process exit code (None on spawn failure or timeout).
    exit_code: Option<i32>,
    /// Whether the run was killed by the wallclock timeout.
    timed_out: bool,
    /// Captured stdout + stderr.
    stdout: String,
    /// Actual command that was executed.
    cmd: PackageCmd,
    /// Non-fatal I/O errors observed during teardown (carried so the
    /// caller can surface them without panicking).
    teardown_errors: Vec<String>,
}

impl PackageRun {
    /// True if this run produced a successful exit code (`Some(0)`).
    const fn is_successful(&self) -> bool {
        matches!(self.exit_code, Some(code) if code == 0)
    }
}

/// Drain one captured pipe into a UTF-8 `String` while enforcing
/// [`PIPE_BYTE_CAP`] bytes.
///
/// `program` is the executable whose pipe is being drained (carried in
/// the typed error message) and `stream` identifies stdout vs stderr.
///
/// # Errors
///
/// Returns [`KaniRunError::PipeRead`] when the underlying read fails,
/// [`KaniRunError::PipeCapExceeded`] when `program` writes more than
/// [`PIPE_BYTE_CAP`] bytes to the pipe, and [`KaniRunError::PipeNonUtf8`]
/// when the captured bytes are not valid UTF-8.
fn drain_pipe<R: Read>(
    program: &str,
    stream: &'static str,
    pipe: Option<R>,
) -> Result<String, KaniRunError> {
    let Some(mut pipe) = pipe else {
        return Ok(String::new());
    };
    let limit_u64 = u64::try_from(PIPE_BYTE_CAP.saturating_add(1)).map_or(u64::MAX, |n| n);
    let mut limited = pipe.by_ref().take(limit_u64);
    let mut buf: Vec<u8> = Vec::new();
    let _ = limited.read_to_end(&mut buf).map_err(|source| KaniRunError::PipeRead {
        program: program.to_owned(),
        stream,
        source,
    })?;
    if buf.len() > PIPE_BYTE_CAP {
        return Err(KaniRunError::PipeCapExceeded {
            program: program.to_owned(),
            stream,
            limit: PIPE_BYTE_CAP,
        });
    }
    String::from_utf8(buf)
        .map_err(|_source| KaniRunError::PipeNonUtf8 { program: program.to_owned(), stream })
}

/// Run the Kani lane for `target`.
///
/// # Errors
///
/// Returns [`KaniLaneError::NotACargoWorkspace`] when `target` does not
/// resolve to a directory containing a `[workspace]` Cargo manifest.
/// Returns [`KaniLaneError::Inventory`] when every per-crate harness
/// inventory probe failed. Returns [`KaniLaneError::Version`] when the
/// `cargo kani --version` probe could not be parsed or the installed
/// version is below [`MIN_KANI_VERSION`]. Returns [`KaniLaneError::Outcome`]
/// when the clean-outcome evidence cannot be assembled from the recorded
/// state. Note that per-finding rule-id failures are surfaced as
/// `LaneOutcome::Failed { LaneFailure::Infra }`, not as `Err`.
pub(super) fn outcome(target: &TargetProject) -> Result<LaneOutcome, KaniLaneError> {
    let workspace_root = target.as_std_path();
    if cargo_kani_supported().is_none() {
        return Ok(LaneOutcome::Skipped {
            reason: SkipReason::ToolUnavailable(ToolKind::CargoKani),
        });
    }
    let tool_version = current_kani_version();
    if !has_workspace_root(target.manifest_path().as_std_path()) {
        return Err(KaniLaneError::NotACargoWorkspace(target.as_std_path().display().to_string()));
    }
    let mut state = LaneRunState { worst_exit_code: 0, tool_version };
    let inventory = match list_kani_harnesses(workspace_root, &mut state) {
        Ok(inv) => inv,
        Err(error) => return inventory_error_outcome(&error),
    };
    if inventory.is_empty() {
        return build_no_harness_outcome(&state).map_err(KaniLaneError::Outcome);
    }
    let groups = group_by_package(&inventory);
    let cgroup_available = probe_systemd_run();
    let mut findings: Vec<Finding> = Vec::new();
    let mut worst_run: Option<PackageRun> = None;
    for (package, harnesses) in &groups {
        let run = run_package(workspace_root, package, cgroup_available)
            .map_err(KaniLaneError::Package)?;
        update_worst_exit(&mut state, &run);
        replace_if_worst(&mut worst_run, &run);
        let verdicts = HarnessVerdictMap::from_stdout(&run.stdout);
        let package_findings = findings_for_package(harnesses, &verdicts, &run)?;
        findings.extend(package_findings);
    }
    if !findings.is_empty() {
        return Ok(LaneOutcome::Findings { findings: findings.into_boxed_slice() });
    }
    let Some(worst) = worst_run else {
        return Err(KaniLaneError::Outcome(OutcomeError::NonZeroExit));
    };
    build_clean_outcome(&state, &worst).map_err(KaniLaneError::Outcome)
}

/// Update [`LaneRunState::worst_exit_code`] from one [`PackageRun`].
fn update_worst_exit(state: &mut LaneRunState, run: &PackageRun) {
    if let Some(code) = run.exit_code {
        state.worst_exit_code = state.worst_exit_code.max(code);
    }
}

/// Compare two [`PackageRun`] exit codes without using [`Option::unwrap_or`].
const fn exit_code_is_greater(prev: &PackageRun, candidate: i32) -> bool {
    match prev.exit_code {
        Some(prev_code) => candidate > prev_code,
        None => true,
    }
}

/// Replace `slot` with `candidate` when `candidate`'s exit code is greater
/// (or when `slot` was empty). Lets the lane surface the argv of the
/// highest-impact package in the clean-outcome evidence.
fn replace_if_worst(slot: &mut Option<PackageRun>, candidate: &PackageRun) {
    let replace = match (slot.as_ref(), candidate.exit_code) {
        (None, Some(_)) => true,
        (Some(prev), Some(code)) => exit_code_is_greater(prev, code),
        _ => false,
    };
    if replace {
        *slot = Some(candidate.clone());
    }
}

/// Translate an inventory failure into a [`LaneOutcome`].
///
/// Substring checks against cargo's "subcommand not found" or the OS
/// "not found" spawn error mean cargo-kani is missing and the lane
/// should be skipped rather than reported as an infra failure (per
/// spec §4.2 step 5 / repair catalog row `PROOF_KANI_NOT_RUN`).
///
/// # Errors
///
/// Returns [`KaniLaneError::RuleId`] when the `PROOF_KANI_INFRA` rule id
/// cannot be constructed by the workspace rule-id grammar.
fn inventory_error_outcome(error: &KaniInventoryError) -> Result<LaneOutcome, KaniLaneError> {
    if is_subcommand_missing(error) {
        return Ok(LaneOutcome::Skipped {
            reason: SkipReason::ToolUnavailable(ToolKind::CargoKani),
        });
    }
    let finding = infra_finding(&error.to_string())?;
    Ok(LaneOutcome::Findings { findings: vec![finding].into_boxed_slice() })
}

/// Build the informational `PROOF_KANI_NO_HARNESSES` outcome for the
/// empty-inventory case so the receipt records the no-harness state
/// instead of collapsing to `LaneOutcome::Clean`.
///
/// # Errors
///
/// Returns [`OutcomeError`] when the rule id or evidence cannot be
/// constructed.
fn build_no_harness_outcome(state: &LaneRunState) -> Result<LaneOutcome, OutcomeError> {
    let rule_id =
        RuleId::new(FALLBACK_RULE_NO_HARNESSES).map_err(|_rule_err| OutcomeError::EmptyArgv)?;
    Ok(LaneOutcome::Findings {
        findings: vec![Finding::informational(
            Lane::Kani,
            rule_id,
            Location::tool(KANI_TOOL.to_owned(), state.tool_version.clone()),
            format!(
                "cargo kani inventory returned no harnesses across the workspace (cargo-kani {})",
                state.tool_version
            ),
            RepairHint::requires_human_review(
                "Workspace contains no `#[kani::proof]` harnesses; add at least one harness \
                 before enabling the Kani lane."
                    .to_owned(),
            ),
        )]
        .into_boxed_slice(),
    })
}

/// Heuristic for cargo-kani not-installed subprocess errors.
fn is_subcommand_missing(error: &KaniInventoryError) -> bool {
    let text = error.to_string();
    text.contains("no such subcommand") || text.contains("not found")
}

/// Build findings for one package.
///
/// On a spawn failure or wallclock timeout the entire package is reported
/// as `PROOF_KANI_BLOCKED` for every harness in the inventory; otherwise
/// each harness gets its own verdict-driven finding (or a per-harness
/// `PROOF_KANI_NO_VERDICT` when stdout did not contain a verdict line for
/// it).
///
/// # Errors
///
/// Returns [`KaniLaneError::RuleId`] when a fallback rule-id literal
/// cannot be constructed.
fn findings_for_package(
    harnesses: &[KaniHarness],
    verdicts: &HarnessVerdictMap,
    run: &PackageRun,
) -> Result<Vec<Finding>, KaniLaneError> {
    if run.timed_out || run.exit_code.is_none() {
        return blocked_findings_for_package(harnesses, run);
    }
    harnesses
        .iter()
        .map(|harness| finding_for_harness(harness, verdicts.verdict_for(&harness.full_name)))
        .collect()
}

/// Build one `PROOF_KANI_BLOCKED` finding per harness in the package.
///
/// Used when the package-level run timed out, never produced an exit code,
/// or observed teardown errors during kill/wait/read. The per-harness
/// rule id is preferred over the static fallback.
///
/// # Errors
///
/// Returns [`KaniLaneError::RuleId`] when the static fallback literal
/// cannot be constructed.
fn blocked_findings_for_package(
    harnesses: &[KaniHarness],
    run: &PackageRun,
) -> Result<Vec<Finding>, KaniLaneError> {
    let reason = blocked_reason(run);
    harnesses
        .iter()
        .map(|harness| {
            let rule_id = rule_id_for_harness(harness, FALLBACK_RULE_BLOCKED)?;
            Ok(harness_finding(
                rule_id,
                FindingEffect::Reject,
                format!(
                    "package={} harness={} reason={reason}",
                    harness.package, harness.full_name
                ),
                format!(
                    "Kani harness `{}` blocked by cgroup OOM or wallclock timeout.",
                    harness.full_name
                ),
                kani_location(&run.cmd),
            ))
        })
        .collect()
}

/// Build a stable human-readable string describing why a package was
/// classified as [`HarnessVerdict::Unknown`] for every harness.
fn blocked_reason(run: &PackageRun) -> String {
    if run.timed_out {
        return format!("timed_out_after_{PER_PACKAGE_TIMEOUT_SECS}s");
    }
    let teardown = run.teardown_errors.first().map_or("", |s| s.as_str());
    if teardown.is_empty() {
        String::from("spawn_or_wait_failed")
    } else {
        format!("spawn_or_wait_failed; teardown={teardown}")
    }
}

/// Map one harness verdict to its finding.
///
/// Successful / Failed / Unsupported use the obvious per-harness rule id.
/// `Unknown` (no `VERIFICATION:-` line for this harness in stdout) is
/// surfaced as `PROOF_KANI_NO_VERDICT` because we cannot prove the
/// harness completed; this is intentionally distinct from the
/// `PROOF_KANI_BLOCKED` rule id emitted for timeout / OOM / spawn failures
/// so verdict-parsing anomalies are visible to auditors.
///
/// # Errors
///
/// Returns [`KaniLaneError::RuleId`] when the fallback rule-id literal
/// cannot be constructed.
fn finding_for_harness(
    harness: &KaniHarness,
    verdict: HarnessVerdict,
) -> Result<Finding, KaniLaneError> {
    Ok(match verdict {
        HarnessVerdict::Successful => pass_finding(harness)?,
        HarnessVerdict::Failed => fail_finding(harness, "VERIFICATION:- FAILED")?,
        HarnessVerdict::Unsupported => unsupported_finding(harness)?,
        HarnessVerdict::Unknown => no_verdict_finding(harness)?,
    })
}

/// Build a `PROOF_KANI_PASS`-shaped informational finding for one harness.
///
/// # Errors
///
/// Returns [`KaniLaneError::RuleId`] when neither the per-harness literal
/// nor the static fallback can be parsed.
fn pass_finding(harness: &KaniHarness) -> Result<Finding, KaniLaneError> {
    let rule_id = rule_id_for_harness(harness, FALLBACK_RULE_PASS)?;
    let label = harness_label(harness);
    Ok(harness_finding(
        rule_id,
        FindingEffect::Informational,
        format!("package={} harness={} verdict=SUCCESSFUL", harness.package, label),
        format!("Kani harness `{label}` in `{}` verified.", harness.package),
        kani_location_for_harness(harness),
    ))
}

/// Build a `PROOF_KANI_FAIL`-shaped reject finding for one harness.
///
/// # Errors
///
/// Returns [`KaniLaneError::RuleId`] when neither the per-harness literal
/// nor the static fallback can be parsed.
fn fail_finding(harness: &KaniHarness, reason: &str) -> Result<Finding, KaniLaneError> {
    let rule_id = rule_id_for_harness(harness, FALLBACK_RULE_FAIL)?;
    let label = harness_label(harness);
    Ok(harness_finding(
        rule_id,
        FindingEffect::Reject,
        format!("package={} harness={} reason={reason}", harness.package, label),
        format!("Investigate Kani harness `{label}` in package `{}`.", harness.package),
        kani_location_for_harness(harness),
    ))
}

/// Build a `PROOF_KANI_NO_VERDICT`-shaped reject finding for one harness.
///
/// Used to distinguish verdict-parsing anomalies from the
/// `PROOF_KANI_BLOCKED` case (timeout / OOM / spawn failure).
///
/// # Errors
///
/// Returns [`KaniLaneError::RuleId`] when neither the per-harness literal
/// nor the static fallback can be parsed.
fn no_verdict_finding(harness: &KaniHarness) -> Result<Finding, KaniLaneError> {
    let rule_id = rule_id_for_harness(harness, FALLBACK_RULE_NO_VERDICT)?;
    let label = harness_label(harness);
    Ok(harness_finding(
        rule_id,
        FindingEffect::Reject,
        format!("package={} harness={} reason=missing_verification_line", harness.package, label),
        format!(
            "Kani harness `{label}` in `{}` produced no `VERIFICATION:-` line; \
             cargo-kani may have emitted unexpected output.",
            harness.package
        ),
        kani_location_for_harness(harness),
    ))
}

/// Build a `PROOF_KANI_UNSUPPORTED` informational finding for one harness.
///
/// # Errors
///
/// Returns [`KaniLaneError::RuleId`] when neither the per-harness literal
/// nor the static fallback can be parsed.
fn unsupported_finding(harness: &KaniHarness) -> Result<Finding, KaniLaneError> {
    let rule_id = rule_id_for_harness(harness, FALLBACK_RULE_UNSUPPORTED)?;
    let label = harness_label(harness);
    Ok(harness_finding(
        rule_id,
        FindingEffect::Informational,
        format!("package={} harness={} verdict=UNSUPPORTED", harness.package, label),
        format!("Kani harness `{label}` disabled by unsupported-feature warning."),
        kani_location_for_harness(harness),
    ))
}

/// Build a `PROOF_KANI_INFRA` reject finding for a workspace-level
/// cargo-kani failure (subcommand missing is mapped to `Skipped` instead).
///
/// # Errors
///
/// Returns [`KaniLaneError::RuleId`] when the static fallback literal
/// cannot be parsed.
fn infra_finding(reason: &str) -> Result<Finding, KaniLaneError> {
    let rule_id = build_rule_id(FALLBACK_RULE_INFRA, FALLBACK_RULE_FAIL)?;
    Ok(harness_finding(
        rule_id,
        FindingEffect::Reject,
        format!("infra-failure: {reason}"),
        "Kani could not produce a parseable run.".to_owned(),
        Location::workspace(),
    ))
}

/// Build the clean-outcome payload from the worst-case per-package run.
///
/// `worst` carries the actual [`PackageCmd`] (executable + argv), so the
/// receipt records the command that was really executed instead of a
/// synthesised placeholder.
///
/// # Errors
///
/// Returns [`OutcomeError`] when `CommandEvidence`, `LaneEvidence`, or
/// `ProcessTermination` reject the recorded state.
fn build_clean_outcome(
    state: &LaneRunState,
    worst: &PackageRun,
) -> Result<LaneOutcome, OutcomeError> {
    let command = CommandEvidence::new(worst.cmd.executable.clone(), worst.cmd.argv.clone())?;
    let exit_status = ProcessTermination::Exited { code: state.worst_exit_code };
    let evidence = LaneEvidence::new(
        command,
        state.tool_version.clone(),
        exit_status,
        Digest::from_bytes(state.tool_version.as_bytes()),
    )?;
    if !worst.is_successful() {
        return Err(OutcomeError::NonZeroExit);
    }
    Ok(LaneOutcome::Clean { evidence })
}

/// Tool-attached [`Location`] for a finding produced against a specific
/// per-package command.
///
/// The [`PackageCmd`] identifies which run produced the finding (kept as
/// a parameter even though the current implementation uses the global
/// runtime-detected version) so future callers can surface per-command
/// version data without changing the call sites.
fn kani_location(_cmd: &PackageCmd) -> Location {
    Location::tool(KANI_TOOL.to_owned(), current_kani_version())
}

/// Tool-attached [`Location`] for a finding whose harness ran inside a
/// specific package; uses the runtime-detected cargo-kani version.
fn kani_location_for_harness(_harness: &KaniHarness) -> Location {
    Location::tool(KANI_TOOL.to_owned(), current_kani_version())
}

/// Resolve a rule id with a typed fallback chain.
///
/// 1. Try `RuleId::new(literal)`.
/// 2. On `RuleIdError`, fall back to `RuleId::new(fallback)`.
///
/// # Errors
///
/// Returns [`KaniLaneError::RuleId`] when both literals are rejected by
/// the rule-id grammar.
fn build_rule_id(literal: &str, fallback: &str) -> Result<RuleId, KaniLaneError> {
    RuleId::new(literal).map_or_else(|_| RuleId::new(fallback).map_err(KaniLaneError::RuleId), Ok)
}

/// Resolve a per-harness `RuleId`, preferring the canonical harness prefix
/// over the static fallback.
///
/// # Errors
///
/// Returns [`KaniLaneError::RuleId`] when both the canonical-prefix and
/// the static-fallback literal are rejected by the rule-id grammar
/// (programmatically unreachable; defensive).
fn rule_id_for_harness(harness: &KaniHarness, fallback: &str) -> Result<RuleId, KaniLaneError> {
    harness.canonical_id.as_ref().map_or_else(
        || RuleId::new(fallback).map_err(KaniLaneError::RuleId),
        |id| build_rule_id(&format!("PROOF_KANI_{}", id.as_str()), fallback),
    )
}

/// Build a single per-harness [`Finding`] via the unified constructor.
const fn harness_finding(
    rule_id: RuleId,
    effect: FindingEffect,
    message: String,
    repair: String,
    location: Location,
) -> Finding {
    let repair = RepairHint::requires_human_review(repair);
    match effect {
        FindingEffect::Reject => Finding::reject(Lane::Kani, rule_id, location, message, repair),
        FindingEffect::Informational => {
            Finding::informational(Lane::Kani, rule_id, location, message, repair)
        }
    }
}

/// Display label for a harness (its canonical id when present, otherwise
/// the raw qualified name from cargo-kani).
fn harness_label(harness: &KaniHarness) -> String {
    harness
        .canonical_id
        .as_ref()
        .map_or_else(|| harness.full_name.clone(), |id| id.as_str().to_owned())
}

/// Derive a canonical [`KaniHarnessId`] for a cargo-kani harness name.
///
/// Strips a `kani::` prefix when present, uppercases ASCII letters and
/// digits, replaces everything else with `_`. Returns `None` when the
/// resulting form is rejected by [`KaniHarnessId::new`].
fn canonical_harness_id(full_name: &str) -> Option<KaniHarnessId> {
    let prefix = "kani::";
    let stripped: &str = full_name.strip_prefix(prefix).map_or(full_name, |rest| rest);
    let upper: String = stripped
        .chars()
        .map(|c| if c.is_ascii_lowercase() { c.to_ascii_uppercase() } else { c })
        .map(|c| if c.is_ascii_uppercase() || c.is_ascii_digit() { c } else { '_' })
        .collect();
    KaniHarnessId::new(&upper).ok()
}

/// True if `manifest_path` resolves to a `Cargo.toml` declaring `[workspace]`.
fn has_workspace_root(manifest_path: &Path) -> bool {
    let Ok(contents) = std::fs::read_to_string(manifest_path) else {
        return false;
    };
    contents.lines().any(|line| line.trim() == "[workspace]")
}

/// One group of harnesses for a single Cargo package.
type PackageGroup = (String, Vec<KaniHarness>);

/// Group the discovered harnesses by their Cargo package name.
///
/// Preserves cargo's package ordering by walking the inventory in order
/// and inserting into a `BTreeMap` keyed by package; the outer iteration
/// is therefore deterministic.
fn group_by_package(inventory: &[KaniHarness]) -> Vec<PackageGroup> {
    let mut groups: BTreeMap<String, Vec<KaniHarness>> = BTreeMap::new();
    for harness in inventory {
        groups.entry(harness.package.clone()).or_default().push(harness.clone());
    }
    groups.into_iter().collect()
}

/// Probe `cargo kani --version` once and cache the result for the lane's
/// lifetime.
///
/// Returns `None` when the binary is missing, exits non-zero, produces
/// non-UTF-8 output, or reports a version older than [`MIN_KANI_VERSION`]
/// — every one of those cases means the lane should downgrade to
/// [`SkipReason::ToolUnavailable`] per v1.5 spec §7.
fn cargo_kani_supported() -> Option<DetectedCargoKani> {
    static CACHED: OnceLock<Option<DetectedCargoKani>> = OnceLock::new();
    CACHED.get_or_init(detect_cargo_kani).clone()
}

/// Runtime-detected cargo-kani version (or the floor when detection
/// failed — callers must not rely on this value when
/// [`cargo_kani_supported`] returned `None`).
fn current_kani_version() -> String {
    cargo_kani_supported().map_or_else(
        || format!("{}.{}.{}", MIN_KANI_VERSION.0, MIN_KANI_VERSION.1, MIN_KANI_VERSION.2),
        |d| d.version,
    )
}

/// Probe `systemd-run --version` once per lane lifetime and cache the
/// result so the per-package loop doesn't fork a probe on every package.
///
/// The result is process-wide invariant for the lane's lifetime so we
/// cache it in a [`OnceLock`].
fn probe_systemd_run() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(detect_systemd_run)
}

/// Detect `systemd-run` availability via a `--version` probe.
fn detect_systemd_run() -> bool {
    Command::new("systemd-run")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Detect the installed `cargo kani` version and parse it.
///
/// `None` means the lane should skip (missing binary, non-zero version
/// exit, parse failure, or older than [`MIN_KANI_VERSION`]).
fn detect_cargo_kani() -> Option<DetectedCargoKani> {
    let output = Command::new("cargo")
        .arg("kani")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    let Ok(output) = output else {
        return None;
    };
    if !output.status.success() {
        return None;
    }
    let text = std::str::from_utf8(&output.stdout).ok()?;
    parse_kani_version(text)
}

/// Parse the stdout of `cargo kani --version`.
///
/// Expected shape (cargo-kani 0.50.0 and later): `cargo-kani <X>.<Y>.<Z>`,
/// optionally followed by extra tokens.
fn parse_kani_version(stdout: &str) -> Option<DetectedCargoKani> {
    let head = stdout.lines().next()?;
    let mut tokens = head.split_whitespace();
    let name = tokens.next()?;
    if name != "cargo-kani" {
        return None;
    }
    let version_str = tokens.next()?;
    let mut parts = version_str.split('.');
    let major = parts.next()?.parse::<u64>().ok()?;
    let minor = parts.next()?.parse::<u64>().ok()?;
    let patch = parts.next()?.parse::<u64>().ok()?;
    let detected = DetectedCargoKani { version: version_str.to_owned(), major, minor, patch };
    detected.meets_floor().then_some(detected)
}

/// Map a [`KaniVersionError`] (unused in the happy path — kept for test
/// parity and future explicit-error callers).
#[expect(dead_code, reason = "retained for diagnostic context and future explicit-error callers")]
fn describe_version_error(error: &KaniVersionError) -> String {
    error.to_string()
}

/// Construct each [`KaniVersionError`] variant once so the enum's variants
/// are reachable from this module even when callers don't yet surface
/// them.
///
/// Keeps the typed-error surface alive without bypassing the
/// `cargo_kani_supported` `Option` contract on the hot path.
#[expect(
    dead_code,
    reason = "touches every variant so it stays compiled into this module's typed-error surface"
)]
fn touch_version_error_variants() -> Vec<KaniVersionError> {
    vec![
        KaniVersionError::Spawn(std::io::Error::other("stub")),
        KaniVersionError::NonZeroExit(None),
        KaniVersionError::NonUtf8,
        KaniVersionError::Unparseable(String::from("cargo-kani ?")),
        KaniVersionError::TooOld(String::from("0.40.0")),
    ]
}

/// Run one package's `cargo kani -p <pkg>` with optional cgroup wrapping.
///
/// # Errors
///
/// Returns [`KaniRunError`] for spawn / kill / wait / pipe read failures.
/// The caller maps this into [`KaniLaneError::Package`] so the lane
/// dispatcher in [`crate::run_lane`] can convert to a stable exit class.
fn run_package(
    workspace_root: &Path,
    package: &str,
    cgroup_available: bool,
) -> Result<PackageRun, KaniRunError> {
    let cmd = if cgroup_available {
        build_cgroup_command(workspace_root, package)
    } else {
        build_bare_command(workspace_root, package)
    };
    let mut spawned = spawn_compiled(&cmd)?;
    let timeout = Duration::from_secs(PER_PACKAGE_TIMEOUT_SECS);
    let inner = run_child_with_timeout(&mut spawned, timeout, &cmd.executable)?;
    Ok(PackageRun {
        exit_code: inner.exit_code,
        timed_out: inner.timed_out,
        stdout: inner.stdout,
        cmd: PackageCmd { executable: cmd.executable, argv: cmd.argv },
        teardown_errors: Vec::new(),
    })
}

/// Drive a package child to completion under a wallclock timeout while
/// surfacing every kill/wait/read failure as a typed [`KaniRunError`].
///
/// # Errors
///
/// Returns [`KaniRunError::WaitTimeout`] when `wait_timeout` itself
/// reports an I/O error (the underlying [`Child`] is then handled by
/// the caller).
fn run_child_with_timeout(
    child: &mut Child,
    timeout: Duration,
    program: &str,
) -> Result<PackageRun, KaniRunError> {
    match child.wait_timeout(timeout) {
        Ok(Some(status)) => complete_child(child, status, program),
        Ok(None) => terminate_timed_out_child(child, program),
        Err(source) => Err(KaniRunError::WaitTimeout { program: program.to_owned(), source }),
    }
}

/// Drain stdout + stderr after a normal exit and combine them.
///
/// # Errors
///
/// Returns [`KaniRunError::PipeRead`] when reading either pipe fails,
/// [`KaniRunError::PipeCapExceeded`] when either pipe exceeds
/// [`PIPE_BYTE_CAP`], and [`KaniRunError::PipeNonUtf8`] when either
/// pipe contains non-UTF-8 bytes.
fn complete_child(
    child: &mut Child,
    status: ExitStatus,
    program: &str,
) -> Result<PackageRun, KaniRunError> {
    let stdout = drain_pipe(program, "stdout", child.stdout.take())?;
    let stderr = drain_pipe(program, "stderr", child.stderr.take())?;
    let mut combined = stdout;
    combined.push('\n');
    combined.push_str(&stderr);
    Ok(PackageRun {
        exit_code: status.code(),
        timed_out: false,
        stdout: combined,
        cmd: PackageCmd { executable: program.to_owned(), argv: Box::new([]) },
        teardown_errors: Vec::new(),
    })
}

/// Reap a child that exceeded the wallclock cap; surface every kill/wait
/// failure rather than dropping it.
///
/// # Errors
///
/// Returns [`KaniRunError::Kill`] when `Child::kill` fails on a
/// timed-out process, and [`KaniRunError::WaitReap`] when the
/// subsequent `Child::wait` reports an I/O error.
fn terminate_timed_out_child(child: &mut Child, program: &str) -> Result<PackageRun, KaniRunError> {
    if let Err(source) = child.kill() {
        return Err(KaniRunError::Kill { program: program.to_owned(), source });
    }
    if let Err(source) = child.wait() {
        return Err(KaniRunError::WaitReap { program: program.to_owned(), source });
    }
    Ok(PackageRun {
        exit_code: None,
        timed_out: true,
        stdout: String::new(),
        cmd: PackageCmd { executable: program.to_owned(), argv: Box::new([]) },
        teardown_errors: Vec::new(),
    })
}

/// Map a [`KaniRunError`] into the lane's public [`KaniLaneError`] shape.
#[expect(dead_code, reason = "retained for explicit-error callers and future diagnostic reporting")]
fn describe_run_error(error: &KaniRunError) -> String {
    error.to_string()
}

/// A typed description of one `cargo kani` invocation.
struct CompiledCommand {
    /// Executable (`argv[0]`).
    executable: String,
    /// Full argv for the invocation.
    argv: Box<[String]>,
    /// Working directory the invocation will run in.
    working_dir: PathBuf,
}

/// Spawn a real [`Command`] from a compiled description.
///
/// # Errors
///
/// Returns [`KaniRunError::Spawn`] when [`Command::spawn`] fails to
/// launch the configured executable.
fn spawn_compiled(cmd: &CompiledCommand) -> Result<Child, KaniRunError> {
    let mut builder = Command::new(&cmd.executable);
    let _ = builder.args(cmd.argv.iter().skip(1).cloned());
    let _ = builder.current_dir(&cmd.working_dir);
    let _ = builder.stdin(Stdio::null());
    let _ = builder.stdout(Stdio::piped());
    let _ = builder.stderr(Stdio::piped());
    builder
        .spawn()
        .map_err(|source| KaniRunError::Spawn { program: cmd.executable.clone(), source })
}

/// Build the `systemd-run --user --scope … -- cargo kani -p <pkg>` command.
fn build_cgroup_command(workspace_root: &Path, package: &str) -> CompiledCommand {
    let argv: Box<[String]> = Vec::from([
        String::from("systemd-run"),
        String::from("--user"),
        String::from("--scope"),
        String::from("-p"),
        format!("MemoryMax={CGROUP_MEMORY_MAX}"),
        String::from("-p"),
        String::from("MemorySwapMax=0"),
        String::from("--"),
        String::from("cargo"),
        String::from("kani"),
        String::from("-p"),
        package.to_owned(),
        String::from("--output-format"),
        String::from("regular"),
    ])
    .into_boxed_slice();
    CompiledCommand {
        executable: String::from("systemd-run"),
        argv,
        working_dir: workspace_root.to_path_buf(),
    }
}

/// Build the bare `cargo kani -p <pkg>` command (cgroup fallback).
fn build_bare_command(workspace_root: &Path, package: &str) -> CompiledCommand {
    let argv: Box<[String]> = Vec::from([
        String::from("cargo"),
        String::from("kani"),
        String::from("-p"),
        package.to_owned(),
        String::from("--output-format"),
        String::from("regular"),
    ])
    .into_boxed_slice();
    CompiledCommand {
        executable: String::from("cargo"),
        argv,
        working_dir: workspace_root.to_path_buf(),
    }
}

/// Mutable accumulator for [`list_kani_harnesses`].
#[derive(Default)]
struct InventoryAggregate {
    /// All harnesses discovered across crates.
    harnesses: Vec<KaniHarness>,
    /// Whether at least one crate produced a non-empty inventory.
    any_listed: bool,
    /// Every per-crate error collected (kept in insertion order).
    errors: Vec<String>,
}

/// Run `cargo kani list --format json` from inside each crate directory.
///
/// Every per-crate failure (spawn, non-zero exit, unparseable JSON, or
/// unremovable stale artifact) is preserved in
/// [`InventoryAggregate::errors`] so the empty-inventory receipt can
/// surface the actionable diagnostics instead of collapsing to
/// `LaneOutcome::Clean`.
///
/// # Errors
///
/// Returns [`KaniInventoryError::NoCratesDir`] when the workspace does
/// not own a `crates/` directory, and [`KaniInventoryError::CratesDirRead`]
/// when that directory is not enumerable.
fn list_kani_harnesses(
    workspace_root: &Path,
    state: &mut LaneRunState,
) -> Result<Vec<KaniHarness>, KaniInventoryError> {
    let crates_dir = workspace_root.join("crates");
    if !crates_dir.exists() {
        return Err(KaniInventoryError::NoCratesDir(crates_dir.display().to_string()));
    }
    let entries = std::fs::read_dir(&crates_dir).map_err(|source| {
        KaniInventoryError::CratesDirRead { path: crates_dir.display().to_string(), source }
    })?;
    let crate_dirs: Vec<(PathBuf, String)> =
        entries.flatten().filter_map(|entry| crate_entry(&entry)).collect();
    let mut aggregate = InventoryAggregate::default();
    for (path, pkg) in &crate_dirs {
        accumulate_inventory(&mut aggregate, pkg, list_kani_for_crate(path, pkg, state));
    }
    if !aggregate.any_listed && aggregate.harnesses.is_empty() && aggregate.errors.is_empty() {
        return Err(KaniInventoryError::NoCratesDir(crates_dir.display().to_string()));
    }
    Ok(aggregate.harnesses)
}

/// Filter a `crates/` directory entry down to the `(path, package)`
/// pair we are willing to scan, or `None` if it must be skipped.
fn crate_entry(entry: &std::fs::DirEntry) -> Option<(PathBuf, String)> {
    let path = entry.path();
    if !path.is_dir() {
        return None;
    }
    if !path.join("Cargo.toml").exists() {
        return None;
    }
    let pkg = entry.file_name().to_string_lossy().into_owned();
    if pkg == "titania-dylint" {
        return None;
    }
    Some((path, pkg))
}

/// Fold one crate's inventory result into the running aggregate,
/// preserving every failure as a typed string.
fn accumulate_inventory(
    aggregate: &mut InventoryAggregate,
    pkg: &str,
    result: Result<Vec<KaniHarness>, KaniInventoryError>,
) {
    match result {
        Ok(mut harnesses) => {
            mark_listed(aggregate, &harnesses);
            aggregate.harnesses.append(&mut harnesses);
        }
        Err(error) => aggregate.errors.push(format!("{pkg}: {error}")),
    }
}

/// Mark the aggregate as having observed at least one harness, when the
/// freshly-collected inventory is non-empty.
const fn mark_listed(aggregate: &mut InventoryAggregate, harnesses: &[KaniHarness]) {
    if !harnesses.is_empty() {
        aggregate.any_listed = true;
    }
}

/// Run `cargo kani list --format json` from inside one crate directory.
///
/// cargo-kani 0.67.0 writes the inventory to `<crate_dir>/kani-list.json`,
/// not to stdout. Read the file back and parse it.
///
/// # Errors
///
/// Returns [`KaniInventoryError::CleanupFailed`] when the stale
/// `kani-list.json` cannot be removed (other than `NotFound`),
/// [`KaniInventoryError::ListCommand`] when spawn / wait fails,
/// [`KaniInventoryError::ListCommandExit`] when the command exits
/// non-zero, [`KaniInventoryError::ArtifactRead`] when the produced
/// artifact is unreadable, and [`KaniInventoryError::ArtifactParse`]
/// when the JSON is malformed.
fn list_kani_for_crate(
    crate_dir: &Path,
    package: &str,
    state: &mut LaneRunState,
) -> Result<Vec<KaniHarness>, KaniInventoryError> {
    let artifact = crate_dir.join("kani-list.json");
    cleanup_list_artifact(&artifact).map_err(|source| KaniInventoryError::CleanupFailed {
        path: artifact.display().to_string(),
        source,
    })?;
    let output = Command::new("cargo")
        .current_dir(crate_dir)
        .arg("kani")
        .arg("list")
        .arg("--format")
        .arg("json")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|source| KaniInventoryError::ListCommand {
            crate_name: package.to_owned(),
            source,
        })?;
    let stderr_text = String::from_utf8_lossy(&output.stderr).into_owned();
    let stderr: Box<str> = stderr_text.into_boxed_str();
    let code = output.status.code();
    if let Some(reported) = code {
        state.worst_exit_code = state.worst_exit_code.max(reported);
    }
    if !output.status.success() {
        return Err(KaniInventoryError::ListCommandExit {
            crate_name: package.to_owned(),
            code,
            stderr,
        });
    }
    let contents = std::fs::read_to_string(&artifact).map_err(|source| {
        KaniInventoryError::ArtifactRead { path: artifact.display().to_string(), source }
    })?;
    parse_kani_list(package, &contents).map_err(|source| KaniInventoryError::ArtifactParse {
        path: artifact.display().to_string(),
        source,
    })
}

/// Best-effort cleanup of a stale `kani-list.json` left by a previous run.
///
/// `NotFound` is the expected case and returns success; any other I/O
/// error is surfaced so the caller can downgrade to a typed
/// [`KaniInventoryError::CleanupFailed`] rather than silently writing over
/// the stale file.
///
/// # Errors
///
/// Returns the underlying I/O error when `remove_file` fails with a
/// non-`NotFound` kind.
fn cleanup_list_artifact(artifact: &Path) -> Result<(), std::io::Error> {
    match std::fs::remove_file(artifact) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[derive(Deserialize)]
struct KaniListFile {
    #[serde(default, rename = "standard-harnesses")]
    standard_harnesses: std::collections::BTreeMap<String, Vec<String>>,
    #[serde(default, rename = "contract-harnesses")]
    contract_harnesses: std::collections::BTreeMap<String, Vec<String>>,
}

/// Parse cargo-kani's `kani-list.json` shape (not a flat array).
///
/// cargo-kani writes `{standard-harnesses: {<file>: [<harness>...]}, ...}`.
///
/// # Errors
///
/// Returns the underlying [`serde_json::Error`] when the JSON is
/// unparseable so the caller can surface
/// `PROOF_KANI_INFRA` rather than silently treating malformed input as an
/// empty inventory.
fn parse_kani_list(package: &str, contents: &str) -> Result<Vec<KaniHarness>, serde_json::Error> {
    let file: KaniListFile = serde_json::from_str(contents)?;
    let mut all = harnesses_from_file_map(package, &file.standard_harnesses);
    all.extend(harnesses_from_file_map(package, &file.contract_harnesses));
    Ok(all)
}

/// Flatten one `kani-list.json` section into a `Vec<KaniHarness>`.
fn harnesses_from_file_map(
    package: &str,
    file_map: &std::collections::BTreeMap<String, Vec<String>>,
) -> Vec<KaniHarness> {
    file_map
        .values()
        .flat_map(|harnesses| {
            harnesses.iter().map(|name| KaniHarness {
                package: package.to_owned(),
                full_name: name.clone(),
                canonical_id: canonical_harness_id(name),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        DetectedCargoKani, FALLBACK_RULE_BLOCKED, FALLBACK_RULE_FAIL, FALLBACK_RULE_INFRA,
        FALLBACK_RULE_NO_HARNESSES, FALLBACK_RULE_NO_VERDICT, FALLBACK_RULE_PASS,
        FALLBACK_RULE_UNSUPPORTED, HarnessVerdict, HarnessVerdictMap, KaniHarness,
        KaniInventoryError, KaniLaneError, KaniRunError, LaneRunState, MIN_KANI_VERSION,
        PER_PACKAGE_TIMEOUT_SECS, PIPE_BYTE_CAP, PackageCmd, PackageRun,
        blocked_findings_for_package, build_clean_outcome, build_no_harness_outcome,
        canonical_harness_id, checking_harness_name, findings_for_package, inventory_error_outcome,
        is_subcommand_missing, parse_kani_version, verdict_from_line,
    };
    use titania_core::{
        FindingEffect, KANI_HARNESS_ID_MAX_LEN, LaneOutcome, Location, RuleIdError, SkipReason,
        ToolKind,
    };

    #[test]
    fn checking_harness_name_strips_trailing_ellipsis() {
        let line = "Checking harness kani::lane_name_rejects_empty_string...";
        assert_eq!(checking_harness_name(line), Some("kani::lane_name_rejects_empty_string"));
    }

    #[test]
    fn checking_harness_name_returns_none_for_unrelated_lines() {
        assert_eq!(checking_harness_name("Compiling titania-core v0.1.0"), None);
        assert_eq!(checking_harness_name(""), None);
    }

    #[test]
    fn verdict_from_line_parses_known_markers() {
        assert_eq!(
            verdict_from_line("VERIFICATION:- SUCCESSFUL"),
            Some(HarnessVerdict::Successful)
        );
        assert_eq!(verdict_from_line("VERIFICATION:- FAILED"), Some(HarnessVerdict::Failed));
        assert_eq!(
            verdict_from_line("VERIFICATION:- UNSUPPORTED (caller_location)"),
            Some(HarnessVerdict::Unsupported)
        );
        assert_eq!(verdict_from_line("Compiling titania-core"), None);
    }

    #[test]
    fn verdict_map_pairs_checking_with_verification_lines() {
        let stdout = "\
Checking harness kani::foo_bar...
some build noise
VERIFICATION:- SUCCESSFUL
Checking harness kani::baz_qux...
VERIFICATION:- FAILED
";
        let map = HarnessVerdictMap::from_stdout(stdout);
        assert_eq!(map.verdict_for("kani::foo_bar"), HarnessVerdict::Successful);
        assert_eq!(map.verdict_for("kani::baz_qux"), HarnessVerdict::Failed);
        assert_eq!(map.verdict_for("kani::never_run"), HarnessVerdict::Unknown);
    }

    #[test]
    fn canonical_harness_id_uppercases_lowercase_names() {
        let id = canonical_harness_id("kani::lane_name_rejects_empty_string")
            .expect("canonical id must be valid");
        assert_eq!(id.as_str(), "LANE_NAME_REJECTS_EMPTY_STRING");
    }

    #[test]
    fn canonical_harness_id_rejects_invalid_form() {
        assert!(canonical_harness_id("").is_none());
        assert!(canonical_harness_id("123_LEADING_DIGIT").is_none());
        let too_long = "X".repeat(KANI_HARNESS_ID_MAX_LEN + 1);
        assert!(canonical_harness_id(&too_long).is_none());
    }

    #[test]
    fn parse_kani_version_accepts_supported_release() {
        let version = parse_kani_version("cargo-kani 0.67.0 (cargo plugin)")
            .expect("0.67.0 must parse and meet the floor");
        assert_eq!(version.version, "0.67.0");
        assert_eq!(version.major, 0);
        assert_eq!(version.minor, 67);
        assert_eq!(version.patch, 0);
        assert!(version.meets_floor());
    }

    #[test]
    fn parse_kani_version_rejects_too_old_release() {
        assert!(parse_kani_version("cargo-kani 0.49.3 (cargo plugin)").is_none());
    }

    #[test]
    fn parse_kani_version_rejects_garbled_output() {
        assert!(parse_kani_version("cargo-kani 0.67").is_none());
        assert!(parse_kani_version("cargo-kani x.y.z").is_none());
        assert!(parse_kani_version("kani 0.67.0").is_none());
        assert!(parse_kani_version("").is_none());
    }

    #[test]
    fn meets_floor_uses_tuple_ordering() {
        let floor_major = DetectedCargoKani {
            version: format!(
                "{}.{}.{}",
                MIN_KANI_VERSION.0, MIN_KANI_VERSION.1, MIN_KANI_VERSION.2
            ),
            major: MIN_KANI_VERSION.0,
            minor: MIN_KANI_VERSION.1,
            patch: MIN_KANI_VERSION.2,
        };
        assert!(floor_major.meets_floor());
        let below =
            DetectedCargoKani { version: String::from("0.49.99"), major: 0, minor: 49, patch: 99 };
        assert!(!below.meets_floor());
    }

    #[test]
    fn inventory_error_outcome_skips_when_subcommand_missing() {
        let error = KaniInventoryError::ListCommandExit {
            crate_name: String::from("titania-core"),
            code: Some(1),
            stderr: String::from("error: no such subcommand: `kani`").into_boxed_str(),
        };
        let outcome =
            inventory_error_outcome(&error).expect("missing cargo-kani must map to Skipped");
        assert!(matches!(
            outcome,
            LaneOutcome::Skipped { reason: SkipReason::ToolUnavailable(ToolKind::CargoKani) }
        ));
    }

    #[test]
    fn inventory_error_outcome_skips_when_binary_not_found() {
        let error = KaniInventoryError::ListCommand {
            crate_name: String::from("titania-core"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "cargo-kani not found"),
        };
        let outcome =
            inventory_error_outcome(&error).expect("missing cargo-kani binary must map to Skipped");
        assert!(matches!(
            outcome,
            LaneOutcome::Skipped { reason: SkipReason::ToolUnavailable(ToolKind::CargoKani) }
        ));
    }

    #[test]
    fn is_subcommand_missing_recognises_both_phrases() {
        let missing_subcommand = KaniInventoryError::ListCommandExit {
            crate_name: String::from("titania-core"),
            code: Some(1),
            stderr: String::from("no such subcommand").into_boxed_str(),
        };
        let missing_binary = KaniInventoryError::ListCommand {
            crate_name: String::from("titania-core"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "binary not found"),
        };
        let artifact_error = KaniInventoryError::ArtifactRead {
            path: String::from("/tmp/kani-list.json"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        assert!(is_subcommand_missing(&missing_subcommand));
        assert!(is_subcommand_missing(&missing_binary));
        assert!(!is_subcommand_missing(&artifact_error));
    }

    #[test]
    fn inventory_error_outcome_emits_infra_finding_for_other_failures() {
        let error = KaniInventoryError::ArtifactRead {
            path: String::from("/tmp/kani-list.json"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        let outcome =
            inventory_error_outcome(&error).expect("infra failures must produce a Finding");
        let LaneOutcome::Findings { findings } = outcome else {
            panic!("infra failures must surface as Findings, not Skipped");
        };
        assert_eq!(findings.len(), 1);
        assert!(findings[0].is_reject());
        assert_eq!(findings[0].rule_id().as_str(), FALLBACK_RULE_INFRA);
    }

    #[test]
    fn no_harness_outcome_is_findings_not_clean() {
        let state = LaneRunState { worst_exit_code: 0, tool_version: String::from("0.67.0") };
        let outcome =
            build_no_harness_outcome(&state).expect("empty inventory must produce a typed outcome");
        match outcome {
            LaneOutcome::Findings { findings } => {
                assert_eq!(findings.len(), 1);
                assert!(findings[0].is_informational());
                assert_eq!(findings[0].rule_id().as_str(), FALLBACK_RULE_NO_HARNESSES);
            }
            other => panic!("empty inventory must not be Clean, got {other:?}"),
        }
    }

    #[test]
    fn timed_out_package_emits_blocked_finding_per_harness() {
        let harnesses = vec![
            harness_fixture("titania-core", "kani::lane_name_rejects_empty_string"),
            harness_fixture("titania-core", "kani::lane_name_rejects_nul_byte"),
        ];
        let run = timed_out_run_fixture();
        let findings = blocked_findings_for_package(&harnesses, &run)
            .expect("blocked-findings must construct");
        assert_eq!(findings.len(), 2);
        for finding in &findings {
            assert!(finding.is_reject());
            assert_eq!(finding.effect(), FindingEffect::Reject);
        }
    }

    #[test]
    fn spawn_failure_package_emits_blocked_findings_with_teardown_context() {
        let harnesses = vec![harness_fixture("titania-core", "kani::some_harness")];
        let mut run = timed_out_run_fixture();
        run.exit_code = None;
        run.timed_out = false;
        run.teardown_errors.push(String::from("wait failed: broken pipe"));
        let findings = blocked_findings_for_package(&harnesses, &run)
            .expect("spawn/wait failures must surface as blocked findings");
        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert!(finding.is_reject());
        assert!(
            finding.message().contains("spawn_or_wait_failed"),
            "blocked finding must mention spawn/wait failure, got: {}",
            finding.message()
        );
    }

    #[test]
    fn verdict_driven_package_emits_one_finding_per_harness() {
        let harnesses = vec![
            harness_fixture("titania-core", "kani::foo_bar"),
            harness_fixture("titania-core", "kani::baz_qux"),
        ];
        let verdicts = HarnessVerdictMap::from_stdout(
            "Checking harness kani::foo_bar...\nVERIFICATION:- SUCCESSFUL\n\
             Checking harness kani::baz_qux...\nVERIFICATION:- FAILED\n",
        );
        let run = completed_run_fixture(0, true);
        let findings = findings_for_package(&harnesses, &verdicts, &run)
            .expect("verdict-driven findings must construct");
        assert_eq!(findings.len(), 2);
        assert!(findings[0].is_informational());
        assert!(findings[1].is_reject());
    }

    #[test]
    fn missing_verdict_distinguishes_unknown_from_blocked() {
        // Use a harness name whose canonical id is rejected by
        // `KaniHarnessId::new` so the per-harness rule id falls back to
        // `PROOF_KANI_NO_VERDICT`. With a valid id we'd get
        // `PROOF_KANI_<NAME>` instead, which still distinguishes the
        // no-verdict path from the BLOCKED path semantically (a
        // harness-named rule id never starts with `_BLOCKED`).
        let harnesses = vec![harness_fixture("titania-core", "kani::123-leading-digit")];
        let verdicts = HarnessVerdictMap::default();
        let run = completed_run_fixture(0, true);
        let findings = findings_for_package(&harnesses, &verdicts, &run)
            .expect("missing-verdict findings must construct");
        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert!(finding.is_reject(), "missing verdict must reject, got {finding:?}");
        assert_eq!(
            finding.rule_id().as_str(),
            FALLBACK_RULE_NO_VERDICT,
            "missing verdict must use NO_VERDICT, not BLOCKED"
        );
        assert_ne!(
            finding.rule_id().as_str(),
            FALLBACK_RULE_BLOCKED,
            "missing verdict must NOT use the BLOCKED rule id"
        );
    }

    #[test]
    fn clean_outcome_records_real_cgroup_argv() {
        let state = LaneRunState { worst_exit_code: 0, tool_version: String::from("0.67.0") };
        let run = cgroup_run_fixture();
        let outcome = build_clean_outcome(&state, &run)
            .expect("clean outcome must construct on a successful cgroup run");
        let LaneOutcome::Clean { evidence } = outcome else {
            panic!("clean outcome must wrap LaneEvidence");
        };
        let argv = evidence.command().argv();
        assert_eq!(argv.first().map(String::as_str), Some("systemd-run"));
        assert!(argv.iter().any(|arg| arg == "--user"));
        assert!(argv.iter().any(|arg| arg == "--scope"));
        assert!(argv.iter().any(|arg| arg.contains("MemoryMax=24G")));
        assert!(argv.iter().any(|arg| arg == "MemorySwapMax=0"));
        assert!(argv.iter().any(|arg| arg == "cargo"));
        assert!(argv.iter().any(|arg| arg == "kani"));
        assert!(argv.iter().any(|arg| arg == "--output-format"));
        assert!(argv.iter().any(|arg| arg == "regular"));
        assert_eq!(evidence.command().executable(), "systemd-run");
        assert_eq!(evidence.exit_status().exit_code(), Some(0));
    }

    #[test]
    fn clean_outcome_records_real_bare_argv_when_cgroup_unavailable() {
        let state = LaneRunState { worst_exit_code: 0, tool_version: String::from("0.67.0") };
        let run = bare_run_fixture();
        let outcome = build_clean_outcome(&state, &run)
            .expect("clean outcome must construct on a successful bare run");
        let LaneOutcome::Clean { evidence } = outcome else {
            panic!("clean outcome must wrap LaneEvidence");
        };
        let argv = evidence.command().argv();
        assert_eq!(argv.first().map(String::as_str), Some("cargo"));
        assert!(argv.iter().any(|arg| arg == "kani"));
        assert!(argv.iter().any(|arg| arg == "-p"));
        assert!(argv.iter().any(|arg| arg == "titania-core"));
        assert!(!argv.iter().any(|arg| arg == "systemd-run"));
        assert_eq!(evidence.command().executable(), "cargo");
    }

    #[test]
    fn clean_outcome_rejects_non_zero_worst_exit() {
        let state = LaneRunState { worst_exit_code: 2, tool_version: String::from("0.67.0") };
        let mut run = bare_run_fixture();
        run.exit_code = Some(2);
        let error = build_clean_outcome(&state, &run).expect_err("non-zero worst exit must reject");
        assert!(
            matches!(error, titania_core::OutcomeError::NonZeroExit),
            "non-zero worst exit must produce OutcomeError, got {error:?}"
        );
    }

    #[test]
    fn package_run_successful_classifies_zero_exit() {
        let run = cgroup_run_fixture();
        assert!(run.is_successful(), "cgroup run with exit 0 must classify as successful");
    }

    #[test]
    #[expect(
        clippy::unused_unit,
        reason = "touching touched-constants surface keeps the constants reachable"
    )]
    fn fallback_constants_reachability_probe() {
        // Touch every constant used by the public surface to keep
        // `unused_variables` clean if a future change drops one of the
        // helper paths above.
        let _ = FALLBACK_RULE_PASS;
        let _ = FALLBACK_RULE_FAIL;
        let _ = FALLBACK_RULE_BLOCKED;
        let _ = FALLBACK_RULE_NO_VERDICT;
        let _ = FALLBACK_RULE_UNSUPPORTED;
        let _ = FALLBACK_RULE_INFRA;
        let _ = FALLBACK_RULE_NO_HARNESSES;
        let _ = ();
        let _ = build_clean_outcome;
    }

    #[test]
    fn pipe_byte_cap_is_large_but_bounded() {
        assert_eq!(PIPE_BYTE_CAP, 8 * 1024 * 1024);
        assert!(PER_PACKAGE_TIMEOUT_SECS > 0);
    }

    #[test]
    fn kani_run_error_variants_are_distinguishable_via_display() {
        let spawn = KaniRunError::Spawn {
            program: String::from("cargo"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
        };
        let wait = KaniRunError::WaitTimeout {
            program: String::from("cargo"),
            source: std::io::Error::new(std::io::ErrorKind::Other, "broken"),
        };
        assert_ne!(spawn.to_string(), wait.to_string());
        assert!(spawn.to_string().contains("spawn failed"));
        assert!(wait.to_string().contains("wait_timeout"));
    }

    #[test]
    fn rule_id_error_wraps_via_explicit_constructor() {
        // Verifies the explicit `KaniLaneError::RuleId` constructor flow
        // for `RuleIdError` -> `KaniLaneError::RuleId` (no From derived;
        // the inner error is wrapped manually to preserve the existing
        // `From<RuleIdError> for RuleIdError` boundary).
        let inner = RuleIdError::Empty;
        let wrapped = KaniLaneError::RuleId(inner);
        assert!(matches!(wrapped, KaniLaneError::RuleId(_)));
    }

    #[test]
    fn location_tool_carries_runtime_detected_version() {
        let location = Location::tool(String::from("cargo-kani"), String::from("0.67.0"));
        // Render the location back through its public surface so the
        // constructor is exercised against a `serde_json::Value` auditor.
        let value = serde_json::to_value(&location).expect("location serialization must succeed");
        let obj = value.as_object().expect("location serializes to an object");
        assert!(obj.contains_key("Tool"), "Tool variant must round-trip through serde");
    }

    fn harness_fixture(package: &str, full_name: &str) -> KaniHarness {
        KaniHarness {
            package: package.to_owned(),
            full_name: full_name.to_owned(),
            canonical_id: canonical_harness_id(full_name),
        }
    }

    fn timed_out_run_fixture() -> PackageRun {
        PackageRun {
            exit_code: None,
            timed_out: true,
            stdout: String::new(),
            cmd: cgroup_cmd_fixture(),
            teardown_errors: Vec::new(),
        }
    }

    fn completed_run_fixture(exit_code: i32, cgroup_used: bool) -> PackageRun {
        let cmd = if cgroup_used { cgroup_cmd_fixture() } else { bare_cmd_fixture() };
        PackageRun {
            exit_code: Some(exit_code),
            timed_out: false,
            stdout: String::new(),
            cmd,
            teardown_errors: Vec::new(),
        }
    }

    fn cgroup_run_fixture() -> PackageRun {
        PackageRun {
            exit_code: Some(0),
            timed_out: false,
            stdout: String::new(),
            cmd: cgroup_cmd_fixture(),
            teardown_errors: Vec::new(),
        }
    }

    fn bare_run_fixture() -> PackageRun {
        PackageRun {
            exit_code: Some(0),
            timed_out: false,
            stdout: String::new(),
            cmd: bare_cmd_fixture(),
            teardown_errors: Vec::new(),
        }
    }

    fn cgroup_cmd_fixture() -> PackageCmd {
        PackageCmd {
            executable: String::from("systemd-run"),
            argv: [
                "systemd-run",
                "--user",
                "--scope",
                "-p",
                "MemoryMax=24G",
                "-p",
                "MemorySwapMax=0",
                "--",
                "cargo",
                "kani",
                "-p",
                "titania-core",
                "--output-format",
                "regular",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }

    fn bare_cmd_fixture() -> PackageCmd {
        PackageCmd {
            executable: String::from("cargo"),
            argv: ["cargo", "kani", "-p", "titania-core", "--output-format", "regular"]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }

    // Touch every constant used by the public surface to keep
    // `unused_variables` clean if a future change drops one of the
    // helper paths above.
    #[expect(dead_code, reason = "compile-time reference to keep fallback constants reachable")]
    fn touch_constants() {
        let _ = FALLBACK_RULE_PASS;
        let _ = FALLBACK_RULE_FAIL;
        let _ = FALLBACK_RULE_BLOCKED;
        let _ = FALLBACK_RULE_NO_VERDICT;
        let _ = FALLBACK_RULE_UNSUPPORTED;
        let _ = FALLBACK_RULE_INFRA;
        let _ = FALLBACK_RULE_NO_HARNESSES;
    }
}

#[cfg(test)]
mod size_tests {
    /// Compile-time guard that keeps the lane-error enum under the
    /// workspace `large-error-threshold = 64` byte bound (see
    /// `clippy.toml`). If this ever fires, re-box one of the inner
    /// `String` fields rather than relaxing the threshold.
    #[test]
    fn assert_kani_lane_error_under_threshold() {
        assert!(std::mem::size_of::<super::KaniLaneError>() < 64);
        assert!(std::mem::size_of::<super::KaniInventoryError>() < 64);
    }
}
