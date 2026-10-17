# tn-ebn9 Research Notes

## Files opened (exact refs)

- `crates/titania-dylint/src/functional_surface.rs:38-57` — blanket bans: `FUNC_LOOPS_FOR` (for → iterator pipelines), `FUNC_LOOPS_WHILE` (while → bounded iterator/state-machine), `FUNC_LOOPS_LOOP` (loop → bounded control flow). `emit_loop_policy` (`:239-246`) flags every `ForLoop`/`While`/`Loop` outside `#[cfg(test)]`. `method_policy` (`:280-287`) is spelling-only for unwrap family.
- `crates/titania-lanes/src/ast_grep_lane/rules.rs:109-133` — `RULES` table: `FUNC_LOOPS_FOR`/`WHILE`/`LOOP`, all `FindingEffect::Reject` + `UseIteratorPipeline`, engine detectors `detect_for_loop`/`detect_while_loop`/`detect_loop_block`.
- `crates/titania-lanes/src/ast_grep_lane/engine.rs:94-111` — detectors: `for $LOOP in $ITER`, `while $COND`, `loop { $$$BODY }`, all skipping `#[cfg(test)]` ranges.
- `crates/titania-lanes/rules/functional.yml:5-38` — embedded catalog mirrors the three loop bans (Reject/UseIteratorPipeline, production-only ignores).
- `crates/titania-check/tests/killer_demo.rs:169-218` — bad fixture asserts `FUNC_LOOPS_FOR` + `CLIPPY_UNWRAP_USED`; repaired asserts Pass. Fixtures: `fixtures/strict_ai_loop_unwrap/bad/src/lib.rs` (`for` + `unwrap`), `repaired/src/lib.rs` (`flatten`, no loop/unwrap).
- `crates/titania-dylint/tests/policy_surface.rs:20-33,288-294` — `FIXTURE_LOOP_SURFACE` (for+while+loop) asserts all three `FUNC_LOOPS_*` fire.
- `crates/titania-lanes/tests/ast_grep_functional.rs:26-37` — `REQUIRED_RULE_IDS` includes the three `FUNC_LOOPS_*`.
- `crates/titania-lanes/src/run_lane_dylint.rs:16-36` — `DYLINT_RULE_IDS` includes the three `FUNC_LOOPS_*`.
- `crates/titania-core/src/finding/repair_catalog.tsv:1-3` — catalog rows for the three loop bans.
- `crates/titania-output/src/explain.rs:86-98` — worked examples for `FUNC_LOOPS_FOR`.
- `v1-spec.md §6` — structural table mandates the three loop bans; `§9.8` example exception cites `FUNC_LOOPS_FOR`; `§9.10` notes tests may use for/while/loop while production may not.
- Parent contract `tn-6r36`: `contract.md` D1 (syntax-irrelevant boundedness), `type-contracts.md` (`Bound`/`TerminationClaim`/`TerminationStatus`), `workflow-model.md` transitions 1 (bounded for+? → Permitted) and 3 (unclaimed combinator retry → Rejected), `error-taxonomy.md` (`MissingBound`, `UnboundedDespiteCombinators`).

## Patterns searched

- `FUNC_LOOPS` consumers: dylint surface + tests, lanes rules/engine/functional.yml/run_lane_dylint, lanes tests (functional, lane, catalog_dispatch, cfg_test_skip, span_accuracy), check tests (killer_demo, explain), output explain, core repair_catalog + rule_id + core tests. Full list in implementation.md §migration.
- `detect_for_loop|detect_while_loop|detect_loop_block`: only `engine.rs` + `rules.rs`.
- v1-spec `bounded`: §6 table + §9.8 exception shape; no existing bounded-iterator rule.

## Research questions

- Which consumer-visible regression proves this control without a second convention? The killer_demo bad/repaired pair (AstGrep lane finding + Dylint diagnostic → aggregate Reject vs Pass). After cutover the same pair proves the new contract: bounded `for`+`?` passes; unbounded `loop`+`unwrap` rejects with the named boundedness obligation; unbounded iterator without loop syntax rejects under the same policy.

## Scope (smallest correct change)

1. Dylint: remove `FUNC_LOOPS_FOR/WHILE/LOOP`; add `BOUNDED_EXECUTION_UNBOUNDED_LOOP` (`loop {}`, `while` without `let`) and `BOUNDED_EXECUTION_UNBOUNDED_ITERATOR` (`repeat`, `repeat_with`, `successors`, `from_fn`, `cycle`). Permit `for` and `while let`.
2. Ast-grep: same cutover in `functional.yml` + `RULES` + engine detectors (first-match-outside-tests preserved; `while let` excluded by condition-text check).
3. Migrate consumers: killer_demo (bad → unbounded loop + unwrap; new positive bounded-for+? control), explain, repair_catalog, lane/dylint tests, fixtures, v1-spec §6 (+§9.8 example id).
4. No change to `FUNC_UNWRAP_OR`/panic surface (owned by tn-8obr), no obligation-gate wiring (owned by tn-ce7x), no kani/mutants/moon-ci (owned by tn-4isu/tn-7bq2).

## Duplicates ruled out

- `tn-8obr` owns approved-recovery (unwrap_or family); `tn-ce7x` owns critical-gate blocking; `tn-9acl` owns seeded-defect QA. No overlap: this bead owns only the loop→bounded-execution detector cutover.
