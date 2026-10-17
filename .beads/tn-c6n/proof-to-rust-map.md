# Proof-to-Rust map — tn-c6n (State 7)

Every behavior-affecting proof obligation bridges to production source refs,
independent behavior tests, and a disjoint refinement harness ref. No
behavior test doubles as a harness; no harness copies production logic (all
harnesses call production constructors).

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|---|---|---|---|---|---|---|---|---|
| tn-c6n.O-P1 | lane-name accept/reject equivalence | true | crates/titania-core/src/receipt/lane_name.rs::LaneName::new | crates/titania-core/tests/receipt_domain_invariants.rs::lane_name_rejects_empty_with_empty_lane_name, ::lane_name_rejects_nul_with_invalid_lane_name, ::lane_name_preserves_valid_through_accessors_and_json | crates/titania-core/tests/receipt_domain_properties.rs::prop_lane_name_decides_validity | proptest | cargo test -p titania-core --test receipt_domain_properties | 7 |
| tn-c6n.O-P2 | lane-digest passed<=scanned equivalence | true | crates/titania-core/src/receipt.rs::LaneDigest::new | crates/titania-core/tests/receipt_domain_invariants.rs::lane_digest_rejects_passed_above_scanned_constructor, ::lane_digest_accepts_equal_and_zero_boundaries, ::lane_digest_json_rejects_passed_above_scanned | crates/titania-core/tests/receipt_domain_properties.rs::prop_lane_digest_guards_counts | proptest | cargo test -p titania-core --test receipt_domain_properties | 7 |
| tn-c6n.O-P3 | nine-variant reachability + message stability | true | crates/titania-core/src/error.rs::ReceiptError | crates/titania-core/tests/receipt_domain_invariants.rs::receipt_error_match_is_exhaustive_with_stable_messages | crates/titania-core/tests/receipt_domain_properties.rs::error_messages_stable_across_all_variants | proptest | cargo test -p titania-core --test receipt_domain_properties | 7 |
| tn-c6n.O-P4 | period ordering equivalence | true | crates/titania-core/src/receipt.rs::ReceiptPeriod::new | crates/titania-core/tests/receipt_domain_invariants.rs::receipt_period_accepts_equal_stamps, ::receipt_period_rejects_finish_before_start | crates/titania-core/tests/receipt_domain_properties.rs::prop_receipt_period_orders_stamps | proptest | cargo test -p titania-core --test receipt_domain_properties | 7 |
| tn-c6n.O-P5 | target-root validity equivalence | true | crates/titania-core/src/receipt/target_root.rs::RecordedTargetRoot::new | crates/titania-core/tests/receipt_domain_invariants.rs::recorded_root_rejects_empty_relative_nul, ::recorded_root_manifest_path_joins_cargo_toml | crates/titania-core/tests/receipt_domain_properties.rs::prop_recorded_root_decides_validity | proptest | cargo test -p titania-core --test receipt_domain_properties | 7 |
| tn-c6n.O-P6 | schema gate version behavior | true | crates/titania-core/src/receipt/schema.rs::RECEIPT_ENVELOPE_SCHEMA_VERSION | crates/titania-core/tests/receipt_domain_invariants.rs::schema_gate_accepts_v2_envelope, ::envelope_rejects_non_v2_schema_versions | crates/titania-core/tests/receipt_domain_properties.rs::prop_schema_gate_rejects_other_versions | proptest | cargo test -p titania-core --test receipt_domain_invariants | 7 |
| tn-c6n.O-V6 | binding file verifies clean (hygiene) | true | verification/verus/receipt_schema.rs::schema_accepts_current | crates/titania-core/tests/receipt_domain_invariants.rs::schema_gate_accepts_v2_envelope, ::envelope_rejects_non_v2_schema_versions | verification/verus/receipt_schema.rs | verus | verus verification/verus/receipt_schema.rs | 7 |

Bridge IDs `tn-c6n.B-P1`…`tn-c6n.B-P6`, `tn-c6n.B-V6` live in
`rust-refinement-obligations.jsonl`. All rows `mapping_status: planned`
until State-12 execution flips them to `verified` with raw logs.
