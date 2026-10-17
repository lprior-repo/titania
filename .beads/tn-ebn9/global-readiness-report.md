# Global Readiness Report — tn-ebn9

Targeted gates (this bead's mandatory set) are green; global lanes are
explicitly out of scope per parent direction (owned by tn-4isu/tn-7bq2) or
blocked by pre-existing environmental failures. No global gate is claimed
as passing.

Targeted (this bead, all with exit codes in `machine-gate-report.md`):

- `cargo fmt --all -- --check`: exit 0.
- `cargo check --workspace --all-targets --all-features`: exit 0.
- Source-only strict clippy (AGENTS.md invocation): exit 0, no diagnostics.
- `cargo test` touched crates: titania-lanes, titania-dylint, titania-core,
  titania-output fully green (855 passed, 0 failed across 63 suites);
  titania-check killer_demo 17/17, explain 9/9, doctor 15/15,
  emit_restriction 7/7, aggregate_cli 3/3 green.
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps`:
  exit 0.

Accepted blockers (NOT run / NOT passing; never counted as evidence):

- `moon ci` (full): not run per parent direction; owned by release lanes.
- Kani harnesses and cargo-mutants: not run per parent direction; owned by
  tn-4isu/tn-7bq2.
- `titania-check --test cli_dispatch`
  `dispatch_run_lane_test_successful_cargo_output_writes_clean_artifact`:
  FAILs identically on the base commit (verified in a clean
  `/dev/shm/base-check` worktree at `927893f`): nested `cargo test` doctest
  phase fails writing rustdoc argfiles under the `/tmp` disk quota.
  Pre-existing environmental failure, unrelated to this bead.
- `titania-check --test template_prepush` (2 tests): fail while installing
  `cargo-binstall` from crates.io (network installs unavailable in this
  sandbox). Template content (`titania/template/src/lib.rs`) is a 3-line
  stub with no policy surface, so the loop-policy cutover cannot affect
  it. Pre-existing environmental failure.
