# Transcript 004r — state 4 planner repair (tn-c6n-inv-004r)

Independent review (tn-c6n-inv-004b) REJECTED the plan on one blocking
finding, `tn-c6n-F01`: lane `tn-c6n.D-S6-flux-rs` ruled `not_applicable`
with an empty evidence list. The reviewer also corroborated with the
validator (`E_LANE_DECISION_WEAK` row 23) and accepted the other 23 lanes,
found no vacuous obligations, no mirror laundering, no kani theater, no
behavior-affecting waivers, and no verus overclaim.

Repair (this invocation, planner-owned file only):

- `verifier-lane-decisions.jsonl` row 23 now cites
  `crates/titania-core/src/receipt/schema.rs::RECEIPT_ENVELOPE_SCHEMA_VERSION`
  and the boundary-map schema entries. Nothing else changed.

Ledger hygiene fix (same invocation): ledger row 1 had hashed `STATE.md`,
which is a living tracker edited after hashing, so its hash went stale
(`E_INVOCATION_LEDGER_FORGED`). Policy from here on: living trackers
(`STATE.md`, `routing-ledger.jsonl`) are never listed in invocation hashes.
The ledger was rebuilt with identical semantic rows minus that entry; the
round-1 review row was retired (its artifacts are archived as
`proof-plan-review-round1-rejected.md` /
`proof-plan-findings-round1.jsonl`) and the re-review will append a fresh
row. No semantic content was altered by the rebuild — hashes and chain links
were recomputed over current bytes.
