# tn-ebn9 Test Plan

Behavior tests prove the detector contract end to end (Testing Trophy:
integration-heavy; exact assertions everywhere; deterministic; public
APIs only).

## Proof/Refinement Coverage Matrix

| Obligation | Behavior tests (integration) | Refinement harness (detector corpus) |
|---|---|---|
| tn-ebn9.O-P1-PROP | killer_demo `bounded_for_with_question_passes` (exit 0, Pass, no BOUNDED finding); lane `real_for_loop` Clean | dylint `bounded_execution_permits_finite_for_with_question`; functional `allowed_bounded_for` content pins |
| tn-ebn9.O-P2-PROP | killer_demo bad rejects (`BOUNDED_EXECUTION_UNBOUNDED_LOOP` AstGrep + `CLIPPY_UNWRAP_USED`, CodeOnly) | dylint `bounded_execution_rejects_unbounded_iterators`; lane `real_unbounded_violations_still_detected` |
| tn-ebn9.O-P3-PROP | lane `allowed_bounded_for` while-let arm stays clean | dylint `bounded_execution_permits_while_let` |
| tn-ebn9.O-P4-PROP | lane `real_for_loop` Clean vs `real_unbounded_violations` Reject pair | dylint `bounded_execution_permits_finite_iterators` + `..._rejects_condition_while` + `..._rejects_unbounded_loop` |

## Failing-first record

Before implementation, the new-behavior tests fail against the blanket-ban
baseline: bounded `for`+`?` is rejected (`FUNC_LOOPS_FOR`), and no
`BOUNDED_EXECUTION_*` rule exists (unknown-id errors). After the cutover,
all suites pass: lanes + dylint + core + output 855/855; killer_demo
17/17; explain 9/9.

## Negative/edge coverage

- Unbounded `loop`, bare `while`, `repeat`/`cycle`/`successors`/`from_fn`
  each have a dedicated Reject test in at least one enforcer.
- `while let`, finite `for`, `(0..max)`, `.map`/`.collect` each have a
  dedicated Clean/Pass test.
- Comment/string-embedded keywords stay clean (pre-existing false-positive
  battery, untouched semantics).
- `#[cfg(test)]` modules skip both new rules (dedicated skip tests).
- Span accuracy pins first-match coordinates for both new rules.
