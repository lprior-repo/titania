# v1.5 Functional / Shell Boundary Map

> Maps every v1.5 boundary action to its layer, and which layer owns
> which concern. The functional-core / imperative-shell doctrine in
> AGENTS.md §3 requires this; it is also the basis for the type-driven
> design under §3 of the contract.

## Functional core (`crates/titania-core/src/`)

Pure; no I/O, no clock, no network, no filesystem, no environment.

### Concerns owned

- All v1.5 newtypes (`KaniHarnessId`, `MutantId`, `MutantsBaseline`,
  `MutantBaselineEntry`, `ToolKind`, `KaniRunOutcome`,
  `HarnessOutcome`).
- All `LaneOutcome` constructors.
- All `RuleId` validation for the new id strings.
- All `Lane`/`GateScope`/`SkipReason` constructors and predicates.
- The JSON harness inventory parser (cargo kani list `--format json`).
- The `mutants.out/outcomes.json` and `mutants.json` parsers.
- The `MutantsBaseline::diff` algorithm (pure survivor-set
  difference).
- All `serde::Serialize`/`Deserialize` round-trips for v1.5 artifacts.

### Forbidden

- `std::fs`, `std::env`, `std::process`, `std::time::SystemTime`,
  `tokio`, `serde_json::from_reader` (must use `&str` only),
  `Cargo` discovery, `systemd-run`, `cbml`, `mysql`, `sqlite`.

## Imperative shell (`crates/titania-lanes/src/`, `crates/titania-check`)

I/O and effects. Owns every command spawn, cgroup setup, filesystem
write, network call, and clock read.

### Concerns owned

- Spawning `cargo-kani` and `cargo-mutants` processes with the right
  flags (per-crate enumeration for Kani; full test mode for Mutants).
- Cgroup wrapping for Kani runs (`MemoryMax=24G`, `MemorySwapMax=0`,
  `-j 1`). Uses `systemd-run --user --scope --collect` when available;
  fallback to direct spawn with documented risk.
- Reading `kjson inventory`. Mutants output via `CommandIn`.
- Writing `.titania/out/full/{kani,mutants}.json`.
- Writing the baseline bootstrap prompt to stderr when
  `MUTANT_BASELINE_MISSING` fires.
- Mapping typed core errors to lane artifacts.
- Tool version detection (`cargo kani --version`, `cargo mutants
  --version`).

### Forbidden

- File-system manipulation outside the lane artifact directory
  (`.titania/out/<scope>/<lane>.json` and the workspace
  `.titania/profiles/strict-ai/mutants.baseline.json`).
- Spawning arbitrary binaries beyond `cargo-kani`, `cargo-mutants`,
  `systemd-run`, and the existing `titania-check` toolchain.

## Boundary types

| Boundary | Source | Sink | Tooling |
|----------|--------|------|---------|
| cargo-kani inventory parse | `titania-lanes/src/run_lane_kani.rs` | `titania-core/src/kani_inventory.rs` (new) | `serde_json` |
| cargo-mutants outcomes parse | `titania-lanes/src/run_lane_mutants.rs` | `titania-core/src/mutants_outcomes.rs` (new) | `serde_json` |
| Baseline load | `titania-lanes/src/run_lane_mutants.rs` | `titania-core/src/mutants_baseline.rs` (new) | `serde_json` |
| HarnessOutcome → Finding | `titania-core/src/outcome.rs` | `titania-lanes/src/run_lane_kani.rs` | borrow |
| Survivor diff | `titania-core/src/mutants_baseline.rs` | `titania-lanes/src/run_lane_mutants.rs` | borrow |

## Async/await

v1 lanes are sync (per the v1.0 post-mortem). v1.5 inherits that
discipline; new lanes MUST also be sync. Any future need for async is
deferred to a contract review.

## Concurrency

Per v1 spec §4.7, multi-harness Kani runs serialize one harness at a
time. The lane uses `Iterator::try_for_each` with the per-harness
cgroup wrapper. Parallelism across harnesses is not in v1.5 scope.

## Error-path discipline

- Every fallible path returns `Result<T, _>` where the error is a
  typed thiserror enum.
- `Result<T, String>` is forbidden in core.
- `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`,
  `unreachable!`, and unchecked indexing/slicing are denied by
  workspace lints. The new code keeps that promise.

## Verification

- Pure-core Kani harnesses get new `#[kani::proof]` annotations in
  `crates/titania-core/src/kani.rs` (proof-writer task `tn-7bq2.2`).
- Pure-core Verus specs (where they exist; v1.5 defers hot-path Verus
  to v2.5).
- Pure-core proptest properties for `MutantsBaseline::diff`,
  `KaniHarnessId::new`, `MutantId::new`.
- Pure-core unit tests with `#[test]` where pure logic is non-property.
- Lane shell: per-package integration tests with fixture harnesses
  (test-planner/writer tasks for v1.5).
