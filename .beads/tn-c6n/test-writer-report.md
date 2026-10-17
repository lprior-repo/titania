# Test-writer report — tn-c6n (State 9)

## Authored

- `crates/titania-core/tests/receipt_domain_invariants.rs` (263 lines): 14
  behavior checks through public production constructors with exact variant
  and message assertions. No mocks, no mirrors, no `is_ok()`-only assertions.
- `crates/titania-core/tests/receipt_domain_properties.rs` (141 lines): 6
  executable properties (5 proptest accept/reject equivalences + 1
  message-stability check) calling production constructors directly.
- No production file was modified: every invariant in scope is already
  enforced by its smart constructor, confirmed by the suite passing against
  unmodified production.

## Failing-first evidence (adapted: production already correct)

A temporary red control (`scratch_red_control_empty_name_is_accepted`,
asserting the opposite of enforced R1) was added, run, and removed:

- RED: `evidence/red-control.log` — 14 passed, 1 failed (exactly the control),
  exit 101. Proves the suite executes assertions and fails on violation.
- GREEN after removal: `evidence/cargo-test-invariants-s9.log` (14 passed,
  exit 0) and `evidence/cargo-test-properties-s9.log` (6 passed, exit 0).
- The control is absent from the final files (verified by name search).

Compile errors met and fixed during writing (all in test code, none in
production): proptest config placement, two temporary-borrow bindings, one
turbofish annotation. No test was weakened to satisfy the compiler.

## Proof/Refinement Coverage Matrix

| Proof ID | Bridge ID | Behavior Tests (in invariants file) | Harness (in properties file) | Run evidence |
|---|---|---|---|---|
| tn-c6n.O-P1 | tn-c6n.B-P1 | lane_name_rejects_empty_with_empty_lane_name, lane_name_rejects_nul_with_invalid_lane_name, lane_name_preserves_valid_through_accessors_and_json | prop_lane_name_decides_validity | invariants-s9 + properties-s9 logs, exit 0 |
| tn-c6n.O-P2 | tn-c6n.B-P2 | lane_digest_rejects_passed_above_scanned_constructor, lane_digest_accepts_equal_and_zero_boundaries, lane_digest_json_rejects_passed_above_scanned | prop_lane_digest_guards_counts | same |
| tn-c6n.O-P3 | tn-c6n.B-P3 | receipt_error_match_is_exhaustive_with_stable_messages | error_messages_stable_across_all_variants | same; exhaustiveness also compile-enforced |
| tn-c6n.O-P4 | tn-c6n.B-P4 | receipt_period_accepts_equal_stamps, receipt_period_rejects_finish_before_start | prop_receipt_period_orders_stamps | same |
| tn-c6n.O-P5 | tn-c6n.B-P5 | recorded_root_rejects_empty_relative_nul, recorded_root_manifest_path_joins_cargo_toml | prop_recorded_root_decides_validity | same |
| tn-c6n.O-P6 | tn-c6n.B-P6 | schema_gate_accepts_v2_envelope, envelope_rejects_non_v2_schema_versions | prop_schema_gate_rejects_other_versions | same |
| tn-c6n.O-V6 | tn-c6n.B-V6 | schema_gate_accepts_v2_envelope, envelope_rejects_non_v2_schema_versions | verification/verus/receipt_schema.rs | evidence/verus-state5.log, exit 0 |

Plus one behavior check not tied to a single obligation (exit round-trip)
guarding the `ReceiptLaneExit` wire contract used inside lane digests.

## Handoff

To test-reviewer (State 10, independent child): review behavior tests only —
exactness of assertions, absence of mocks/mirrors, frozen bridge-name
conformance, and that no test duplicates a harness. Findings with repair
guide, or approval files.
