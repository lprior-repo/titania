# tn-ebn9 Test-Writer Report

Failing-first behavior tests written before the implementation cutover;
all now pass (see `formal-verification-report.md` for commands + exits).

## Proof/Refinement Coverage Matrix

(Same matrix as `test-plan.md`; writer attests each cell exists and
passes.)

- Dylint (`crates/titania-dylint/tests/policy_surface.rs`): 6 new tests —
  finite-for+? pass, loop reject, condition-while reject, while-let pass,
  unbounded-iterator reject, finite-iterator pass. 15/15 file-green.
- Ast-grep lane: new violation fixtures
  (`bounded_execution_unbounded_loop_violation`,
  `bounded_execution_unbounded_iterator_violation`,
  `bounded_execution_unbounded_while_violation`), new allow fixture
  (`allowed_bounded_for`), new cfg-test fixtures (3), new span fixture
  (`loop_indented_col4_violation`); catalog/span/dispatch/skip suites
  updated to the new ids. Full lanes suite green.
- Killer-demo E2E: bad fixture migrated to unbounded `loop` + `unwrap`
  (rejects with `BOUNDED_EXECUTION_UNBOUNDED_LOOP` +
  `CLIPPY_UNWRAP_USED`); new `bounded_for_with_question_passes` positive
  control (exit 0, Pass, zero BOUNDED findings). 17/17 green.
- Explain: catalog + CLI suites updated to the new rule entries. 9/9 CLI
  green; output-catalog suite green.

No test was weakened: retired assertions (blanket `for` rejection) were
replaced by strictly stronger paired assertions (bounded passes +
unbounded rejects under the same policy).
