# Proof-to-Rust review — tn-c6n State 7 bridge

reviewer_skill: proof-reviewer
reviewer_invocation_id: tn-c6n-inv-007r
writer_invocation_id: tn-c6n-inv-007

Reviewed: `.beads/tn-c6n/proof-to-rust-map.md` and
`.beads/tn-c6n/rust-refinement-obligations.jsonl` (7 rows:
`tn-c6n.B-P1`..`tn-c6n.B-P6` proptest + `tn-c6n.B-V6` verus) against
`.beads/tn-c6n/proof-obligations.planned.jsonl` (7 obligations:
`tn-c6n.O-P1`..`tn-c6n.O-P6` + `tn-c6n.O-V6`).

STATUS: APPROVED

## Findings (each reject criterion checked, verbatim outcome)

1. Behavior-affecting obligation without a bridge: PASS. All 7
   obligations are `behavior_affecting: true` and each has exactly one
   bridge row with equal `proof_id` (`O-P1`→`B-P1`, …, `O-P6`→`B-P6`,
   `O-V6`→`B-V6`). No orphan obligation, no orphan bridge.
2. Requirement/contract/verifier identity mismatch: PASS. Per row,
   `requirement_id`, `contract_clause`, and `verifier` are identical
   between obligation and bridge (`REQ-LANE-NAME`/`R1`/proptest,
   `REQ-LANE-CONSISTENCY`/`R2`/proptest,
   `REQ-ERROR-EXHAUSTIVENESS`/`R3`/proptest,
   `REQ-PERIOD-ORDERING`/`R4`/proptest, `REQ-TARGET-ROOT`/`R5`/proptest,
   `REQ-SCHEMA-GATE`/`R6`/proptest for `B-P6`,
   `REQ-SCHEMA-GATE`/`R6`/verus for `B-V6`). `rust_target` equals the
   obligation `target`, and each `evidence_command` equals the paired
   obligation `command`.
3. Bridge not behavior_affecting: PASS. All 7 bridge rows carry
   `behavior_affecting: true`.
4. Behavior_test_refs pointing at verifier harnesses: PASS. `B-P1`..`B-P6`
   behavior refs all live in
   `crates/titania-core/tests/receipt_domain_invariants.rs` (independent
   executable checks); harness refs all live in
   `crates/titania-core/tests/receipt_domain_properties.rs`. `B-V6`
   behavior refs are the same two invariants tests; its harness ref is
   the registered binding file `verification/verus/receipt_schema.rs`.
   No behavior ref names a harness.
5. Behavior/harness ref overlap: PASS. Per row the two ref sets are
   disjoint; no test symbol appears on both sides.
6. Source_refs not in path::symbol form: PASS. Every row has exactly one
   `source_ref` of the form `<path>::<Symbol>(::member)?`, matching the
   obligation target (`lane_name.rs::LaneName::new`,
   `receipt.rs::LaneDigest::new`, `error.rs::ReceiptError`,
   `receipt.rs::ReceiptPeriod::new`,
   `target_root.rs::RecordedTargetRoot::new`,
   `schema.rs::RECEIPT_ENVELOPE_SCHEMA_VERSION`,
   `verification/verus/receipt_schema.rs::schema_accepts_current`).
7. B-V6 claiming constructor-body proof: PASS. `B-V6.refinement_claim`
   states the harness is the registered binding file itself with no
   copied model, and `expected_evidence` carries the parenthetical
   "(binding hygiene; behavior weight on the paired behavior tests)".
   The map row claims only "binding file verifies clean (hygiene)".
   Obligation `O-V6` assumptions independently pin this classification
   (source linkage, not body proof; vacuous lemma carries zero behavior
   weight; constructor behavior evidenced by `O-P1`..`O-P6`, never by
   this run).

## Non-blocking observation (State-12 execution, not a rejection)

Each row's `evidence_command` runs a single test binary while the row's
weight spans both binaries (e.g. `B-P1`..`B-P5` evidence runs the
properties binary but behavior tests live in the invariants binary;
`B-P6` is the mirror image). State-12 must execute both
`receipt_domain_properties` and `receipt_domain_invariants` (plus the
verus file for `B-V6`) and attach the raw logs before flipping any
`mapping_status` from `planned` to `verified`.
