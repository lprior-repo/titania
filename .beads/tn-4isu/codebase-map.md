# Codebase Map — tn-4isu

Files read (no edits at State 2):

- `crates/titania-core/src/kani.rs` (535 lines): all 12 `#[kani::proof]` harnesses.
- `crates/titania-lanes/src/run_lane_kani.rs` (per-package lane: inventory per
  crate, ONE `cargo kani -p <pkg>` per package, 600s wallclock cap, cgroup
  MemoryMax=24G/MemorySwapMax=0, 8 MiB pipe cap, verdict parsing).
- `crates/titania-core/src/proof_id.rs`: `KANI_HARNESS_ID_MAX_LEN = 96`,
  `check_khi` single byte scan (max 96 iterations).
- `crates/titania-core/src/mutants_baseline.rs`: `contains`/`diff` loops ≤ 8.
- `.moon/tasks/all.yml`: `titania-kani` task wiring (lane via `titania-check run-lane kani`).

Repair surface (harness-only, `cfg(kani)`; zero production-behavior change):

- `crates/titania-core/src/kani.rs` `#[kani::unwind(...)]` attributes only:
  harness 11 (`32 → 16` candidate), harness 12 (`64 → 24` candidate),
  harness 10 unchanged unless triage implicates it, harness 9 unchanged
  (bound 100 is already near-minimal for the 97-byte overflow scan).

Explicitly out of scope: production constructors, lane logic, timeout/cgroup
constants, moon tasks, mutants, `moon ci`.
