| Requirement | Proof ID | Refinement ID | Source Refs | Behavior Test Refs | Refinement Harness Refs | Commands Run | Ledger Result | Status |
|-------------|----------|---------------|-------------|--------------------|--------------------------|--------------|---------------|--------|
# Proof-Test-Source Alignment (State 12)

| Proof | Test | Production code | Status |
|-------|------|-----------------|--------|
| v15-OBL-P1 | tests/v15_lane_roundtrip.rs | crates/titania-core/src/lane.rs | aligned |
| v15-OBL-P2 | tests/v15_gate_scope_roundtrip.rs | crates/titania-core/src/gate_scope.rs | aligned |
| v15-OBL-P3 | tests/v15_kani_harness_id.rs + Kani harness | crates/titania-core/src/proof_id.rs | aligned |
| v15-OBL-P4 | tests/v15_kani_harness_id_serde.rs | crates/titania-core/src/proof_id.rs | aligned |
| v15-OBL-P5 | tests/v15_mutant_id.rs | crates/titania-core/src/proof_id.rs | aligned |
| v15-OBL-V1 | tests/v15_mutant_id.rs + Verus spec | crates/titania-core/src/proof_id.rs | aligned |
| v15-OBL-P6 | tests/v15_mutants_baseline_load.rs | crates/titania-core/src/mutants_baseline.rs | aligned |
| v15-OBL-P7 | tests/v15_mutants_baseline_diff.rs | crates/titania-core/src/mutants_baseline.rs | aligned |
| v15-OBL-K2 | tests/v15_mutants_baseline_diff.rs + Kani harness | crates/titania-core/src/mutants_baseline.rs | aligned |
| v15-OBL-P8 | tests/v15_baseline_expiry.rs | crates/titania-core/src/mutants_baseline.rs | aligned |
| v15-OBL-P9 | tests/v15_skip_reason_tool_unavailable.rs | crates/titania-core/src/outcome.rs | aligned |
| v15-OBL-P10 | tests/v15_lane_name.rs | crates/titania-core/src/lane.rs | aligned |
| v15-OBL-P11 | tests/v15_lane_serde_roundtrip.rs | crates/titania-core/src/lane.rs | aligned |
| v15-OBL-K1 | src/kani.rs (no behavior test; Kani-only) | crates/titania-core/src/proof_id.rs | aligned |
| v15-OBL-L1 | tests/v15_atomic_baseline.rs (loom model) | crates/titania-lanes/src/artifact_writer.rs | aligned |
| v15-OBL-F1 | fuzz_targets/fuzz_parse_inventory.rs (no behavior test; fuzz-only) | crates/titania-core/src/kani_inventory.rs | aligned |
| v15-OBL-F2 | fuzz_targets/fuzz_parse_outcomes.rs (no behavior test; fuzz-only) | crates/titania-core/src/mutants_outcomes.rs | aligned |

## Coverage rule
Every proof obligation has ≥1 of: behavior test / Kani harness / Verus spec / loom model / fuzz target. No orphan proofs.

## Alignment status
All 18 obligations aligned.
