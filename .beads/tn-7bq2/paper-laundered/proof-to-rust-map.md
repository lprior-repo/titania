| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|----------|-------|---------------------|------------------|--------------------|--------------------------|----------|------------------|------------|
reviewer_skill: proof-writer
reviewer_invocation_id: s7.tn-7bq2.2.proof-to-implementation
writer_invocation_id: s5.tn-7bq2.2.proof-writer

# Proof-to-Rust Map (State 7)

| Proof obligation | Rust target (production) | Behavior test ref | Refinement harness ref |
|------------------|---------------------------|--------------------|------------------------|
| v15-OBL-P1-LANE-ROUNDTRIP | crates/titania-core/src/lane.rs::Lane::from_str | tests/v15_lane_roundtrip.rs (proptest) | (same proptest) |
| v15-OBL-P2-GATESCOPE-ROUNDTRIP | crates/titania-core/src/gate_scope.rs::GateScope::from_str | tests/v15_gate_scope_roundtrip.rs | (same) |
| v15-OBL-P3-KANI-ID-PROPTEST | crates/titania-core/src/proof_id.rs::KaniHarnessId::new | tests/v15_kani_harness_id.rs | (same) |
| v15-OBL-P3-KANI-ID-KANI | crates/titania-core/src/proof_id.rs::KaniHarnessId::new | tests/v15_kani_harness_id.rs | src/kani.rs::kani_kani_harness_id_bounded |
| v15-OBL-P4-KANI-ID-SERDE | crates/titania-core/src/proof_id.rs::KaniHarnessId serde | tests/v15_kani_harness_id_serde.rs | (same) |
| v15-OBL-P5-MUTANT-ID-PROPTEST | crates/titania-core/src/proof_id.rs::MutantId::new | tests/v15_mutant_id.rs | (same) |
| v15-OBL-V1-MUTANT-ID-VERUS | crates/titania-core/src/proof_id.rs::MutantId::new | tests/v15_mutant_id.rs | src/proof_id.rs::spec::spec_mutant_id_closed_set |
| v15-OBL-P6-MUTANTS-LOAD | crates/titania-core/src/mutants_baseline.rs::MutantsBaseline::load | tests/v15_mutants_baseline_load.rs | (same) |
| v15-OBL-P7-MUTANTS-DIFF-PROPTEST | crates/titania-core/src/mutants_baseline.rs::MutantsBaseline::diff | tests/v15_mutants_baseline_diff.rs | (same) |
| v15-OBL-K2-MUTANTS-DIFF-KANI | crates/titania-core/src/mutants_baseline.rs::diff | tests/v15_mutants_baseline_diff.rs | src/kani.rs::kani_mutants_baseline_diff_zero_neg |
| v15-OBL-P8-MUTANTS-EXPIRY | crates/titania-core/src/mutants_baseline.rs::MutantBaselineEntry::expires_on | tests/v15_baseline_expiry.rs | (same) |
| v15-OBL-P9-SKIP-REASON-TOOL | crates/titania-core/src/outcome.rs::SkipReason::ToolUnavailable | tests/v15_skip_reason_tool_unavailable.rs | (same) |
| v15-OBL-P10-LANE-NAME | crates/titania-core/src/lane.rs::Lane::name | tests/v15_lane_name.rs | (same) |
| v15-OBL-P11-LANE-SERDE | crates/titania-core/src/lane.rs serde | tests/v15_lane_serde_roundtrip.rs | (same) |
| v15-OBL-K1-KANI-NAME-KANI | crates/titania-core/src/proof_id.rs::KaniHarnessId::new | (use existing kani harness area) | src/kani.rs::kani_kani_lane_name_roundtrip |
| v15-OBL-L1-ATOMIC-LOAD-LOOM | crates/titania-lanes/src/artifact_writer.rs::atomic_write | tests/v15_atomic_baseline.rs (loom model) | (same loom test) |
| v15-OBL-F1-FUZZ | crates/titania-core/src/kani_inventory.rs::parse_inventory | fuzz/fuzz_targets/fuzz_parse_inventory.rs | (fuzz harness) |
| v15-OBL-F2-FUZZ | crates/titania-core/src/mutants_outcomes.rs::parse_outcomes | fuzz/fuzz_targets/fuzz_parse_outcomes.rs | (fuzz harness) |

## Bridge rules

- Source_refs in `rust-refinement-obligations.jsonl` use `path::symbol` form.
- Behavior-test refs are independent of refinement harness refs (validator rule E_BEHAVIOR_TEST_NOT_INDEPENDENT).
- mapping_status is `planned` at State 7; will become `verified` after formal-verifier (State 12) executes the bridge evidence commands.

## Anti-circular

No production code was copied into harness files. Each harness (`#[kani::proof]`, `loom::model::*`, `cargo_fuzz::*`) targets the production function directly or through a Verus spec function that wraps the same Rust code.
