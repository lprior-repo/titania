# tn-pfp1 Contract — Resolved Forbidden-Method Identities

Bead: `tn-pfp1` (P0, in_progress, owner Lewis). Scope: replace name-only
method decisions with resolved standard `Option`/`Result` identities;
inspect macro-expanded production calls with original actionable spans.
No other lint families. Tests/build-script boundaries retained.

## Decisions

| # | Decision | Choice |
|---|----------|--------|
| D1 | Identity source | `type_dependent_def` / `qpath_res` to `AssocFn` DefId; name is a pre-filter only. |
| D2 | Core test | Inherent impl (`InherentImpl`) whose self type is an ADT from `core` with def path `core|std ::option::Option` or `core|std ::result::Result` (`def_path_str` renders `std`). |
| D3 | Unresolvable receiver | Fail closed: keep historic name-based verdict (never silently skip). |
| D4 | Macro expansions | No `from_expansion` skip; unfold to outermost call site (`actionable_span`). |
| D5 | UFCS/alias | `Call` with path callee resolving to the same core `AssocFn` is linted identically (covers `Option::unwrap(v)` and type-alias paths). |
| D6 | Boundaries | Early-pass `#[cfg(test)]` gate and late-pass behavior unchanged (retained, not extended). |
| D7 | No relaxation | Direct core calls still rejected exactly as before; custom same-name methods newly ignored. |

## Ubiquitous language (excerpt)

Resolved identity (DefId, not spelling); inherent method; actionable span
(outermost macro call site); fail-closed fallback; UFCS; type alias.

## Open domain questions

None.

## Illegal states that remain representable

- A `let`-bound function pointer (`let f = Option::unwrap;`) evades the
  `Call`-path check (callee resolves to a local); same limitation as Clippy
  itself. Documented, not solved here.
- Unresolvable receivers fall back to name matching (D3): a custom method in
  unresolvable (erroneous) code could still be flagged. Fail-closed by design.

## Proof seeds emitted

4 seeds: 3 proptest-style behavior (covered by dylint fixture tests), 1 Kani
n/a (no numeric content; evidence: structural DefId comparison).

## Next steps

Behavior tests in `policy_surface.rs` (custom ignored; macro/UFCS/alias
rejected); gates: fmt, workspace check, strict source clippy, dylint fixture
suite, `moon ci`.
