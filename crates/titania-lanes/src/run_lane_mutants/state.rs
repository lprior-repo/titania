//! Runtime state types carried across the mutants-lane pipeline.
//!
//! These types stay small, single-purpose, and own only the data the
//! orchestrator needs to thread through the workspace-level run. They
//! have no I/O and no derived behaviour beyond plain data access.

use titania_core::MutantId;

/// Loaded and parsed cargo-mutants `outcomes.json` plus the populated
/// survivor records (one per `MissedMutant`).
#[derive(Debug, Default)]
pub(super) struct MutantsReport {
    /// Mutation names cargo-mutants reported as surviving the test run.
    pub(super) survivor_names: Vec<String>,
    /// Exit code from the `cargo mutants` invocation.
    pub(super) exit_code: i32,
}

/// Mutable aggregator carried across the per-survivor build loop.
///
/// Holds the runtime-detected version string and the cgroup policy
/// decision so the final `LaneEvidence` argv reflects the actual command
/// that ran.
#[derive(Debug, Default)]
pub(super) struct LaneRunState {
    /// cargo-mutants version string reported by the binary (e.g.
    /// `cargo-mutants 27.0.0`).
    pub(super) tool_version: String,
    /// Cgroup wrapper policy: `true` when `systemd-run --user --scope`
    /// is applied; `false` for the bare cargo-mutants fallback.
    pub(super) cgroup_used: bool,
}

/// Parsed survivor record carrying real per-mutation identity.
///
/// Previous iterations stored only the typed [`MutantId`], which carried
/// the package/path/line/col line but no `Location` evidence usable for
/// receipts. The new shape propagates every field the receipt expects.
#[derive(Debug, Clone)]
pub(super) struct NewSurvivor {
    /// Cargo package the survivor belongs to.
    pub(super) package: String,
    /// Source-relative path within the package (e.g. `src/foo.rs`).
    pub(super) rel_path: String,
    /// 1-based source line per cargo-mutants.
    pub(super) line: u32,
    /// 1-based source column per cargo-mutants.
    pub(super) column: u32,
    /// cargo-mutants genre tag (preserved for diagnostics).
    pub(super) genre: String,
    /// cargo-mutants textual replacement (preserved for diagnostics).
    pub(super) replacement: String,
    /// Raw cargo-mutants name (preserved for diagnostics and
    /// human-readable echoes).
    pub(super) raw_name: String,
    /// Typed mutation id derived from `rel_path`+line+column+operator.
    pub(super) typed_id: MutantId,
}
