# Baseline Report — tn-bvv7

Worktree: isolated subagent checkout at 927893f (origin/main).
Bead scope: enforce pure-core effect boundaries (aliases, re-exports,
macros, dependencies; process, async, ambient-state families).

Pre-change enforcement (all verified by reading source, not memory):

- `crates/titania-lanes/src/ast_grep_lane/rules/detectors.rs`: four
  `ARCHITECTURE_IMPORT_CORE_*` detectors matched `use` lines only.
  Fully-qualified paths with no import (`std::fs::read_to_string`,
  `tokio::spawn`, `std::time::SystemTime::now`, `rand::thread_rng`)
  did not fire. Proven by 4 failing-first tests (see
  `evidence/failing-first-detectors.txt`: 5 passed, 4 failed before the
  fix; 16 passed after).
- `crates/titania-lanes/rules/architecture.yml`: 4 architecture rule
  documents (INFRA, FS, TIME, RANDOM), `use`-anchored patterns only.
- `crates/titania-core/src/lib.rs`: pure domain types, zero IO; the only
  `std::sync` use is `LazyLock` memoization of the frozen repair catalog
  (`finding/repair_catalog.rs:28`), and the only file macro is
  `include_str!` of the frozen TSV — both explicitly allowed by this bead.
- `crates/titania-core/Cargo.toml`: no dependency on tokio, axum, sqlx,
  reqwest, rand, futures, async-std, or smol (contract test pins this).
- `titania-core` self-scan: zero occurrences of any newly banned marker
  in `crates/titania-core/src/` (checked 2026-10-16; `use std::io;` in
  `error.rs` and `use std::sync::LazyLock;` remain allowed).

Historical context: tn-ijmw and tn-6u7d core IO migrations are prior
fixes, not current behavior; tn-6r36 (contract) and tn-pfp1 (resolved
method identities) are closed dependencies. Planning notes are intent,
not evidence.
