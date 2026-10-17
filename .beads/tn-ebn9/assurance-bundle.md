# tn-ebn9 Assurance Bundle

Every requirement maps to contract, proof/refinement/test/source
evidence (paths are worktree-relative):

- tn-ebn9-REQ-BOUNDED-FOR → contract D1; O-P1-PROP + R-P1;
  `crates/titania-dylint/tests/policy_surface.rs`
  (bounded_execution_permits_finite_for_with_question),
  `crates/titania-check/tests/killer_demo.rs`
  (bounded_for_with_question_passes); ledger PASS;
  raw `raw/cargo-test-*.log`.
- tn-ebn9-REQ-COMBINATOR-RETRY → contract D4; O-P2-PROP + R-P2;
  `bounded_execution_rejects_unbounded_iterators`,
  `ast_grep_functional_real_unbounded_violations_still_detected`,
  bad-fixture Reject (`BOUNDED_EXECUTION_UNBOUNDED_LOOP` +
  `CLIPPY_UNWRAP_USED`); ledger PASS.
- tn-ebn9-REQ-WHILE-LET → contract D2; O-P3-PROP + R-P3;
  `bounded_execution_permits_while_let`, `allowed_bounded_for`;
  ledger PASS.
- tn-ebn9-REQ-BOUNDED-PAIR → contract D1/D4; O-P4-PROP + R-P4;
  `bounded_execution_permits_finite_iterators`,
  `real_for_loop` Clean vs unbounded Reject; ledger PASS.

Gates: fmt/check/strict-clippy/tests/doc exits recorded in
`machine-gate-report.md`. Global lanes explicitly not run (parent
direction) and recorded as accepted blockers; two environmental test
failures classified BLOCK_GLOBAL with base-commit reproduction for the
cli_dispatch case.
