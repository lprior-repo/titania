# Proof-to-Rust map — tn-4isu

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|---|---|---|---|---|---|---|---|---|
| tn4isu.OBL-DIFF | diff survivor-superset at unwind 16 | false | crates/titania-core/src/kani.rs::kani_mutants_baseline_diff_zero_neg | (none: proof-only change) | crates/titania-core/src/kani.rs::kani_mutants_baseline_diff_zero_neg | kani | cargo kani -p titania-core -j 1 --harness kani_mutants_baseline_diff_zero_neg | current-revision |
| tn4isu.OBL-WIRE | LaneOutcome round-trip at unwind 24 | false | crates/titania-core/src/kani.rs::outcome_wire_invariants | (none: proof-only change) | crates/titania-core/src/kani.rs::outcome_wire_invariants | kani | cargo kani -p titania-core -j 1 --harness outcome_wire_invariants | current-revision |
| tn4isu.OBL-ID | id-harness cost attribution, bound kept | false | crates/titania-core/src/kani.rs::kani_kani_harness_id_bounded | (none: proof-only change) | crates/titania-core/src/kani.rs::kani_kani_harness_id_bounded | kani | cargo kani -p titania-core -j 1 --harness kani_kani_harness_id_bounded | current-revision |
| tn4isu.OBL-FULL | 12/12 SUCCESSFUL within 600s | false | crates/titania-core/src/kani.rs::outcome_wire_invariants | (none: proof-only change) | all 12 harnesses in crates/titania-core/src/kani.rs | kani | cargo kani -p titania-core -j 1 | current-revision |

All rows behavior_affecting=false: the change is unwind attributes on
`cfg(kani)` harnesses. Production code, lane code, and configs are untouched,
so no behavior test can observe a difference — the harness verdicts ARE the
evidence. No bridge row hides a behavior change.
