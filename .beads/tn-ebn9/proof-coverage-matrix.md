# tn-ebn9 Proof Coverage Matrix

| Obligation | Seed | Verifier | Evidence | Ledger |
|---|---|---|---|---|
| tn-ebn9.O-P1-PROP | tn-ebn9.P1 | proptest | Dylint `bounded_execution_permits_finite_for_with_question` + lane `real_for_loop` clean + killer-demo `bounded_for_with_question_passes` | PASS |
| tn-ebn9.O-P2-PROP | tn-ebn9.P2 | proptest | Dylint `bounded_execution_rejects_unbounded_iterators` + lane `real_unbounded_violations_still_detected` + killer-demo bad rejects | PASS |
| tn-ebn9.O-P3-PROP | tn-ebn9.P3 | proptest | Dylint `bounded_execution_permits_while_let` + lane `allowed_bounded_for` (while-let arm) | PASS |
| tn-ebn9.O-P4-PROP | tn-ebn9.P4 | proptest | Lane `real_for_loop` clean vs `real_unbounded_violations` findings + `bounded_execution_unbounded_loop_violation` vs `..._iterator_violation` same-disposition pair | PASS |

Bridge (RRO) coverage: tn-ebn9.R-P1..R-P4 map each obligation to
production source refs + disjoint behavior-test / refinement-harness
refs with executed command evidence (see `proof-to-rust-map.md`).
