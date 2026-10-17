# Contract — tn-c6n (production-bound receipt invariants)

Bead `tn-c6n`. This contract reconciles the retained receipt-domain
requirements into production-bound obligations. No mirror types, no recovered
shadow files, no trust markers in new code.

## Requirements

- R1 lane-name validity → `type-contracts.md` §LaneName.
- R2 lane-digest consistency → §LaneDigest.
- R3 receipt-error exhaustiveness → `error-taxonomy.md` (9 variants).
- R4 receipt-period ordering → §ReceiptPeriod.
- R5 recorded-target-root validity → §RecordedTargetRoot.
- R6 receipt-schema gate → §ReceiptEnvelope + `schema.rs`.

## Obligation shape

Each requirement is discharged by executed checks that call production
constructors directly (behavior tests with exact variant assertions +
proptest acceptance/rejection properties), except R6 which additionally cites
the pinned-`verus` binding run as binding hygiene with zero behavior weight.

## Proof seeds

`proof-seeds.jsonl` (6 seeds, `proof-seed/v1`, all behavior-affecting).
Seeds are hints only; `proof-planner` owns lane decisions.

## Traceability

`traceability-matrix.jsonl` maps R1–R6 → contract clause → seed → planned
obligation → bridge → ledger (filled forward as states advance).
