# tn-ebn9 Regression Diff

Base: `927893f922fe2b9c658a0c1574086afe7d6a6dc6`. No commit/push/sync;
diff lives in the isolated worktree only.

Modified (24): VISION.md, v1-spec.md,
`fixtures/strict_ai_loop_unwrap/bad/src/lib.rs`,
`crates/titania-check/tests/{explain,killer_demo}.rs`,
`crates/titania-core/src/finding/{repair_catalog.rs,repair_catalog.tsv}`,
`crates/titania-dylint/src/functional_surface.rs`,
`crates/titania-dylint/tests/policy_surface.rs`,
`crates/titania-lanes/rules/functional.yml`,
`crates/titania-lanes/src/{ast_grep_lane/{engine,rules}.rs,run_lane_dylint.rs}`,
`crates/titania-lanes/tests/{ast_grep_catalog_dispatch,ast_grep_cfg_test_skip,ast_grep_functional,ast_grep_lane,ast_grep_span_accuracy}.rs`,
`crates/titania-output/src/explain.rs`,
`crates/titania-output/tests/explain.rs`.

Deleted (5 obsolete blanket-ban fixtures):
`func_loops_{for,while,loop}_violation.rs`, `for_indented_col4_violation.rs`,
`real_for_in_prod_and_cfg_test_module.rs`.

Added (8 fixtures): `allowed_bounded_for.rs`,
`allowed_unbounded_loop_in_cfg_{test_module,all_test_module}.rs`,
`real_unbounded_loop_in_prod_and_cfg_test_module.rs`,
`bounded_execution_unbounded_{loop,while,iterator}_violation.rs`,
`loop_indented_col4_violation.rs`.

No gate was weakened: every retired rejection still rejects (unbounded
shapes, new ids); every newly-permitted shape has a positive control.
`git status` delta is confined to the files above plus `.beads/tn-ebn9/`.
