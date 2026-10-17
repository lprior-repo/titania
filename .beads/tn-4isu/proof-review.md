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

## Addendum — post-split review plus Verus wiring

reviewer_skill: proof-reviewer (with kani, proof-writer, verus, verus-reviewer, formal-verifier lanes)
reviewed_artifacts: crates/titania-core/src/kani.rs (16 harnesses),
proof-obligations.planned.jsonl (OBL-DIFF-SPLIT, OBL-WIRE-SER, OBL-ID-SPLIT,
OBL-VERUS-KERNEL, OBL-VERUS-DIFF), trusted-base-ledger.jsonl (T8-T13),
evidence/kani-verify-batcha-current.log, verus toolchain smoke

## Checks

6. Batch-A verdicts: 9/10 harnesses SUCCESSFUL with exact times recorded in
   proof-evidence.md (roundtrip 43.5s with the new non-empty guard; eight
   small harnesses under 2.2s each). Raw log copied to evidence. PASS with
   the T11 capping deviation noted, not hidden.
7. valid_bounded failure triage: single failed check is the unwinding
   assertion, all functional asserts pass; remedy (unwind 34) is evidence-shaped,
   not blind. Finding PROOF-UNWIND-32-EDGE, disposition pending_evidence. PASS
   as triage, verdict stays FAILED until the rerun.
8. Assume audit: 7 textual kani::assume sites, each a bounded-input constraint
   with satisfiable domain and same-harness cover points; early returns self-fail.
   No Verus-style proof assumption exists in scope. Ledger T9 accepted. PASS.
9. Wire infeasibility: two uncapped runs killed on memory explosion with RSS
   numbers recorded; matches prior corpus timeouts; concrete compensation green.
   Finding PROOF-WIRE-INFEASIBLE awaits the owner debt decision. Documented,
   not passed. CONDITIONAL.
10. Verus wiring: toolchain smoked (1 verified, 0 errors, --no-cheating);
    obligations name production symbols with the extracted-helper parity rule;
    no specs written, no proof claims made, mandatory references queued as
    prerequisite. Ledger T12 accepted, T13 open. PASS as wiring only.
11. Governance: uncapped ad-hoc runs recorded as finding PROOF-UNCAPPED-RUNS;
    resume commands in the new obligations carry systemd-run caps. PROCESS FIX.

## STATUS: REJECTED

Three findings carry disposition pending_evidence (UNWIND-32-EDGE rerun,
WIRE-INFEASIBLE owner decision, UNCAPPED-RUNS resume discipline) and the Verus
specs are unwritten proof-debt (T13). Close-out requires: valid_bounded SUCCESS
at unwind 34, DIFF-split plus ID-split verdicts under caps, the wire debt
decision, and OBL-VERUS-KERNEL extraction design. No behavior claim above is
advanced beyond its recorded evidence.

## Addendum 2 — re-review blockers P1-P4 closed

V-VERUS-KERNEL (PASS, exit 0, raw log evidence/verus-verify-khi-kernel.log)
plus T14 (TRUSTED assume_specification contract) close P1/P2. PROOF-UNWIND-
SUFFICIENCY rewritten to a single pending_evidence disposition, V-BATCHA-PASS
exit corrected to 0 with per-harness note, V-ID-VALID clause clarified, and
PROOF-DIFF-ORACLE-CIRCULARITY added (mixed_boundary as required fix), closing
P3. The A1 Entry.path decision record lives on tn-amiw, closing P4. OBL-VERUS-
KERNEL now executes as extraction plus #[path] binding plus TRUSTED contract
(WEAK_EXTERN); the mirror-only design is superseded, not merely re-graded.
Remaining open: capped ladder verdicts (running), the wire-debt owner decision,
and OBL-VERUS-DIFF. STATUS remains REJECTED until those land.

## Addendum 3 — wire debt accepted, verus-diff deferred

Owner decision: OBL-WIRE symbolic-text debt ACCEPTED with artifact_roundtrip
5/5 plus json_roundtrip 22/22 compensation (finding disposition now
owner_approved_debt, T10 likewise). OBL-VERUS-DIFF deferred with reason and
48 green concrete tests as compensation (T15); contains/diff need closures
over Vec plus String equality, outside the verus-supported subset short of a
production rewrite. Remaining open: capped ladder verdicts only. STATUS
remains REJECTED until the ladder lands its terminal lines.

## Addendum 4 — ladder terminal without verdicts

The capped 7-harness ladder scope-terminated after 12h (exit 143) with zero
VERIFICATION lines; mixed_boundary alone consumed 21GB without closing and
the remaining six harnesses never started (V-DIFF-LADDER, FAIL_LOCAL). Six
heavyweight attempts now share one signature: String/format/serde machinery
explodes Kani formulas independent of unwind bounds, so bound changes cannot
fix it. The DIFF-split obligation resolves as explicit-unsatisfied-blocker
with the 48 green concrete baseline tests as compensation, pending the same
owner debt decision already taken for wire. No absent verdict is recorded as
a pass anywhere in the ledger.
