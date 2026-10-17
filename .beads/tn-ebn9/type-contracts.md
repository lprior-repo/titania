# tn-ebn9 Type Contracts

```rust
pub enum SiteVerdict { Permitted, Rejected { rule: BoundRule } }
pub enum BoundRule { UnboundedLoop, UnboundedIterator }
```

Detector inputs are already-parsed syntax facts (AST/HIR node or ast-grep
match); outputs are `SiteVerdict`. Both enforcers implement the same table:

| Site | Verdict |
|---|---|
| `for x in finite` (+ `?` or not) | `Permitted` |
| `while let pat = expr` | `Permitted` |
| `while cond` (non-`let`) | `Rejected { UnboundedLoop }` |
| `loop {}` | `Rejected { UnboundedLoop }` |
| `repeat`/`repeat_with`/`successors`/`from_fn` (core `iter::`) | `Rejected { UnboundedIterator }` |
| zero-arg `.cycle()` | `Rejected { UnboundedIterator }` |
| `(0..max)`, `.map`, `.collect`, `.flatten` | `Permitted` |

Error type: existing lane `Finding` with `rule_id`
(`BOUNDED_EXECUTION_UNBOUNDED_LOOP` /
`BOUNDED_EXECUTION_UNBOUNDED_ITERATOR`), `effect: Reject`,
`repair: RequiresHumanReview`. No new error enum: the obligation identity
(the "named bound") is a tn-ce7x aggregate concern, not a detector type.

Residual precision notes (fail-closed, documented, not silent):

- Take-bounded infinite sources (`repeat(x).take(n)`) are still rejected;
  the detector does not prove `take` dominance (refinement candidate
  tn-ebn9-R1, non-behavior).
- Zero-arg `.cycle()` on a custom non-iterator type is still rejected;
  method-identity resolution for trait methods is heuristic (candidate
  tn-ebn9-R2, non-behavior).
