# tn-n8y6 research notes (Gate 0)

## Files opened with exact refs

- `crates/titania-check/tests/killer_demo.rs` — existing killer-demo pattern:
  bad fixture (for-loop + `.unwrap()`) rejected with `FUNC_LOOPS_FOR` +
  `CLIPPY_UNWRAP_USED` findings (`bad_fixture_rejects_with_code_findings`,
  lines 169–218); repaired fixture passes with v1 receipt
  (`repaired_fixture_passes_with_receipt`, lines 376–390). Consumer-visible
  regression proving a control without a second convention: YES — this file.
- `crates/titania-lanes/src/run_lane_mutants/command.rs` — real mutation
  execution shape is locked: single
  `cargo mutants --no-shuffle --output <dir> --workspace`, optionally wrapped
  in `systemd-run` cgroup caps (`build_cargo_mutants_command`, lines 37–46).
  Full-workspace run; NOT re-run by this bead (owned by `tn-7bq2`).
- `crates/titania-core/src/kani.rs` — harnesses bind production constructors:
  `lane_digest_rejects_passed_greater_than_scanned` (lines 30–50),
  `lane_digest_accepts_passed_not_greater_than_scanned` (lines 53–77),
  `kani_mutants_baseline_diff_zero_neg` (lines 371+, survivor-superset with
  `cover!` non-vacuity only). Harnesses are NOT behavior tests (go-skill gate
  `harness_not_behavior_test`); executable tests below mirror the same
  production APIs.
- `crates/titania-core/src/mutants_baseline.rs` — `contains` (lines 117–119,
  `entries.iter().any(entry_covers)`), `diff` (lines 126–128, filter-not-contains
  over survivor refs), `parse_str` (lines 100–109). Expiry boundary:
  `entry_covers` treats `expires_on_unix == now_unix` as covering (proven by
  existing `boundary_timestamp_is_inclusive` test).
- `crates/titania-core/src/receipt.rs` — `LaneDigest::new` rejects
  `passed > scanned` (lines 70–79 via `check_passed_not_above_scanned`,
  lines 116–118); custom `Deserialize` re-validates (lines 57–63).
  `ReceiptPeriod::new` accepts `finished_at >= started_at` (lines 132–136).
- `crates/titania-core/src/receipt/lane_name.rs` — `LaneName` serializes as the
  raw validated string (lines 52–56); wire form is the plain name.
- `crates/titania-core/src/proof_id.rs` — `MutantId::new(package, rel_path,
  line, col, operator)` (lines 238–247); `parse` of canonical
  `<pkg>::<rel-path>:<line>:<col>:<operator>` (lines 275+).
- `crates/titania-core/src/error.rs` — `ReceiptError::PassedExceedsScanned
  { passed, scanned }` message
  `lane passed count {passed} exceeds scanned count {scanned}` (lines 340+);
  `FinishedBeforeStarted { started_at, finished_at }`.
- `.beads/tn-6r36/contract.md` + `boundary-map.md` + `traceability-matrix.jsonl`
  + `proof-seeds.jsonl` — D1 (boundedness = named bound + evidence, syntax
  irrelevant), D2 (recovery = typed-failure consumption + live approval),
  D5 (unproven obligations block). This bead does not implement the future
  classifier (owned by `tn-ebn9`/`tn-8obr`/`tn-pfp1`/`tn-ce7x`); it supplies
  executable boundary evidence for receipt/baseline value contracts that the
  classifier and gates depend on.

## Existing-test gap analysis (why the new file is not duplication)

- `receipt_invariants.rs:181` rejects `(1,2)` with `matches!`; no test asserts
  the accepted equality boundary preserves all five getters, nor the `(0,0)`
  boundary, nor rejection with one-above values and field-exact payload, nor
  direct `LaneDigest` (non-envelope) deserialization rejection.
- `v15_mutants_baseline_diff.rs` asserts diff len/content; no test asserts
  `contains()` directly, nor diff reference identity (`ptr::eq` into the
  survivor slice — the Kani-proven "no identifier outside the survivor slice"
  invariant), nor empty-baseline `contains`.
- `v15_baseline_expiry.rs` covers expiry via `diff`; no test asserts
  `contains() == false` for the expired entry alongside the retained diff.
- `receipt_invariants.rs:191` rejects `(12,10)`; no test asserts the
  `(t,t)` equality boundary is accepted with preserved getters, nor the
  adjacent `(10,9)` rejection with field-exact payload.

## Duplicate/overlap ruling

- `tn-7bq2` (Full-scope readiness) owns real full-workspace mutation runs and
  kani execution — explicitly out of scope here per parent direction.
- `tn-9acl` (seeded-defect rejection through Moon gates) is blocked BY this
  bead; this bead's named-defect kill demo is local `cargo test` evidence, not
  a Moon gate claim — no overlap.
