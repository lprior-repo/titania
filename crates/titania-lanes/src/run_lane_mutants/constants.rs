//! Static string constants and rule ids used by the mutants lane.
//!
//! Centralised so every module shares the same locked-spec literals and
//! the lane never duplicates the typed rule-id constants across files.

/// Output directory passed to cargo-mutants via `--output`. cargo-mutants
/// 27 places its JSON artifacts at `<output>/mutants.out/{outcomes.json,
/// mutants.json}`.
pub(super) const MUTANTS_OUTPUT_DIR: &str = "mutants.out";

/// Per-invocation wallclock cap (seconds).
///
/// The workspace-level command is one cargo-mutants run; 30 minutes is a
/// service containment cap (H1 equivalent for mutants), not a static
/// bound proof. Hosts that need more wallclock can override via
/// operational processes; the lane never silently extends the cap.
pub(super) const MUTANTS_WALLCLOCK_TIMEOUT_SECS: u64 = 1800;

/// Minimum cargo-mutants major version required by v1.5 spec §7.
/// cargo-mutants 27.x is the development target; anything below 25 maps
/// to `SkipReason::ToolUnavailable(CargoMutants)`.
pub(super) const MUTANTS_VERSION_FLOOR_MAJOR: u32 = 25;

/// Tool name recorded in `Location::tool(...)` and `LaneFailure::Infra`.
pub(super) const MUTANTS_TOOL: &str = "cargo-mutants";

/// Cgroup memory high-water mark; matches the brief.
pub(super) const CGROUP_MEMORY_HIGH: &str = "20G";

/// Cgroup memory hard cap; matches the brief.
pub(super) const CGROUP_MEMORY_MAX: &str = "24G";

/// Rule id for a survivor outside the baseline.
pub(super) const RULE_MUTANT_SURVIVED: &str = "MUTANT_SURVIVED";

/// Rule id emitted when `.titania/profiles/strict-ai/mutants.baseline.json`
/// is absent.
pub(super) const RULE_MUTANT_BASELINE_MISSING: &str = "MUTANT_BASELINE_MISSING";
