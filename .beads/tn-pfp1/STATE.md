# tn-pfp1 State

Bead: dylint resolved forbidden-method identities (P0, in_progress).
Execution: single-operator in-checkout per user direction.

- Contract: DONE (contract.md, 4 proof seeds, traceability 4/4).
- Implementation: DONE. `functional_surface.rs`: late pass resolves method
  identities via `type_dependent_def`/`qpath_res` + core ADT check; macro
  spans unfolded to call site; UFCS/alias `Call` paths covered.
- Tests: DONE. 4 new fixture tests in `policy_surface.rs` (custom ignored;
  macro/UFCS/alias rejected). Full file: 10 passed.
- Gates: fmt exit 0; workspace check exit 0; strict source clippy exit 0;
  rustdoc -D warnings exit 0; lanes+dylint suites 401 pass / 0 fail;
  final `moon ci` running (baseline had 4 pre-existing/global failures:
  kani 12x600s timeouts in untouched titania-core, mutants no-artifact,
  geiger transient now exit 0, policy-scan hermetic env finding).
- Debug detour (resolved, no leftovers): `def_path_str` renders
  `std::option::Option`; `AssocContainer::InherentImpl` is a unit variant
  (parent impl via `opt_parent`); `hir_id` is a field; `QPath` has no
  `LangItem` arm in this nightly.
