# Contract — tn-bvv7

Bead: architecture — enforce designated pure-core effect boundaries.

Postconditions (must all hold):
1. Forbidden effect capabilities are rejected inside designated pure
   core modules (filesystem, network, clock, environment, process,
   ambient mutable state, async runtime), including renamed imports,
   re-exports, macro-hidden uses, fully-qualified paths with no import,
   and manifest dependencies.
2. Deterministic core calculations over explicit inputs pass, and shell
   I/O behind a declared typed port is accepted.

Acceptance map:
- Given deterministic core calculations over explicit input values,
  when checked, then core policy passes → allowed fixtures + scope
  tests + lane clean-controls test.
- Given shell I/O behind the declared typed port, when checked, then
  effect policy accepts the boundary → scope tests (shell paths out of
  scope) + `allowed_*` lane controls.
- Given direct or renamed clock/fs/env/network use in a core module,
  when checked, then a named effect violation blocks → FQ/alias/macro
  fixtures + detector tests + lane rejection tests.
- Given a dependency or local macro that introduces forbidden ambient
  effects in core, when checked, then it cannot bypass capability
  policy → manifest contract test + macro fixture + STATE detector.

Invariants: Moon remains the only CI orchestrator; no GitHub Actions
restoration; no source listing or coverage figure is presented as proof
of production execution; no commit/push/sync without authorization.
