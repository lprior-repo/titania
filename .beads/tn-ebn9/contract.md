# tn-ebn9 Contract

Bead: `tn-ebn9` (P1). Cut over Dylint and structural rules from universal
`for`/`while`/`loop` rejection to the bounded-execution contract.

## Decisions

| # | Decision | Choice |
|---|----------|--------|
| D1 | `for` loops | Always permitted (finite-input iteration; infinite inner sources caught by the iterator rule). |
| D2 | `while let` | Permitted (bound = `Option`/iterator exhaustion). |
| D3 | `loop {}` + bare `while` | Rejected as `BOUNDED_EXECUTION_UNBOUNDED_LOOP` (named bound required). |
| D4 | Infinite iterator sources | Rejected as `BOUNDED_EXECUTION_UNBOUNDED_ITERATOR` (same bound policy; syntax irrelevant). |
| D5 | Unresolvable callee identity | Pass for free functions (historic verdict); flag for zero-arg `.cycle()` (heuristic, residual R2). |
| D6 | Repair hint | `RequiresHumanReview` (a bound citation is a human-audited claim, not a rewrite). |
| D7 | Landing | No relaxation: every previously-rejected unbounded shape is still rejected, under new ids. |

## Artifacts

Domain/type/workflow/error/boundary/hazard (this directory),
`proof-seeds.jsonl`, `traceability-matrix.jsonl`.

## Illegal states that remain representable

- A `take`-bounded infinite source is still rejected (residual R1): the
  detector cannot prove `take` dominance. Fail-closed by design.
- A custom zero-arg `.cycle()` method is still rejected (residual R2).

## Proof seeds emitted

4 seeds, all proptest (executable detector-corpus evidence):
`tn-ebn9.P1` bounded-for permit, `tn-ebn9.P2` combinator-retry reject,
`tn-ebn9.P3` while-let permit, `tn-ebn9.P4` bounded-pair parity.
Kani/Verus/Flux/cargo-fuzz lanes are not-applicable per seed (structural
detector: no arithmetic, no deductive invariant, no refined indices, no
byte codec). Loom/Miri need no lanes (no concurrency/unsafe surface).
