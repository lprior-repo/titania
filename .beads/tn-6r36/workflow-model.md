# tn-6r36 Workflow Model

## States

`Unclassified → Classified(Permitted | Rejected)`. Terminal: verdict emitted
with reason code. No re-entry without new facts.

## Transitions (guards → outcomes)

1. Bounded `for` over finite input propagating `process(item)?`
   → `Permitted(BoundedIteration)`. Guard: `TerminationStatus::Claimed`
   with `Bound::FiniteInput`; failure path preserves typed cause (`?`).
2. `unwrap_or_else` consuming typed failure under live approval
   → `Permitted(ApprovedRecovery)`. Guards: handler consumes `E`
   (`match`/`map_or_else`/typed default type); `RecoveryApproval` live
   (owner + reason + unexpired + review cited).
3. Unbounded retry spelled as iterator combinators
   → `Rejected(UnboundedDespiteCombinators)`. Guard: no `TerminationClaim`;
   syntax form is irrelevant per INV-1.
4. `unwrap_or` / `let _ =` / bare statement discarding cause without approval
   → `Rejected(ErrorErasure)`.
5. Any required-behavior evidence absent, contradictory, `SKIP`, timeout,
   `UNPROVEN`, or missing tool → `Rejected(EvidenceAbsent)` (fail closed).
6. `RequiredUnproven` obligation on a `Critical` module
   → shipping blocked (`Rejected(CriticalGateBlocked)`).

## ATDD mapping (bead §4)

- Happy 1 → transition 1. Happy 2 → transition 2.
- Error 1 → transition 3.

## Terminal states

`Permitted` (ship-eligible) and `Rejected` (named code). `Rejected` on a
critical path blocks `tn-fnyg` release packaging by dependency edge.
