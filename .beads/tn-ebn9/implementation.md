# tn-ebn9 Implementation (holzman-rust)

Production Rust cutover from blanket loop bans to bounded-execution
checks, under Holzman zero-panic discipline (no unwrap/expect/panic/
indexing/cast/arithmetic-side-effect surface added; iterator pipelines
and early returns throughout; all new fns under the line/nesting
thresholds — strict source-only clippy is clean).

## Source Coverage Matrix

| Contract decision | Dylint source ref | Ast-grep source ref |
|---|---|---|
| D1 for permitted | crates/titania-dylint/src/functional_surface.rs::emit_bounded_policy | (no for detector; `allowed_bounded_for` stays clean) |
| D2 while-let permitted | crates/titania-dylint/src/functional_surface.rs::is_let_condition | crates/titania-lanes/src/ast_grep_lane/engine.rs::is_while_let_match |
| D3 loop/while rejected | crates/titania-dylint/src/functional_surface.rs::emit_unbounded_loop | crates/titania-lanes/src/ast_grep_lane/engine.rs::detect_unbounded_loop |
| D4 infinite iterators rejected | crates/titania-dylint/src/functional_surface.rs::emit_iterator_call_policy | crates/titania-lanes/src/ast_grep_lane/engine.rs::detect_unbounded_iterator |
| D5 call identity | crates/titania-dylint/src/functional_surface.rs::is_core_iter_fn | patterns (fail-closed documented residuals) |
| D6 repair class | BOUNDED_* lints carry RequiresHumanReview help text | crates/titania-lanes/src/ast_grep_lane/rules.rs::RULES |

## Files changed (production)

- `crates/titania-dylint/src/functional_surface.rs`: retired 3 loop
  lints; added `BOUNDED_EXECUTION_UNBOUNDED_LOOP` (Early: `loop` + non-let
  `while`) and `BOUNDED_EXECUTION_UNBOUNDED_ITERATOR` (Late: core-`iter::`
  `repeat`-family + zero-arg `.cycle()`).
- `crates/titania-lanes/src/ast_grep_lane/engine.rs`: retired 3 loop
  detectors; added `detect_unbounded_loop` (loop + non-let-while, earliest
  site) and `detect_unbounded_iterator` (5-pattern any); `#[cfg(test)]`
  skipping preserved.
- `crates/titania-lanes/src/ast_grep_lane/rules.rs`: RULES table cut over;
  retired now-unused `RepairKind::UseIteratorPipeline`.
- `crates/titania-lanes/rules/functional.yml`: catalog cut over (9 rules).
- `crates/titania-lanes/src/run_lane_dylint.rs`: `DYLINT_RULE_IDS`
  tracks the new lint registry.
- `crates/titania-core/src/finding/repair_catalog.tsv` (+ row-count pin
  80→79): new rows, `RequiresHumanReview`.
- `crates/titania-output/src/explain.rs`: worked examples for the new
  rules (bound-citing repairs).

## Files changed (tests/fixtures/docs)

Tests, fixtures, killer-demo bad fixture (now `loop` + `unwrap`), new
positive controls, explain suites, `v1-spec.md` (§6 table, §9.8 example,
§9.10 split, killer narrative, JSON examples), `VISION.md` example.
Full list in `regression-diff.md`.

## Safety evidence

- `cargo fmt --all -- --check`: exit 0.
- `cargo check --workspace --all-targets --all-features`: exit 0.
- Source-only strict clippy (AGENTS.md gate): exit 0.
- `cargo geiger` posture unchanged: zero new `unsafe` (workspace
  `forbid(unsafe_code)` holds outside the pre-existing audited dylint ABI
  exception, which this bead does not touch).
