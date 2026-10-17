# tn-ebn9 Hazard Analysis

- H1 — Parity drift: Dylint flags a site the ast-grep lane permits (or vice
  versa), so `check` reports lane-dependent verdicts. Controlled by
  mirrored positive/negative fixtures in both suites plus the killer-demo
  E2E (AstGrep + Dylint + Clippy must agree: bad rejects, bounded passes).
- H2 — `while let` misclassification: a future rustc AST shape change moves
  `while let` out of `ExprKind::Let`, silently converting permits into
  rejects (fail-closed direction) or vice versa. Controlled by the
  `bounded_execution_permits_while_let` Dylint regression (fails on shape
  drift) — rejection drift is loud, permission drift is impossible without
  deleting the guard.
- H3 — Catalog propagation miss: a consumer list (`DYLINT_RULE_IDS`,
  TSV, `functional.yml`, spec table) keeps a retired `FUNC_LOOPS_*` id.
  Controlled by dispatch/parity/count tests
  (`runtime_dispatch_matches_embedded_yaml_ids`,
  `emits_every_embedded_rule_id`, `catalog_parses_to_expected_row_count`,
  explain coverage).
- H4 — Over-permissive `for`: `for x in repeat_forever()` is permitted by
  the `for` arm while the inner `repeat` is flagged — net verdict still
  Reject via the iterator rule. No bypass: verified by the
  `bounded_execution_unbounded_iterator_violation` fixture shape.
- H5 — Single-operator review: all lanes executed and reviewed by one
  operator; independence is asserted by distinct ledger invocations, not
  distinct humans. Recorded as residual debt (see truth-serum report).
