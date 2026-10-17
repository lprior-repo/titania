# Proof/test/source alignment — tn-4isu

| Requirement | Proof ID | Refinement ID | Source Refs | Behavior Test Refs | Refinement Harness Refs | Commands Run | Ledger Result | Status |
|---|---|---|---|---|---|---|---|---|
| tn4isu-REQ-BOUNDED-DIFF | tn4isu.OBL-DIFF | tn4isu.RRO-DIFF | crates/titania-core/src/kani.rs::kani_mutants_baseline_diff_zero_neg | (none: proof-only) | crates/titania-core/src/kani.rs::kani_mutants_baseline_diff_zero_neg | 2 kani runs (u32/u16) | FAIL_LOCAL | blocked-open |
| tn4isu-REQ-BOUNDED-WIRE | tn4isu.OBL-WIRE | tn4isu.RRO-WIRE | crates/titania-core/src/kani.rs::outcome_wire_invariants | (none: proof-only) | crates/titania-core/src/kani.rs::outcome_wire_invariants | 3 kani runs (u64/u24/u12) + addendum A1 | FAIL_LOCAL | blocked-open |
| tn4isu-REQ-DIAGNOSE-ID | tn4isu.OBL-ID | tn4isu.RRO-ID | crates/titania-core/src/kani.rs::kani_kani_harness_id_bounded | (none: proof-only) | crates/titania-core/src/kani.rs::kani_kani_harness_id_bounded | 1 kani run (u100) | FAIL_LOCAL | blocked-open |
| tn4isu-REQ-INVENTORY | tn4isu.OBL-FULL | tn4isu.RRO-FULL | crates/titania-core/src/kani.rs::outcome_wire_invariants | titania-core cargo suite 96/96 green | all 12 harnesses | 9 light/lanename passes + floor arithmetic | FAIL_LOCAL | blocked-open |

9/12 harnesses verified SUCCESSFUL (raw logs in evidence/); 3/12 blocked with
exact numbers. Bead stays OPEN; Full stays blocked. No divergence between proof
claims, test evidence, and source state.
