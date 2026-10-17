# Boundary map — tn-4isu

Inside the bead boundary (may change):

- `crates/titania-core/src/kani.rs`: `#[kani::unwind(N)]` attributes on
  `kani_mutants_baseline_diff_zero_neg` and `outcome_wire_invariants` only.
  No assert/assume/cover/input-shape edits.

On the boundary (read, never change):

- `crates/titania-lanes/src/run_lane_kani.rs` (`PER_PACKAGE_TIMEOUT_SECS`,
  cgroup flags), `.moon/tasks/all.yml` (`titania-kani` task), production
  constructors in `proof_id.rs` / `mutants_baseline.rs`.

Outside (forbidden):

- Production behavior, error enums, serde impls, lane logic, moon tasks,
  mutants baseline/profile, CI wiring, resource caps, other crates'
  (nonexistent) harnesses.
