# Test Plan: tn-c6n receipt domain invariants

## Summary

- Behaviors identified: 13 (all through public production constructors).
- Trophy allocation: 14 test fns covering 13 behaviors, all integration
  (test targets calling real constructors with real JSON text) / 0 isolated
  unit (constructors are the units; no mocks exist anywhere) / 0 e2e (no CLI
  surface in scope) / static via compile-time exhaustiveness + strict clippy.
  Deviation rationale: the receipt core is a pure validation core; every
  check is an integration check against the real constructor. Static
  exhaustiveness (R3) substitutes for the static layer.
- Proptest invariants: 5 plus 1 plain stability check. Fuzz targets: 0 (JSON boundary covered by hostile
  round-trip tests; fuzzing owned elsewhere). Kani harnesses: 0 (owned by
  `tn-7bq2`).
- Naming: `fn [subject]_[outcome]_when_[condition]` is approximated by the
  frozen bridge names (e.g. `lane_name_rejects_empty_with_empty_lane_name`).

## 1. Behavior Inventory

1. LaneName rejects empty input when constructed from empty string.
2. LaneName rejects NUL-bearing input when constructed or loaded from JSON.
3. LaneName preserves valid input when accessed, displayed, or round-tripped.
4. LaneDigest rejects passed-above-scanned when constructed or loaded.
5. LaneDigest accepts boundary pairs when passed equals scanned or both zero.
6. ReceiptError renders every variant when matched exhaustively.
7. ReceiptPeriod accepts equal stamps when start equals finish.
8. ReceiptPeriod rejects reversed stamps when finish precedes start.
9. RecordedTargetRoot rejects empty/relative/NUL paths when constructed.
10. RecordedTargetRoot joins manifest path when asked for Cargo.toml location.
11. ReceiptLaneExit round-trips every variant when serialized and loaded.
12. Schema gate loads version 2 when the envelope is current.
13. Schema gate rejects other versions when loaded from JSON.

## 2. Trophy Allocation

All 14 test fns (13 behaviors; digest rejection is split constructor + JSON) are integration tests in
`crates/titania-core/tests/receipt_domain_invariants.rs` (real constructors,
real JSON text, exact variant assertions; no `is_ok()`-only assertions).

## 3. BDD Scenarios

### lane_name_rejects_empty_with_empty_lane_name

Given: an empty lane name string.
When: `LaneName::new("")` runs.
Then: `Err(ReceiptError::EmptyLaneName)` (exact match, plus message text).

### lane_name_rejects_nul_with_invalid_lane_name

Given: `"a\0b"` directly and as JSON `"lane"` text.
When: constructed / loaded.
Then: `Err(ReceiptError::InvalidLaneName)` on both paths.

### lane_name_preserves_valid_through_accessors_and_json

Given: `"fmt"`.
When: constructed, then `as_str`/`Display`/JSON round-trip.
Then: identical text on every path.

### lane_digest_rejects_passed_above_scanned_constructor

Given: `(passed=2, scanned=1)`.
When: `LaneDigest::new` runs.
Then: `Err(PassedExceedsScanned { passed: 2, scanned: 1 })` exactly.

### lane_digest_accepts_equal_and_zero_boundaries

Given: `(3,3)` and `(0,0)`.
When: constructed.
Then: accessors return the exact counts.

### lane_digest_json_rejects_passed_above_scanned

Given: valid envelope JSON with lane counts flipped to `(2,1)`.
When: loaded as `ReceiptEnvelope`.
Then: rejection naming passed 2 / scanned 1.

### receipt_error_match_is_exhaustive_with_stable_messages

Given: one reachable value per `ReceiptError` variant (built via production
constructors, plus the v1 empty-lane path).
When: matched exhaustively and displayed.
Then: all nine messages equal the documented strings; adding a variant
breaks compilation (no wildcard arm).

### receipt_period_accepts_equal_stamps / receipt_period_rejects_finish_before_start

Given: `(12,12)` and `(12,10)`.
When: `ReceiptPeriod::new` runs.
Then: `Ok` preserving both stamps / `Err(FinishedBeforeStarted {
started_at: 12, finished_at: 10 })`.

### recorded_root_rejects_empty_relative_nul

Given: `""`, `"relative/path"`, `"\/abs\0lute"`.
When: `RecordedTargetRoot::new` runs.
Then: `TargetRootEmpty` / `TargetRootNonAbsolute` / `TargetRootContainsNul`.

### recorded_root_manifest_path_joins_cargo_toml

Given: `"/proj"`.
When: `manifest_path()` runs.
Then: `"/proj/Cargo.toml"`.

### receipt_exit_round_trips_all_variants

Given: all four `ReceiptLaneExit` variants inside lane digests.
When: JSON round-tripped.
Then: identical variant back (snake_case wire names asserted once).

### schema_gate_accepts_v2_envelope / envelope_rejects_non_v2_schema_versions

Given: a valid envelope and copies stamped `0, 1, 3, u32::MAX`.
When: loaded.
Then: v2 loads; the rest fail with `UnsupportedSchemaVersion(v)` exactly.

## 4. Proptest Invariants

In `crates/titania-core/tests/receipt_domain_properties.rs`:

- `prop_lane_name_decides_validity`: `is_ok() == (!empty && !NUL)`; success
  preserves input byte for byte.
- `prop_lane_digest_guards_counts`: `is_ok() == (passed <= scanned)`.
- `error_messages_stable_across_all_variants` (plain check, not proptest:
  nine fixed strings carry no input class): nine messages non-empty and
  pairwise distinct.
- `prop_receipt_period_orders_stamps`: `is_ok() == (finished >= started)`.
- `prop_recorded_root_decides_validity`: `is_ok() == (!empty && absolute &&
  !NUL)`; success preserves the path.
- `prop_schema_gate_rejects_other_versions`: any `v != 2` fails to load with
  `UnsupportedSchemaVersion(v)`; `RECEIPT_ENVELOPE_SCHEMA_VERSION == 2`.

## 5. Fuzz Targets

None in this bead. The JSON text boundary is exercised deterministically
(hostile shapes above); grammar fuzzing of the envelope remains unowned
follow-up work, not a claim.

## 6. Kani Harnesses

None in this bead (owned by `tn-7bq2`; tool absent). Rejection equivalence
is covered by full-class proptest corpora instead.

## 7. Mutation Checkpoints

- Flipping any guard (`==`→`!=`, dropping a rejection arm, widening schema
  acceptance) must be caught by the paired behavior test (threshold: all
  13 fail-closed on guard removal by construction; measured mutants owned by
  `tn-7bq2`).
- Adding a `ReceiptError` variant must break compilation of the
  exhaustive-match test.

## 8. Combinatorial Coverage Matrix

| Group | Input class | Expected | Layer |
|---|---|---|---|
| LaneName | `""` | `Err(EmptyLaneName)` | integration |
| LaneName | NUL-bearing (direct + JSON) | `Err(InvalidLaneName)` | integration |
| LaneName | valid text | exact preservation | integration |
| LaneDigest | `passed > scanned` (direct + JSON) | exact `Err` payload | integration |
| LaneDigest | `(n,n)`, `(0,0)` | accepted, exact accessors | integration |
| ReceiptError | one value per variant | 9 exact messages | integration + static |
| ReceiptPeriod | equal / reversed | `Ok` / exact `Err` | integration |
| RecordedTargetRoot | empty/relative/NUL | exact variant each | integration |
| RecordedTargetRoot | `/proj` | `/proj/Cargo.toml` | integration |
| ReceiptLaneExit | 4 variants | round-trip identity | integration |
| Schema | v2 valid | loads | integration |
| Schema | 0/1/3/MAX + arbitrary | exact `UnsupportedSchemaVersion` | integration + proptest |

## Proof/Refinement Coverage Matrix

| Proof ID | Bridge ID | Behavior Tests | Harness | Status |
|---|---|---|---|---|
| tn-c6n.O-P1 | tn-c6n.B-P1 | 3 lane-name tests | prop_lane_name_decides_validity | planned |
| tn-c6n.O-P2 | tn-c6n.B-P2 | 3 lane-digest tests | prop_lane_digest_guards_counts | planned |
| tn-c6n.O-P3 | tn-c6n.B-P3 | exhaustive-match test | error_messages_stable_across_all_variants | planned |
| tn-c6n.O-P4 | tn-c6n.B-P4 | 2 period tests | prop_receipt_period_orders_stamps | planned |
| tn-c6n.O-P5 | tn-c6n.B-P5 | 2 root tests | prop_recorded_root_decides_validity | planned |
| tn-c6n.O-P6 | tn-c6n.B-P6 | 2 schema tests | prop_schema_gate_rejects_other_versions | planned |
| tn-c6n.O-V6 | tn-c6n.B-V6 | 2 schema tests | verification/verus/receipt_schema.rs | planned |

## Open Questions

None. Public APIs are frozen; no production change is anticipated unless a
test exposes a real enforcement gap (then route to State 11 with the gap
named).
