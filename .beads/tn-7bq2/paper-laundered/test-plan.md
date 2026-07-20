# Test Plan (State 8 — test-planner)

## Goal
Prove the v1.5 contract end-to-end via behavior tests + property tests + loom + fuzz.

## Behavior-test matrix

| Test target (Rust) | Test name (proptest or #[test]) | Inputs | Expected |
|--------------------|-------------------------------------|--------|----------|
| `crates/titania-core/src/lane.rs::Lane::from_str` | `tests/v15_lane_roundtrip.rs::lane_round_trip` | `b"Kani"`, `"Mutants"`, `b"Edit"`, all variants | `Ok(...)` identity |
| `crates/titania-core/src/gate_scope.rs::GateScope::from_str` | `tests/v15_gate_scope_roundtrip.rs::gate_scope_round_trip` | `b"full"`, `"full"` | `Ok(...)` identity |
| `crates/titania-core/src/proof_id.rs::KaniHarnessId::new` | `tests/v15_kani_harness_id.rs::kani_harness_id_accepts_uppercase` and rejected variants | `"FOO_BAR"`, `""`, `"lower"`, `"5F"`, `"abc.de"` | accept/reject invariant |
| `crates/titania-core/src/proof_id.rs` KaniHarnessId serde | `tests/v15_kani_harness_id_serde.rs::serde_roundtrip` | any `KaniHarnessId` | identity |
| `crates/titania-core/src/proof_id.rs::MutantId::new` | `tests/v15_mutant_id.rs::mutant_id_acceptance` | `("pkg","a/b.rs",12,3,"equal_replace")` etc | accept; rejects bad inputs |

## Proof/Refinement Coverage Matrix

Bridge-of-record between rust-refinement-obligations.jsonl and test files:

| Obligation | Test ref | Refinement ref |
|------------|----------|----------------|
| v15-OBL-P1 | tests/v15_lane_roundtrip.rs | (same) |
| v15-OBL-P2 | tests/v15_gate_scope_roundtrip.rs | (same) |
| v15-OBL-P3 | tests/v15_kani_harness_id.rs | src/kani.rs::kani_kani_harness_id_bounded |
| v15-OBL-P4 | tests/v15_kani_harness_id_serde.rs | (same) |
| v15-OBL-P5 | tests/v15_mutant_id.rs | src/proof_id.rs::spec::spec_mutant_id_closed_set |
| v15-OBL-P6 | tests/v15_mutants_baseline_load.rs | (same) |
| v15-OBL-P7 | tests/v15_mutants_baseline_diff.rs | src/kani.rs::kani_mutants_baseline_diff_zero_neg |
| v15-OBL-P8 | tests/v15_baseline_expiry.rs | (same) |
| v15-OBL-P9 | tests/v15_skip_reason_tool_unavailable.rs | (same) |
| v15-OBL-P10 | tests/v15_lane_name.rs | (same) |
| v15-OBL-P11 | tests/v15_lane_serde_roundtrip.rs | (same) |
| v15-OBL-L1 | tests/v15_atomic_baseline.rs (loom) | (loom model) |
| v15-OBL-F1 | fuzz_targets/fuzz_parse_inventory.rs | (fuzz harness) |
| v15-OBL-F2 | fuzz_targets/fuzz_parse_outcomes.rs | (fuzz harness) |
| `crates/titania-core/src/mutants_baseline.rs::load` | `tests/v15_mutants_baseline_load.rs::{happy_path,missing_path,wrong_schema,invalid_entry}` | paths | returns expected errors |
| `crates/titania-core/src/mutants_baseline.rs::diff` | `tests/v15_mutants_baseline_diff.rs::diff_set` | survivors + baselines | set difference |
| `crates/titania-core/src/mutants_baseline.rs` expiry | `tests/v15_baseline_expiry.rs::expired_entries_suppressed` | expired entries | `expired` triggers no-suppress |
| `crates/titania-core/src/outcome.rs::SkipReason::ToolUnavailable` | `tests/v15_skip_reason_tool_unavailable.rs::serde_roundtrip` | `ToolKind::CargoKani`, `ToolKind::CargoMutants` | round-trips |
| `crates/titania-core/src/lane.rs::Lane::name` | `tests/v15_lane_name.rs::lane_name_uniqueness` | all 12 variants | pairwise unequal |
| `crates/titania-core/src/lane.rs` serde | `tests/v15_lane_serde_roundtrip.rs::lane_serde_roundtrip` | all 12 | identity |
| `crates/titania-lanes/src/artifact_writer.rs::atomic_write` (under `cfg(loom)`) | `tests/v15_atomic_baseline.rs::atomic_baseline_load_concurrent` | 2 threads, multiple writes | both threads see consistent baseline |
| `crates/titania-core/src/kani_inventory.rs::parse_inventory` | fuzz_target `fuzz_parse_inventory.rs` | random bytes | `Ok` or typed error, never panic |
| `crates/titania-core/src/mutants_outcomes.rs::parse_outcomes` | fuzz_target `fuzz_parse_outcomes.rs` | random bytes | `Ok` or typed error, never panic |

## Mutation testing plan

After the test suite passes on a clean tree, the operator runs the
mutants bootstrap recipe (per `.evidence/v1.5/spec.md §4.4`):

1. Per-package `cargo mutants --no-shuffle --output mutants.out -p <pkg>`.
2. For each survivor, write a unit test that fails after mutation.
3. Accept the test survival by adding a `mutant-accept/<owner>/<reason>/<expiry>` entry to `.titania/profiles/strict-ai/mutants.baseline.json`.
4. After zero survivors remain, commit the baseline.

## Bridge to behavior tests

`rust-refinement-obligations.jsonl` lists each proof obligation's
`behavior_test_refs`. Each proptest test above corresponds to one
refinement obligation, ensuring independent executable behavior
coverage.
