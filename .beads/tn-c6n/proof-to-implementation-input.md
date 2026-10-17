# Proof-to-implementation input — tn-c6n (planner → State 7)

Bridge each behavior-affecting obligation to production source refs, one or
more independent behavior tests, and a separate refinement harness ref:

- O-P1 → lane_name.rs::LaneName::new + behavior tests (rejection/round-trip)
  + harness prop_lane_name_decides_validity.
- O-P2 → receipt.rs::LaneDigest::new + behavior tests (rejection/boundary) +
  harness prop_lane_digest_guards_counts.
- O-P3 → error.rs::ReceiptError + exhaustive-match behavior test + harness
  error_messages_stable_across_all_variants.
- O-P4 → receipt.rs::ReceiptPeriod::new + behavior tests + harness
  prop_receipt_period_orders_stamps.
- O-P5 → target_root.rs::RecordedTargetRoot::new + behavior tests + harness
  prop_recorded_root_decides_validity.
- O-P6 → schema.rs::RECEIPT_ENVELOPE_SCHEMA_VERSION + schema-gate behavior
  tests + harness prop_schema_gate (arbitrary-version rejection).
- O-V6 → verification/verus/receipt_schema.rs + schema-gate behavior tests
  (same behavior refs as O-P6 are NOT allowed to overlap harness refs;
  harness ref is the verus binding file itself) + ledger row tn-c6n.T1.

Behavior test refs and harness refs must stay disjoint per obligation.
