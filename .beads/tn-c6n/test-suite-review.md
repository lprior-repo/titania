reviewer_skill: test-reviewer
reviewer_invocation_id: tn-c6n-inv-010b
writer_invocation_id: tn-c6n-inv-010r

# Test-suite review — tn-c6n (State 10 re-review)

STATUS: APPROVED

## Scope

Re-review of the behavior suite after repair invocation tn-c6n-inv-010r
(Option A from the round-1 rejection): `crates/titania-core/tests/
receipt_domain_invariants.rs` (14 tests), `crates/titania-core/tests/
receipt_domain_properties.rs` (6 checks),
`.beads/tn-c6n/test-writer-report.md`, and
`.beads/tn-c6n/rust-refinement-obligations.jsonl`. Round-1 suite review
(tn-c6n-inv-010, archived as `test-suite-review-round1-rejected.md`) gave
a rejection verdict solely on finding F1 (bridge ref
`prop_receipt_error_messages_stable` did not resolve to a built check).

## Fresh evidence (this re-review)

- `TMPDIR=/home/lewis/.cache/muse-tmp cargo test -p titania-core --test
  receipt_domain_invariants --test receipt_domain_properties` → 14 passed
  + 6 passed, exit 0. Saved to
  `.beads/tn-c6n/evidence/cargo-test-rereview-10b.log`. The suite is NOT
  stale: every assertion below was checked against current production.
- `git status --porcelain crates/` shows only the two untracked test
  files (`receipt_domain_invariants.rs`, `receipt_domain_properties.rs`);
  the repair modified no test or production file. Writer claim confirmed.
- `grep scratch_red_control crates/titania-core/tests/` expectation
  unchanged from round 1: the red control is absent from final files.

## Ref resolution (every entry checked)

Behavior refs (`behavior_test_refs`), all in `receipt_domain_invariants.rs`:

- B-P1: `lane_name_rejects_empty_with_empty_lane_name`,
  `lane_name_rejects_nul_with_invalid_lane_name`,
  `lane_name_preserves_valid_through_accessors_and_json` — all resolve.
- B-P2: `lane_digest_rejects_passed_above_scanned_constructor`,
  `lane_digest_accepts_equal_and_zero_boundaries`,
  `lane_digest_json_rejects_passed_above_scanned` — all resolve.
- B-P3: `receipt_error_match_is_exhaustive_with_stable_messages` — resolves.
- B-P4: `receipt_period_accepts_equal_stamps`,
  `receipt_period_rejects_finish_before_start` — both resolve.
- B-P5: `recorded_root_rejects_empty_relative_nul`,
  `recorded_root_manifest_path_joins_cargo_toml` — both resolve.
- B-P6 / B-V6: `schema_gate_accepts_v2_envelope`,
  `envelope_rejects_non_v2_schema_versions` — both resolve.

Harness refs (`refinement_harness_refs`):

- `prop_lane_name_decides_validity`,
  `prop_lane_digest_guards_counts`,
  `prop_receipt_period_orders_stamps`,
  `prop_recorded_root_decides_validity`,
  `prop_schema_gate_rejects_other_versions` — all resolve in
  `receipt_domain_properties.rs`.
- F1 resolved: B-P3 now names
  `receipt_domain_properties.rs::error_messages_stable_across_all_variants`,
  which resolves to the built plain `#[test] fn` at line 124. The stale
  `prop_receipt_error_messages_stable` occurs zero times in the
  obligations file and the test plan. 7 of 7 harness refs resolve.
- B-V6 names `verification/verus/receipt_schema.rs`, which exists and
  defines `schema_accepts_current` (line 13).

## Checks

- No `is_ok()`-only assertions: invariants file has zero `is_ok`; each of
  the 5 properties pairs `is_ok` equivalence with exact payload/message
  assertions. Pass.
- No mocks or mirror types: tests import only `titania_core`,
  `serde_json`, `tempfile`, `camino`, `proptest`; every check calls a
  production constructor (`LaneName::new`, `LaneDigest::new`,
  `ReceiptPeriod::new`, `RecordedTargetRoot::new`,
  `ReceiptEnvelope`/`LaneDigest` deserialization). Pass.
- All 14 behavior refs and all 7 harness refs resolve to real fns/files.
  Pass — F1 closed.
- No behavior/harness overlap: fixed-example behavior tests vs full-corpus
  equivalence properties are complementary, not duplicative. Pass.
- Error/edge/boundary coverage from the plan is fully present: `""`, NUL
  (direct + JSON), `(3,3)`/`(0,0)`, 9 exact messages, `(12,12)`/`(12,10)`,
  empty/relative/NUL roots, `/proj/Cargo.toml`, 4 exit variants, schema
  0/1/3/MAX plus arbitrary `u32`. Pass.

## Observations (non-blocking, carried from round 1)

- O1: `prop_recorded_root_decides_validity` pins the rejection only via
  `is_ok` equivalence; exact variants are pinned by the paired behavior
  test. Acceptable as paired.
- O2: the exhaustive-match test contains a local `match` re-formatting
  messages alongside asserting production `Display`. It calls production
  constructors and asserts production output, so it is not a bypassing
  mirror — but it is duplicated oracle logic to keep in sync.

No repair guide: nothing to repair. The suite is approved.
