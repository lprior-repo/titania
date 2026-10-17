# tn-ebn9 Domain Model

Inherits tn-6r36 ubiquitous language. This bead narrows D1
(bounded-execution criterion) to detector behavior:

- **Bounded retry** — `for` over finite input, `while let` over exhaustible
  state, or a finite combinator chain (`(0..max)`, `.map`, `.collect`).
  Permitted; no obligation raised.
- **Unbounded retry** — `loop {}` or bare-condition `while` with no cited
  `Bound`. Rejected under `BOUNDED_EXECUTION_UNBOUNDED_LOOP`; shipping
  requires a named bound with cited evidence (handled by tn-ce7x gate).
- **Unbounded iterator source** — `repeat`, `repeat_with`, `successors`,
  `from_fn`, `.cycle()` with no bound. Rejected under
  `BOUNDED_EXECUTION_UNBOUNDED_ITERATOR` even though no loop keyword
  appears (INV-1: syntax never establishes boundedness).
- **Named bound** — `Bound::FiniteInput` (input length identity),
  `Bound::Capped` (numeric cap + regression test), `Bound::ProtocolTerminal`
  (explicit terminal state). Citation and enforcement live in tn-ce7x;
  tn-ebn9 only classifies the site.

Out of scope (sibling beads): approved recovery shininess (tn-8obr owns
`unwrap_or` semantics), critical-gate wiring (tn-ce7x), Kani/mutants lanes
(tn-4isu/tn-7bq2).
