# Proof coverage matrix — tn-c6n

| Requirement | Seed | Lane | Obligation | Target | Command |
|---|---|---|---|---|---|
| R1 | tn-c6n.S1 | proptest | tn-c6n.O-P1 | lane_name.rs::LaneName::new | cargo test -p titania-core --test receipt_domain_properties |
| R2 | tn-c6n.S2 | proptest | tn-c6n.O-P2 | receipt.rs::LaneDigest::new | cargo test -p titania-core --test receipt_domain_properties |
| R3 | tn-c6n.S3 | proptest | tn-c6n.O-P3 | error.rs::ReceiptError | cargo test -p titania-core --test receipt_domain_properties + receipt_domain_invariants |
| R4 | tn-c6n.S4 | proptest | tn-c6n.O-P4 | receipt.rs::ReceiptPeriod::new | cargo test -p titania-core --test receipt_domain_properties |
| R5 | tn-c6n.S5 | proptest | tn-c6n.O-P5 | target_root.rs::RecordedTargetRoot::new | cargo test -p titania-core --test receipt_domain_properties |
| R6 | tn-c6n.S6 | proptest | tn-c6n.O-P6 | schema.rs::RECEIPT_ENVELOPE_SCHEMA_VERSION | cargo test -p titania-core --test receipt_domain_invariants |
| R6 | tn-c6n.S6 | verus | tn-c6n.O-V6 | verification/verus/receipt_schema.rs | verus verification/verus/receipt_schema.rs |

All other lanes are ruled out with evidence (see lane matrix). Every
behavior-affecting obligation has a planned bridge (State 7) to production
source refs, behavior tests, and separate harness refs.
