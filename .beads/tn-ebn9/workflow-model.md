# tn-ebn9 Workflow Model

`Unclassified -> Classified(Permitted | Rejected)` per code site, evaluated
identically by Dylint (rustc AST/HIR) and the ast-grep lane (tree-sitter).

Transitions (tn-6r36 transitions 1 and 3, detector half):

1. Bounded `for` over finite input propagating `process(item)?` ->
   `Permitted`. Guard: `for` syntax (any body); failure path untouched.
2. `while let` over exhaustible state -> `Permitted`. Guard: `let`
   condition (bound = `Option`/iterator exhaustion).
3. `loop {}` or bare `while cond` with no cited bound ->
   `Rejected(UnboundedLoop)`. Guard: loop syntax present, no
   `TerminationClaim` visible to the detector (detectors never see claims;
   claims are tn-ce7x evidence).
4. `repeat`/`repeat_with`/`successors`/`from_fn`/`.cycle()` with no bound ->
   `Rejected(UnboundedIterator)`. Guard: infinite-source callee present;
   syntax form irrelevant (parity with transition 3).

ATDD mapping (bead §4):

- Happy 1 (finite collection + `process(item)?` passes) -> transition 1.
- Happy 2 (bounded loop and iterator forms share disposition) ->
  transitions 1 + finite-combinator `Permitted`.
- Error 1 (infinite retry without bound blocked) -> transition 3.
- Error 2 (unbounded stream without loop syntax rejected) -> transition 4.
