reviewer_skill: test-reviewer
reviewer_invocation_id: tn-c6n-inv-010
writer_invocation_id: tn-c6n-inv-009

# Test-suite review — tn-c6n (State 10)

STATUS: REJECTED

## Scope

Reviewed behavior tests only (no proofs):
`crates/titania-core/tests/receipt_domain_invariants.rs` (14 tests),
`crates/titania-core/tests/receipt_domain_properties.rs` (6 checks),
`.beads/tn-c6n/test-writer-report.md` with
`evidence/red-control.log`, `evidence/cargo-test-invariants-s9.log`,
`evidence/cargo-test-properties-s9.log`.

## Fresh evidence (this review)

- `cargo test -p titania-core --test receipt_domain_invariants --test
  receipt_domain_properties` → 14 passed + 6 passed, exit 0. The suite is
  NOT stale: every assertion below was checked against current production.
- `ReceiptError` in `crates/titania-core/src/error.rs` has exactly the 9
  tested variants with the exact tested messages. Exact assertions match.
- `git status --porcelain crates/` shows only the two new test files
  untracked; no production file modified. Writer claim confirmed.
- `grep scratch_red_control crates/titania-core/tests/` → no match; the
  red control is absent from final files (only prose mention in the
  writer report). Red/green evidence is credible: red log 14 passed +
  1 failed control (exit 101); green logs 14 + 6 passed.

## Checks

- No `is_ok()`-only assertions: invariants file has zero `is_ok`; each of
  the 5 properties pairs `is_ok` equivalence with exact payload/message
  assertions. Pass (note O1 below).
- No mocks or mirror types: tests import only `titania_core`, `serde_json`,
  `tempfile`, `camino`, `proptest`; every check calls a production
  constructor (`LaneName::new`, `LaneDigest::new`, `ReceiptPeriod::new`,
  `RecordedTargetRoot::new`, `ReceiptEnvelope`/`LaneDigest` deserialization).
  Pass (note O2 below).
- All 14 behavior refs in the obligations file resolve to real test fns.
  Pass.
- No behavior/harness overlap: fixed-example behavior tests vs full-corpus
  equivalence properties are complementary, not duplicative. Pass.
- Error/edge/boundary coverage from the plan is fully present: `""`, NUL
  (direct + JSON), `(3,3)`/`(0,0)`, 9 exact messages, `(12,12)`/`(12,10)`,
  empty/relative/NUL roots, `/proj/Cargo.toml`, 4 exit variants, schema
  0/1/3/MAX plus arbitrary `u32`. Pass.
- FINDING F1 (blocking): bridge-name mismatch. Obligation tn-c6n.B-P3
  `refinement_harness_refs` names
  `receipt_domain_properties.rs::prop_receipt_error_messages_stable`,
  and the test plan repeats that name — but the built check is the plain
  `#[test] fn error_messages_stable_across_all_variants`. The frozen bridge
  ref does not resolve (5 of 6 harness refs resolve; this one does not),
  breaking automated traceability. Reject.

## Observations (non-blocking)

- O1: `prop_recorded_root_decides_validity` pins the rejection only via
  `is_ok` equivalence; the exact variants are pinned by the behavior test
  `recorded_root_rejects_empty_relative_nul`. Acceptable as paired, but a
  future strengthening could assert the variant inside the property.
- O2: `receipt_error_match_is_exhaustive_with_stable_messages` contains a
  local `match` that re-formats messages alongside asserting production
  `Display`. It still calls production constructors and asserts production
  output, so it is not a bypassing mirror — but it is duplicated oracle
  logic to keep in sync.

## Repair guide (reviewer made no test or production edits)

- Recommended (Option A): keep the built plain-`#[test]` shape — the 9
  messages are fixed strings, proptest adds no input class. Update the
  frozen ref tn-c6n.B-P3 `refinement_harness_refs` to
  `crates/titania-core/tests/receipt_domain_properties.rs::error_messages_stable_across_all_variants`
  (owner: bridge / State 7), and update the test-plan `§4` + matrix cells
  and "Proptest invariants: 6" → "5 proptest + 1 plain" (owner: planner,
  or writer with sign-off). Also correct "13 integration" → "13 behaviors
  in 14 test fns".
- Not recommended (Option B): renaming the test to `prop_…` without
  making it a real proptest property would mislead; converting it to a
  proptest without a genuine strategy adds nothing.
- Re-verify after repair: re-run both test targets (expect 14 + 6, exit 0),
  re-resolve every `behavior_test_refs` / `refinement_harness_refs` entry
  against the test files, then request State 10 re-review.
