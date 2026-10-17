# tn-n8y6 truth-serum self-audit

## Persona 1 — advocate (what the work proves)

- 10 new executable behavior tests through production public APIs, all exact
  assertions (no `is_ok()`-only, no mocks, Given/When/Then), covering
  boundaries the existing suite never asserted (equality getter preservation,
  zero-zero, direct `LaneDigest` serde rejection, `contains()` direct,
  diff reference identity, `(t,t)` period, adjacent `(10,9)` rejection).
- Two named production predicates proven killable twice each: once by
  first-hand seeded-defect runs with observed failure output, once by the
  real `cargo mutants` test-mode verdicts (`CaughtMutant`).
- All five parent-mandated gates exit 0; production diff empty; worktree
  contains only the two intended additions.

## Persona 2 — adversary (what could still be wrong)

1. **Over-broad mutant filter.** `-F` matched 5 mutants, not the intended 2
   (2 extra `kani_inventory` struct-field deletions matched opaquely).
   Counter: the 2 claimed kills are named exactly in `caught.txt` and
   `outcomes.json`; the 3 extras are listed in `unviable.txt` and excluded.
   No inflation — but the filter behavior is recorded so the next run can
   tighten it.
2. **Fast test-mode (6 s).** Suspicious speed for real test execution.
   Counter: `CaughtMutant` is only emitted after test failure by tool
   contract, AND each predicate was independently killed first-hand with
   visible stderr. If the tool result were vacuous, the seeded runs still
   stand. Residual: per-mutant test logs were not retained (summary +
   outcomes only).
3. **`--baseline=skip`.** Unmutated baseline assumed green.
   Counter: full `cargo test -p titania-core` was green in-session minutes
   before; recorded openly, not hidden.
4. **Single-agent specialist provenance.** No named specialist subagents
   exist in this run; skill bodies applied inline.
   Counter: recorded as degraded (`fallback: true`,
   `user_approved_fallback: false`) in `routing-ledger.jsonl` — claimed as
   residual, not as compliance.
5. **Missing validator + deferred global gates.** `tools/go-skill-v9-validate`
   absent; `moon ci`/kani/full mutants not run per parent direction.
   Counter: all recorded as `not-run` in the verification ledger with owners;
   bead-close readiness for global gates belongs to landing (`tn-9acl`
   blocked downstream can re-verify).

## Verdict

Claims match raw evidence; exclusions and deferrals are explicit. No
hallucinated output: every number above traces to a retained log or observed
terminal result. killing-claim scope is 2 named mutants + 2 seeded defects —
nothing broader.
