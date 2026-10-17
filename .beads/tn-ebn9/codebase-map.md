# Codebase Map — tn-ebn9 (explore)

Scope: bounded-execution detector cutover. Production behavior lives in two
parallel enforcers that must stay in parity (same policy, two mechanisms):

## Dylint (compiler lint, nightly + rustc_private)

- `crates/titania-dylint/src/functional_surface.rs` — EarlyLintPass (loop
  syntax) + LateLintPass (method/call identity). Single file owns all
  `FUNC_*`/`BOUNDED_*` surface lints; `lib.rs` registers passes.
- `crates/titania-dylint/tests/policy_surface.rs` — fixture-string contract
  tests driven through real `cargo dylint` runs.
- `crates/titania-lanes/src/run_lane_dylint.rs` — `DYLINT_RULE_IDS` allowlist
  the lane parses out of cargo-dylint output; unknown ids are dropped, so
  this list must track the lint registry exactly.

## Ast-grep lane (structural, stable-safe)

- `crates/titania-lanes/rules/functional.yml` — embedded catalog; doubles as
  the enablement list (`rule_enabled`) and the contract-test surface.
- `crates/titania-lanes/src/ast_grep_lane/rules.rs` — `RULES` dispatch table
  (`RuleDef` id/message/effect/repair/detector/scope); `embedded_rule_ids`
  must equal the YAML ids (pinned by
  `ast_grep_lane_runtime_dispatch_matches_embedded_yaml_ids`).
- `crates/titania-lanes/src/ast_grep_lane/engine.rs` — real
  ast-grep detectors over tree-sitter Rust; `#[cfg(test)]` ranges skipped
  for `FUNC_*`/`BOUNDED_*`.
- `crates/titania-lanes/src/ast_grep_lane.rs` — `run()` parses each source
  once, asks every YAML-enabled rule for its first match site.

## Shared contract + consumers

- `crates/titania-core/src/finding/repair_catalog.tsv` (+ `repair_catalog.rs`
  `for_rule` lookup, row-count pin) — single source of truth for
  `explain`; `titania-output/src/explain.rs` renders worked examples.
- `crates/titania-check/tests/killer_demo.rs` +
  `fixtures/strict_ai_loop_unwrap/{bad,repaired}` — consumer-visible
  regression proving the control end to end.
- `v1-spec.md` §6 (rule catalog), §9.8 (exception example), §9.10
  (source/test split), §12+ killer narrative; `VISION.md` exception example.

## Risks

- Parity drift between Dylint and ast-grep detectors (different mechanisms,
  same verdicts) — covered by mirrored positive/negative fixtures.
- Catalog propagation fan-out (yml, RULES, DYLINT_RULE_IDS, TSV, explain,
  spec, fixtures) — mechanical; verified by dispatch/parity/count tests.
- `cycle()` name heuristic (Dylint flags any zero-arg `.cycle()`) and
  take-bounded infinite sources (flagged fail-closed) — recorded residuals.
