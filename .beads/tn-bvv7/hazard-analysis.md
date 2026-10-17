# Hazard Analysis — tn-bvv7

- H1 Over-blocking core: STATE/ASYNC detectors flagging legitimate
  core code (frozen catalog, doc prose). Mitigated by the
  frozen-catalog allowance tests (exact + property) and the core
  self-scan (zero banned markers in `titania-core/src/`).
- H2 Under-blocking bypass: renamed imports, re-exports,
  fully-qualified paths, macro bodies, manifest dependencies slipping
  through. Mitigated by token-boundary fully-qualified arms (all four
  legacy families), macro-body scanning (string scan sees bodies),
  alias coverage tests, and the manifest contract test.
- H3 Boundary-name collision: `fs_extra` vs `fs`, `RngCore` vs `Rng`,
  `Instantaneous` vs `Instant`. Mitigated by leading/trailing
  identifier-boundary checks, pinned at unit, property, and lane level
  (`allowed_core_rngcore_import`, `allowed_core_std_boundary_imports`).
- H4 Catalog drift: YAML documents vs `RULES` vs TSV vs spec table
  diverging. Mitigated by parity tests (exact-id-set, repair hints,
  full-matrix reachability) and the TSV count pin (83).
- H5 Evidence theater: claiming enforcement from prose or from
  comment/string mentions. Mitigated by comment/string-blindness tests
  and raw command logs for every gate.
