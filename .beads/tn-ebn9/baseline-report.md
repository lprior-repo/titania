# Baseline Report — tn-ebn9

Base commit `927893f922fe2b9c658a0c1574086afe7d6a6dc6` (isolated worktree,
clean at claim; `git status` showed no modifications).

Pre-change enforcement behavior (the blanket-ban baseline this bead removes):

- `crates/titania-dylint/src/functional_surface.rs`: `FUNC_LOOPS_FOR`,
  `FUNC_LOOPS_WHILE`, `FUNC_LOOPS_LOOP` reject every `for`/`while`/`loop`
  outside `#[cfg(test)]`; `method_policy` matches unwrap spelling only.
- `crates/titania-lanes/src/ast_grep_lane/{rules.rs,engine.rs}` +
  `rules/functional.yml`: same three bans as Reject/UseIteratorPipeline;
  first match outside `#[cfg(test)]` ranges.
- `fixtures/strict_ai_loop_unwrap/bad`: `for` + `.unwrap()` rejects with
  `FUNC_LOOPS_FOR` + `CLIPPY_UNWRAP_USED`; `repaired` (`flatten`, no loop)
  passes. `cargo test -p titania-check --test killer_demo`: 16/16 on base
  (pre-change count; post-change suite has 17 with the new positive control).
- Toolchain: `rustc 1.97.0-nightly (ca9a134e0 2026-04-26)`,
  `cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)`, `moon 2.2.4`.
- Environment notes: `/tmp` has a disk quota that breaks compiler/rustdoc
  tempfile writes; all verification below runs with
  `TMPDIR=/dev/shm/tn-ebn9-tmp` and `RUSTC_WRAPPER=""` (sccache bypass).
  See `global-readiness-report.md` for pre-existing failures.
