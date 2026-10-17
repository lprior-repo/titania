# tn-n8y6 evidence bundle (requirement → source → test → raw command)

## 1. Valid boundaries accepted with exact preserved output

- Requirement: equality/zero/time boundaries accepted.
- Source: `crates/titania-core/src/receipt.rs:70-79` (`LaneDigest::new`),
  `:132-136` (`ReceiptPeriod::new`).
- Tests: `tn_n8y6_contract_boundaries.rs`: `lane_digest_accepts_…`,
  `receipt_period_accepts_…` (getters asserted field-by-field).
- Raw: `cargo test -p titania-core --all-features` exit 0
  (`evidence/coretest-summary.txt`, 29 binaries all ok, new file 10/10).

## 2. Off-by-one violations rejected with exact typed payloads

- Requirement: contract-violation failures name exact counts/times.
- Source: `receipt.rs:116-118`, `error.rs` (`PassedExceedsScanned`,
  `FinishedBeforeStarted`).
- Tests: `lane_digest_rejects_passed_one_above_scanned_with_exact_counts`,
  `lane_digest_deserialization_rejects_passed_above_scanned`,
  `receipt_period_rejects_finish_one_below_start_with_exact_times`.
- Raw: same green suite; negative paths assert `Err` payloads, never
  `is_err()`-only.

## 3. Baseline covered/uncovered/expiry transitions

- Requirement: survivor-superset transition through production API.
- Source: `crates/titania-core/src/mutants_baseline.rs:117-128`.
- Tests: `baseline_contains_covered_survivor_and_diff_excludes_it`,
  `baseline_diff_returns_identical_reference_for_uncovered_survivor`
  (`ptr::eq` identity — mirrors the Kani "no id outside the survivor slice"
  invariant executably), `baseline_expired_entry_covers_nothing…`,
  `baseline_empty_contains_no_survivor`.
- Raw: same green suite.

## 4. Named defective implementations killed (behavior + tool)

- Defect A `receipt.rs:117 <= → <`: killed by 2 tests (observed stderr:
  "lane passed count 7 exceeds scanned count 7", "…count 0 exceeds…count 0").
- Defect B `mutants_baseline.rs:127 deleted !`: killed by 4 tests.
- Both temp patches reverted; `git diff` empty (production untouched).
- Real tool: `cargo mutants` test-mode, 5 matched / 2 `CaughtMutant` —
  exactly `receipt.rs:117:13 replace <= with >` and
  `mutants_baseline.rs:127:38 delete ! in MutantsBaseline::diff`;
  3 unviable excluded, never counted (`evidence/mutants-outcomes.json`,
  `mutants-caught.txt`, `mutants-unviable.txt`).

## 5. Honest scope accounting (what was NOT claimed)

- `--list` (761) and `--check` (64 scoped: 32 viable-build / 32
  unviable-build) are build inventory only — zero kill weight.
- Full-package `--check` blocked by /tmp quota (environmental, recorded).
- `moon ci`, kani, full-workspace mutants: not run per parent direction
  (owned by `tn-4isu`/`tn-7bq2`).
- `tools/go-skill-v9-validate`: absent — no validator pass claimed.
- Specialist dispatch: single-agent inline with `functional-rust` and
  `test-writer` bodies loaded; degraded provenance recorded in
  `routing-ledger.jsonl` (no silently-substituted approvals).
