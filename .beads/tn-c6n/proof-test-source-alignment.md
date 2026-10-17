# Proof/test/source alignment — tn-c6n (State 12)

| Requirement | Proof ID | Refinement ID | Source Refs | Behavior Test Refs | Refinement Harness Refs | Commands Run | Ledger Result | Status |
|---|---|---|---|---|---|---|---|---|
| tn-c6n-REQ-LANE-NAME | tn-c6n.O-P1 | tn-c6n.B-P1 | crates/titania-core/src/receipt/lane_name.rs::LaneName::new | 3 lane-name tests | prop_lane_name_decides_validity | cargo test properties | PASS | closed |
| tn-c6n-REQ-LANE-CONSISTENCY | tn-c6n.O-P2 | tn-c6n.B-P2 | crates/titania-core/src/receipt.rs::LaneDigest::new | 3 lane-digest tests | prop_lane_digest_guards_counts | cargo test properties | PASS | closed |
| tn-c6n-REQ-ERROR-EXHAUSTIVENESS | tn-c6n.O-P3 | tn-c6n.B-P3 | crates/titania-core/src/error.rs::ReceiptError | exhaustive-match test | error_messages_stable_across_all_variants | cargo test properties | PASS | closed |
| tn-c6n-REQ-PERIOD-ORDERING | tn-c6n.O-P4 | tn-c6n.B-P4 | crates/titania-core/src/receipt.rs::ReceiptPeriod::new | 2 period tests | prop_receipt_period_orders_stamps | cargo test properties | PASS | closed |
| tn-c6n-REQ-TARGET-ROOT | tn-c6n.O-P5 | tn-c6n.B-P5 | crates/titania-core/src/receipt/target_root.rs::RecordedTargetRoot::new | 2 root tests | prop_recorded_root_decides_validity | cargo test properties | PASS | closed |
| tn-c6n-REQ-SCHEMA-GATE | tn-c6n.O-P6 | tn-c6n.B-P6 | crates/titania-core/src/receipt/schema.rs::RECEIPT_ENVELOPE_SCHEMA_VERSION | 2 schema tests | prop_schema_gate_rejects_other_versions | cargo test invariants | PASS | closed |
| tn-c6n-REQ-SCHEMA-GATE | tn-c6n.O-V6 | tn-c6n.B-V6 | verification/verus/receipt_schema.rs::schema_accepts_current | 2 schema tests | verification/verus/receipt_schema.rs | verus binding file | PASS | closed |
