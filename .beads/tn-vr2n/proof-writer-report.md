# Proof writer report — tn-vr2n

No deductive proof, model, or harness code was authored for this bead, by
plan: all three planned obligations are executable behavior obligations
over a pure total predicate and its call sites. There is no invariant here
that Verus, Kani, or Flux could strengthen beyond the exact-assertion
behavior suites (see proof-plan-review.md for the accepted rationale).

Authored instead (test-writer, state 9): the `run_cargo_invocation`
behavior suite, in-src unit tests (static-argv negative property, wiring
rejection), and ceiling-specific regressions across the env, config, scrub,
and CLI suites.

Behavior evidence lives in `proof-evidence.md`; ledger closure is owned by
formal-verifier at state 12. No verification waivers were used at any point.
