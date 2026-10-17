# Baseline report — tn-4isu (State 1)

Revision: `927893f922fe2b9c658a0c1574086afe7d6a6dc6` (worktree clean, `git status --short` empty).
Toolchain: `cargo-kani 0.67.0`, `rustc 1.97.0-nightly (ca9a134e0 2026-04-26)`.
Cgroup: `systemd-run --user` available.

## Baseline behavior (accepted ground truth, not re-executed)

- Full-scope `cargo kani -p titania-core` (all 12 harnesses, `-j 1`): timeout after
  600s (`PER_PACKAGE_TIMEOUT_SECS`), no counterexamples → 12/12 `PROOF_KANI_BLOCKED`.
- No production code change is proposed. The repair surface is `#[kani::unwind]`
  bounds on proof harnesses only (`crates/titania-core/src/kani.rs`, `cfg(kani)`).
- Normal-build behavior baseline: `cargo check` / `cargo test` for touched crates
  must stay green; recorded in State 12.

## Global readiness

- `moon ci` / mutants lanes: explicitly OUT OF SCOPE per delegating task
  (do NOT run). Recorded as accepted-scope-exclusion, not a pass.
- `cargo kani` present (0.67.0 ≥ floor 0.50.0). No missing-tool blocker.
- Memory: cgroup caps MemoryHigh=20G / MemoryMax=24G / MemorySwapMax=0 per run.
