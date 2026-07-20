# Proof-to-Implementation Input

> Handoff to State 7 (proof-to-implementation). Notes the source anchors
> each proof obligation will materialize to, the expected behavior-test
> refs, and the refinement harness refs.

## Source anchor map

| Req | Obligation ID | Source anchor (production code) |
|-----|---------------|---------------------------------|
| v15-REQ-LANE-ROUNDTRIP | v15-OBL-P1-LANE-ROUNDTRIP | `crates/titania-core/src/lane.rs::{impl Lane, FromStr}` |
| v15-REQ-GATESCOPE-ROUNDTRIP | v15-OBL-P2-GATESCOPE-ROUNDTRIP | `crates/titania-core/src/gate_scope.rs::FromStr` |
| v15-REQ-KANI-HARNESS-ID | v15-OBL-P3-KANI-ID-PROPTEST / -KANI | `crates/titania-core/src/proof_id.rs::KaniHarnessId::new` |
| v15-REQ-KANI-HARNESS-SERDE | v15-OBL-P4-KANI-ID-SERDE | `crates/titania-core/src/proof_id.rs::KaniHarnessId` |
| v15-REQ-MUTANT-ID | v15-OBL-P5-MUTANT-ID-PROPTEST | `crates/titania-core/src/proof_id.rs::MutantId::new` |
| v15-REQ-MUTANT-ID-OPERATOR | v15-OBL-V1-MUTANT-ID-VERUS | `crates/titania-core/src/proof_id.rs::spec_mutant_id_closed_set` (Verus spec/proof) |
| v15-REQ-MUTANTS-BASELINE-LOAD | v15-OBL-P6-MUTANTS-LOAD | `crates/titania-core/src/mutants_baseline.rs::load` |
| v15-REQ-MUTANTS-DIFF | v15-OBL-P7-MUTANTS-DIFF-PROPTEST | `crates/titania-core/src/mutants_baseline.rs::diff` |
| v15-REQ-MUTANTS-DIFF-ZERO-NEG | v15-OBL-K2-MUTANTS-DIFF-KANI | `crates/titania-core/src/kani.rs::kani_mutants_baseline_diff_zero_neg` |
| v15-REQ-MUTANTS-EXPIRY | v15-OBL-P8-MUTANTS-EXPIRY | `crates/titania-core/src/mutants_baseline.rs::MutantBaselineEntry::is_expired` |
| v15-REQ-SKIP-REASON-TOOL | v15-OBL-P9-SKIP-REASON-TOOL | `crates/titania-core/src/outcome.rs::SkipReason::ToolUnavailable` |
| v15-REQ-LANE-NAME | v15-OBL-P10-LANE-NAME | `crates/titania-core/src/lane.rs::Lane::name` |
| v15-REQ-LANE-SERDE | v15-OBL-P11-LANE-SERDE | `crates/titania-core/src/lane.rs::{Lane, Serialize, Deserialize}` |
| v15-REQ-KANI-LANE-NAME | v15-OBL-K1-KANI-NAME-KANI | `crates/titania-core/src/kani.rs::kani_kani_lane_name_roundtrip` |
| v15-REQ-ATOMIC-BASELINE-LOAD | v15-OBL-L1-ATOMIC-LOAD-LOOM | `crates/titania-lanes/src/artifact_writer.rs::atomic_write` |
| v15-REQ-INVENTORY-PARSE | v15-OBL-F1-FUZZ | `crates/titania-core/src/kani_inventory.rs::parse_inventory` |
| v15-REQ-OUTCOMES-PARSE | v15-OBL-F2-FUZZ | `crates/titania-core/src/mutants_outcomes.rs::parse_outcomes` |

## Behavior-test refs (counterpart tests)

| Req | Test name |
|-----|-----------|
| v15-REQ-LANE-ROUNDTRIP | `tests/unit_tests.rs::lane_from_str_to_string_round_trip_all` (existing v1 harness extended) |
| v15-REQ-GATESCOPE-ROUNDTRIP | new `tests/v15_gate_scope_roundtrip.rs` |
| v15-REQ-KANI-HARNESS-ID | new `tests/v15_kani_harness_id.rs` |
| v15-REQ-KANI-HARNESS-SERDE | new `tests/v15_kani_harness_id_serde.rs` |
| v15-REQ-MUTANT-ID | new `tests/v15_mutant_id.rs` |
| v15-REQ-MUTANTS-BASELINE-LOAD | new `tests/v15_mutants_baseline_load.rs` |
| v15-REQ-MUTANTS-DIFF | new `tests/v15_mutants_baseline_diff.rs` |
| v15-REQ-MUTANTS-EXPIRY | new `tests/v15_baseline_expiry.rs` |
| v15-REQ-SKIP-REASON-TOOL | new `tests/v15_skip_reason_tool_unavailable.rs` |
| v15-REQ-LANE-NAME | new `tests/v15_lane_name.rs` |
| v15-REQ-LANE-SERDE | new `tests/v15_lane_serde_roundtrip.rs` |

## Refinement harness refs (Kani/Verus/Loom/fuzz)

- Kani harnesses live in `crates/titania-core/src/kani.rs` (existing
  file extended). `kani::kani_kani_harness_id_bounded`,
  `kani::kani_kani_lane_name_roundtrip`,
  `kani::kani_mutants_baseline_diff_zero_neg`.
- Verus spec/proof lives in `crates/titania-core/src/proof_id.rs` as
  `spec::spec_mutant_id_closed_set`; production-binding via
  `#[verifier::assume_specification[ MutantId::new ]]` is per Verus
  v10+ requirement (production-bound).
- Loom test lives in
  `crates/titania-lanes/tests/v15_atomic_baseline.rs`.
- fuzz harnesses under `fuzz/fuzz_targets/{fuzz_parse_inventory,fuzz_parse_outcomes}.rs`.

## No bridge rows needed

No production-logic copies; every proof targets a production function
either directly (Rust: `cargo test`) or through a contract spec
(Verus / Loom) that targets production semantics.

## Lane-by-lane mapping status

- All 18 obligations: `mapping_status: planned` (State 7) → will become
  `materialized` once proof-writer runs (State 5 → 7 → 12).
