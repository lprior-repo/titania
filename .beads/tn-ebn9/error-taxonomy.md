# tn-ebn9 Error Taxonomy

Detectors reuse the existing lane `Finding` shape; no new `thiserror`
enum is introduced (obligation errors belong to tn-ce7x):

- `BOUNDED_EXECUTION_UNBOUNDED_LOOP` (`Reject`, `RequiresHumanReview`) —
  `loop {}` or bare `while` without a named bound. Message cites the
  required evidence classes (`Bound::FiniteInput` / `Bound::Capped` with
  regression test / `Bound::ProtocolTerminal`).
- `BOUNDED_EXECUTION_UNBOUNDED_ITERATOR` (`Reject`, `RequiresHumanReview`) —
  infinite iterator source without a named bound. Help states the parity
  rule: combinator syntax never establishes boundedness.

Fail-closed rules:

- Unresolvable callee names (`repeat` etc. that do not resolve to core
  `iter::`) are NOT flagged (no finding on unknown identity; the historic
  verdict for those spellings was pass).
- Unresolvable `.cycle()` receivers ARE flagged (zero-arg name heuristic;
  residual tn-ebn9-R2).
- Anything the detector cannot classify stays `Permitted` except the two
  named unbounded shapes above; absence of evidence is handled downstream
  by tn-ce7x (`EvidenceAbsent`), never by the detector.
