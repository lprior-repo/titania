# Black-Hat Review — tn-pfp1 (single-operator pass)

Scope: resolved-identity method checks + macro-span unfolding + UFCS/alias
paths in `functional_surface.rs`. Reviewer and implementer are the same
operator (degraded independence, recorded for landing).

## Attacks attempted

1. Evasion via `let f = Option::unwrap; f(v)` — callee resolves to a local,
   not linted. CONFIRMED LIMIT, shared with Clippy's own method lints.
   Documented in contract D-table; not a regression (old code missed all
   non-method forms too).
2. Evasion via path-only reference (`let f: fn(..) = Option::unwrap;`) —
   not a `Call`, not linted. Same documented limit as (1).
3. False positive via custom trait `unwrap` with default body — resolves to
   `Trait` container → `Other` → skipped. CORRECT per bead (unrelated
   same-name methods must not flag).
4. False positive via `dyn CustomTrait` receiver — trait container → skipped.
   CORRECT per bead.
5. `actionable_span` non-termination — expansion chains are finite; iterator
   terminates at the first non-expansion span. No defect found.
6. Shadowing: macro generating `.unwrap()` on a custom type — resolves to
   the custom method → skipped. CORRECT.
7. `opt_parent` misuse on trait-impl items — guarded by the `InherentImpl`
   check first; trait-impl methods land in `Other`. No defect found.
8. `def_path_str` rendering (`std::` vs `core::`) — caught during testing
   (direct calls went silent); fixed with both spellings + crate-name guard.
   REGRESSION RISK CLOSED by the 10-test suite (direct/UFCS/alias/macro all
   assert rejection; custom asserts silence).

## Verdict

Parity holds for direct, UFCS, alias, and macro-expanded forms with zero
false positives on custom methods (suite: 10/10). Two documented evasions
remain (let-bound and path-only references). No blocking finding.
