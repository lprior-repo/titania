# tn-ebn9 Boundary Map

Parse-at-boundary, total core:

- Dylint Early pass: rustc AST is the boundary; `#[cfg(test)]` modules are
  out of scope (existing `test_module_depth` gate reused for the loop
  check). `for` needs no input validation (always permitted).
- Dylint Late pass: HIR + `typeck` identity is the boundary.
  `repeat`-family calls resolve through `qpath_res`; only `DefKind::Fn`
  in `core`/`std` whose path contains `iter::` is flagged. Unresolvable or
  foreign same-name functions pass (fail open toward the historic verdict).
- Ast-grep lane: tree-sitter parse is the boundary; malformed subtrees
  simply do not match. `#[cfg(test)]` byte ranges are excluded for both
  new rules (same `test_ranges` mechanism as the retired loop rules).
- `while let` discrimination: AST `ExprKind::Let` (Dylint) and
  `while`-match text prefix (ast-grep). Comment-embedded `while let` text
  never forms an AST node, so comment/string fixtures stay clean.
- Shell boundary: `titania-check` aggregates lane findings unchanged; no
  new lanes, no new CLI surface, no new config keys.
