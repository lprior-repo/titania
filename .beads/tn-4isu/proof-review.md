# Proof review — tn-4isu

reviewer_skill: proof-reviewer
reviewer_invocation_id: inv-s06-proofreviewer
writer_invocation_id: inv-s05-proofwriter
review_state: 6
reviewed_artifacts: crates/titania-core/src/kani.rs, proof-writer-report.md, proof-evidence.md, trusted-base-ledger.jsonl

## Checks

1. Diff parity: `git diff` = 1 file, 3+/3-; both attribute changes match D4/D5;
   comment change matches. No assert/assume-shaping/cover/input/production edit.
   PASS (finding F1, fixed with the diff itself as evidence).
2. Bound justification: T1 (16 ≥ 15-byte max comparison, production loops ≤ 8),
   T2 (24 over ≤ 8-byte input; pre-change stall documented at iteration 32+),
   T3 retained with reason. Each carries a revert condition on unwinding
   failure. PASS.
3. Trust ledger: T0–T5 rows present, all behavior_affecting=false, no pending
   formal waivers, W1 rejected. Dispositions stay `pending` until State-12
   execution closes T1/T2 compensation. PASS (conditional).
4. Vacuity: harnesses keep production-code asserts; cover points unchanged and
   not sole evidence. No assert(true) theater. PASS.
5. Watch item (not a failure): pre-change diff log shows a memcmp site reaching
   iteration 30. The State-12 OBL-DIFF run MUST show zero unwinding-assertion
   failures; any occurrence reverts T1 to 24 and re-runs. Recorded as residual
   risk, enforced by the formal-verifier gate (finding F2).

Formal execution (OBL-DIFF/OBL-WIRE/OBL-ID/OBL-FULL) is PENDING_FORMAL_EXECUTION
at this state by pipeline design; smoke/typecheck evidence is in
proof-writer-report.md (diff stat + gate plan). No fake evidence presented.

## STATUS: APPROVED
