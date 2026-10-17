# Proof strategy — tn-vr2n

Implementation-bound lanes only. The change is a pure total predicate plus
a one-line chokepoint guard plus regression tests; executable behavior
evidence (`cargo test`) is the strongest applicable lane. Deductive
(Verus), bounded-model (Kani), and refinement (Flux) lanes are
not-applicable with concrete evidence refs; Kani is additionally excluded
by parent delegation (lane owned by tn-4isu/tn-7bq2). No concurrency, no
unsafe, no hostile-binary input: loom/miri/cargo-fuzz add no rows beyond
the required default four (no trigger vocabulary in any seed).

Obligations: three required behavior obligations (one per seed), all
`mode: planned` → executed at state 12 by formal-verifier.
