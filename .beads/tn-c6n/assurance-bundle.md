# Assurance bundle — tn-c6n (State 14, evidence-packaging)

## Requirement → source → test/proof → raw command

- R1 lane-name validity → lane_name.rs::LaneName::new → 3 behavior tests +
  prop_lane_name_decides_validity → cargo-test logs (14/6, exit 0).
- R2 lane-digest consistency → receipt.rs::LaneDigest::new → 3 behavior
  tests + prop_lane_digest_guards_counts → same logs.
- R3 error exhaustiveness → error.rs::ReceiptError → exhaustive-match test
  (compile-enforced) + stability check → same logs.
- R4 period ordering → receipt.rs::ReceiptPeriod::new → 2 behavior tests +
  prop_receipt_period_orders_stamps → same logs.
- R5 target-root validity → target_root.rs::RecordedTargetRoot::new → 2
  behavior tests + prop_recorded_root_decides_validity → same logs.
- R6 schema gate → schema.rs const + envelope entry points → 2 behavior
  tests + version property → invariants log (14, exit 0); plus pinned verus
  binding run (3 verified, 0 errors, exit 0) classified as binding hygiene.

## Complete artifact inventory (`.beads/tn-c6n/`)

Contract (9): domain-model, type-contracts, workflow-model, error-taxonomy,
boundary-map, hazard-analysis, contract, proof-seeds, traceability-matrix.
Plan (9): proof-strategy, lane-matrix, lane-decisions (24 rows), lane-review
(24 accepted), coverage-matrix, obligations (7), trusted-base-plan,
waiver-candidates (no-waiver record), bridge-input. Proof (3 + TBL + logs):
proof-evidence, proof-writer-report, trusted-base-ledger (T1, accepted).
Reviews: plan (APPROVED after 1 repair), proof (APPROVED), bridge
(APPROVED), test plan + suite (APPROVED after 1 repair), black-hat
(APPROVED). Tests: 2 new files (14 + 6 green), red-control log, s9/s12
logs, full-crate log (30 binaries ok). Gates: fmt/check/clippy/doc/verus
logs, all exit 0. Closure: alignment md+jsonl, 14-row verification ledger
(all PASS/closed), machine-gate + regression + formal + refinement reports.

## Independent review provenance (ledgered)

4b plan (reject→repair→approve), 6 proof (approve), 7 bridge (approve),
10 tests (reject→repair→approve), 13 black-hat (approve), 14 truth-serum
(this state). Every review runs under a distinct invocation ID; no
self-approval anywhere (validator-enforced).

## Residuals (not closed, explicitly owned)

- kani bounded-proof lane: ruled out with evidence; execution ownership
  `tn-7bq2`; tool absent on host.
- flux refinement lane: ruled out with evidence; no annotations exist.
- Full `moon ci`: out of scope per delegation (pre-existing lanes own it).
- Single-operator delivery: all reviews by delegated subagents of one
  operator; no second human reviewer.
- No commit/push/sync performed or authorized (conservative default).

## Decision input

If truth-serum approves, `final-evidence-decision.md` records APPROVED and
the bead may close with the residuals above. If it rejects, the bead stays
open on its repair guide.
