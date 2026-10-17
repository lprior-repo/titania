# tn-n8y6 implementation

Test-only change. Zero production source modifications (`git diff` empty;
`git status` shows only the two additions below).

## Files changed

1. `crates/titania-core/tests/tn_n8y6_contract_boundaries.rs` (new, 10 tests)
   — behavior tests through public production APIs only.
2. `.beads/tn-n8y6/` (new) — `STATE.md`, `research-notes.md`,
   `routing-ledger.jsonl`, `implementation.md`, `verification-ledger.jsonl`,
   `evidence-bundle.md`, `truth-serum-report.md`, `evidence/` raw files.

## Clause mapping (bead acceptance → test)

- "Given a valid boundary value … exact documented output is asserted":
  `lane_digest_accepts_passed_equal_to_scanned_and_preserves_all_fields`
  (7,7 + exit/findings preserved),
  `lane_digest_accepts_zero_scanned_zero_passed` (0,0),
  `receipt_period_accepts_equal_start_and_finish_with_preserved_times`
  (10,10).
- "Given a deliberately wrong boundary predicate … named mutant is killed by
  a semantic assertion": seeded defect 1 (`<=` → `<` in
  `check_passed_not_above_scanned`) killed by the 2 equality tests;
  seeded defect 2 (deleted `!` in `MutantsBaseline::diff`) killed by the 4
  baseline tests; real `cargo mutants` test-mode run reports `CaughtMutant`
  for exactly `receipt.rs:117:13 replace <= with >` and
  `mutants_baseline.rs:127:38 delete ! in MutantsBaseline::diff`
  (`evidence/mutants-outcomes.json`, `evidence/mutants-caught.txt`).
- "Given a missing boundary condition … test fails for the contract
  violation": `lane_digest_rejects_passed_one_above_scanned_with_exact_counts`
  (`PassedExceedsScanned { passed: 6, scanned: 5 }` field-exact),
  `lane_digest_deserialization_rejects_passed_above_scanned` (serde boundary,
  message names the violation),
  `receipt_period_rejects_finish_one_below_start_with_exact_times`
  (`FinishedBeforeStarted { started_at: 10, finished_at: 9 }` field-exact).
- "Only --check/--list is not kill evidence": honored — kill claims rest on
  the test-mode `CaughtMutant` outcomes plus the seeded-defect runs, never on
  `--check`/`--list` output (recorded as build inventory only).
- "Denominator and exclusions": package denominator 761 (`evidence/
  mutants-list.log`); scoped `--check` denominator 64 (32 build-viable, 32
  unviable-build); test-mode run denominator 5 (2 caught, 3 unviable —
  unviable excluded, never counted as killed).
- "cover! is non-vacuity only; proofs name bounds and production binding":
  no proof executed or claimed; Kani harnesses cited as design traceability
  only (execution owned by `tn-7bq2`).

## Production-binding

Every test calls production constructors/queries directly
(`LaneDigest::new`, `MutantsBaseline::{contains,diff,from_bypasses,empty}`,
`ReceiptPeriod::new`, `LaneDigest` serde impl). No logic copies, no shadow
types, no mocks.
