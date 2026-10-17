# Implementation — tn-c6n (State 11, holzman-rust)

## Change summary

Tests-only delivery. Zero production files modified (`git status` shows
only the two new test files untracked under `crates/`; verified by the
State-10 re-review). All six retained invariants were already enforced by
their smart constructors; enforcement was confirmed, not added.

## Files added

- `crates/titania-core/tests/receipt_domain_invariants.rs` (265 lines): 14
  behavior checks, exact variant/message assertions, no mocks, no mirrors.
- `crates/titania-core/tests/receipt_domain_properties.rs` (140 lines): 5
  proptest accept/reject equivalences + 1 stability check over production
  constructors.

Both files stay under the 300-line ceiling. No `unsafe`, no new
dependencies (proptest/tempfile/serde_json/camino already in the crate's
dev/regular deps), no panic-path production code touched.

## Source Coverage Matrix

| Production source | Invariant | Behavior tests | Harness | Result |
|---|---|---|---|---|
| receipt/lane_name.rs::LaneName::new | R1 validity | 3 lane-name tests | prop_lane_name_decides_validity | 14+6 green |
| receipt.rs::LaneDigest::new | R2 consistency | 3 lane-digest tests | prop_lane_digest_guards_counts | 14+6 green |
| error.rs::ReceiptError | R3 exhaustiveness | exhaustive-match test | error_messages_stable_across_all_variants | 14+6 green |
| receipt.rs::ReceiptPeriod::new | R4 ordering | 2 period tests | prop_receipt_period_orders_stamps | 14+6 green |
| receipt/target_root.rs::RecordedTargetRoot::new | R5 validity | 2 root tests | prop_recorded_root_decides_validity | 14+6 green |
| receipt/schema.rs::RECEIPT_ENVELOPE_SCHEMA_VERSION | R6 gate | 2 schema tests | prop_schema_gate_rejects_other_versions + verus binding | 14+6 green, verus 3/0 |

## Rust safety evidence

- Production diff: empty. Holzman zero-panic discipline preserved by
  construction (nothing added to production).
- Test code compiles under `cargo check --workspace --all-targets
  --all-features` with workspace lints; strict source-only clippy is
  unaffected (no source targets changed).
- `cargo test -p titania-core --test receipt_domain_invariants` → 14
  passed, exit 0. `--test receipt_domain_properties` → 6 passed, exit 0.
  Pre-existing `receipt_invariants` (9 tests) untouched and green.

## Residuals

- Formal bounded-proof (kani) and refinement (flux) lanes remain ruled out
  with evidence; ownership `tn-7bq2`. Single-operator review debt carries
  to landing. No commit/push/sync performed.
