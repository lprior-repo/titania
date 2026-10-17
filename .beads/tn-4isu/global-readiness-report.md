# Global readiness — tn-4isu (State 1)

Revision: `927893f922fe2b9c658a0c1574086afe7d6a6dc6`.

## Scope exclusions (deferred global, not passes)

- `moon ci` (canonical gate): NOT RUN — explicitly out of scope per delegating task.
  Carried as accepted-scope-exclusion; `tn-7bq2` Full readiness stays open.
- Mutants lanes (`cargo mutants`, baseline diff execution): NOT RUN — out of scope.
  Mutation readiness stays a separately measured dependency.
- Full-workspace `cargo kani`: NEVER RUN (resource governance). Only per-package
  `cargo kani -p titania-core -j 1` inside the mandated cgroup.

## Targeted gates (mandatory, State 12)

- `cargo fmt --check` (exit required 0).
- `cargo check --workspace --all-targets --all-features` (exit required 0).
- Source-only strict clippy per AGENTS.md (exit required 0).
- `cargo test` for touched crates, fully green (exit required 0).
- Per-harness `cargo kani -p titania-core -j 1 --harness <name>` within cgroup caps,
  each with exact command + log + exit status (see `verification-ledger.jsonl`).

## Tool availability

- `cargo-kani 0.67.0` present (≥ floor 0.50.0). `systemd-run --user` available.
- No missing-tool blocker at State 1. If Kani sysroot setup fails at runtime, it
  will be recorded as an explicit blocker, never a pass.
