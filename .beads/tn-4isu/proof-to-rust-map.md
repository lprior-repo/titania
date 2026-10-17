# Proof-to-Rust map — tn-4isu

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|---|---|---|---|---|---|---|---|---|
| tn4isu.OBL-DIFF | diff survivor-superset at unwind 16 | false | crates/titania-core/src/kani.rs::kani_mutants_baseline_diff_zero_neg | (none: proof-only change) | crates/titania-core/src/kani.rs::kani_mutants_baseline_diff_zero_neg | kani | cargo kani -p titania-core -j 1 --harness kani_mutants_baseline_diff_zero_neg | current-revision |
| tn4isu.OBL-WIRE | LaneOutcome round-trip at unwind 24 | false | crates/titania-core/src/kani.rs::outcome_wire_invariants | (none: proof-only change) | crates/titania-core/src/kani.rs::outcome_wire_invariants | kani | cargo kani -p titania-core -j 1 --harness outcome_wire_invariants | current-revision |
| tn4isu.OBL-ID | id-harness cost attribution, bound kept | false | crates/titania-core/src/kani.rs::kani_kani_harness_id_bounded | (none: proof-only change) | crates/titania-core/src/kani.rs::kani_kani_harness_id_bounded | kani | cargo kani -p titania-core -j 1 --harness kani_kani_harness_id_bounded | current-revision |
| tn4isu.OBL-FULL | 12/12 SUCCESSFUL within 600s | false | crates/titania-core/src/kani.rs::outcome_wire_invariants | (none: proof-only change) | all 12 harnesses in crates/titania-core/src/kani.rs | kani | cargo kani -p titania-core -j 1 | current-revision |
| tn4isu.OBL-DIFF-SPLIT | split-harness DIFF verdicts at unwind 16 | false | crates/titania-core/src/kani.rs::kani_mutants_diff_first_half, kani_mutants_diff_second_half, kani_mutants_diff_mixed_boundary_fixed, kani_mutants_diff_all_active_fixed | (none: proof-only change) | same four harnesses | kani | cargo kani -p titania-core -j 1 --harness kani_mutants_diff_first_half --harness kani_mutants_diff_second_half --harness kani_mutants_diff_mixed_boundary_fixed --harness kani_mutants_diff_all_active_fixed | current-revision |
| tn4isu.OBL-WIRE-SER | serialise-totality or recorded infeasibility | false | crates/titania-core/src/kani.rs::outcome_wire_serialise_bounded | crates/titania-core/tests/artifact_roundtrip.rs (5/5), crates/titania-core/tests/json_roundtrip.rs (22/22) | outcome_wire_serialise_bounded | kani | cargo kani -p titania-core -j 1 --harness outcome_wire_serialise_bounded | current-revision |
| tn4isu.OBL-ID-SPLIT | split-harness ID verdicts, valid at unwind 34 | false | crates/titania-core/src/kani.rs::kani_harness_id_valid_bounded, kani_harness_id_invalid_first, kani_harness_id_invalid_body_and_fixed | crates/titania-core/tests/v15_kani_harness_id.rs (256-exhaustive, boundaries, offsets) | same three harnesses | kani | cargo kani -p titania-core -j 1 --harness kani_harness_id_valid_bounded --harness kani_harness_id_invalid_first --harness kani_harness_id_invalid_body_and_fixed | current-revision |
| tn4isu.OBL-VERUS-KERNEL | khi body-byte contract bound to production kernel, grade WEAK_EXTERN with TRUSTED contract | true | crates/titania-core/src/proof_id/kernel.rs::is_khi_byte (called by proof_id.rs::check_khi_byte) | crates/titania-core/tests/v15_kani_harness_id.rs::body_alphabet_matches_std_byte_classes_for_all_256_bytes plus boundary/offset tests (execute production body) | kani_harness_id_valid_bounded, kani_harness_id_invalid_first (symbolic constructor paths) | verus | verus verification/verus/khi_kernel.rs | current-revision |

All rows behavior_affecting=false: the change is unwind attributes on
`cfg(kani)` harnesses. Production code, lane code, and configs are untouched,
so no behavior test can observe a difference — the harness verdicts ARE the
evidence. No bridge row hides a behavior change.

Addendum: the OBL-VERUS-KERNEL row above is behavior_affecting=true. Its
production change (kernel extraction into proof_id/kernel.rs, called by
check_khi_byte) is behavior-preserving by construction and pinned by the cited
256-exhaustive plus boundary/offset behavior tests, which execute the moved
code through the public constructor. The map claims no Kani verdicts for rows
whose harnesses have not produced terminal SUCCESS; absent verdicts are
MISSING_EVIDENCE, not passes.
