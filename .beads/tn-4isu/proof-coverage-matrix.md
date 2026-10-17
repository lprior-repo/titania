# Proof coverage matrix — tn-4isu

| Obligation | Harness | Property class | Bound | Status |
|---|---|---|---|---|
| tn4isu.OBL-DIFF | kani_mutants_baseline_diff_zero_neg | state-transition (survivor ⊆ diff ∪ covered) + panic-freedom | unwind 16 | planned → execute State 12 |
| tn4isu.OBL-WIRE | outcome_wire_invariants | round-trip equality + panic-freedom | unwind 24 | planned → execute State 12 |
| tn4isu.OBL-ID | kani_kani_harness_id_bounded | constructor accept/reject exactness | unwind 100 | planned → triage first |
| tn4isu.OBL-FULL | all 12 | inventory reconciliation | lane 600s | planned → after per-harness |

The 8 fixed-input + 2 u32-assume harnesses carry no unwind annotation and are
covered by OBL-FULL; triage runs the lightest one as a canary.
