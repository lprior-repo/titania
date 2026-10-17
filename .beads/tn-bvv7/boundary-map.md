# Boundary Map — tn-bvv7

- **Pure core** — `titania-core` domain types plus the architecture
  detectors themselves (total `&str -> bool` predicates). Deterministic,
  no I/O, no clock, no async. Time/entropy enter core only as explicit
  injected values at the shell edge.
- **Imperative shell** — lane runner (file reads, `AstEngine` parsing,
  outcome emission), Moon tasks, report writers. `std::fs`, `tokio`,
  process spawning, and clocks live here behind typed ports.
- **Parser boundary** — YAML catalog and fixture text are parsed once
  into `RuleDef`/typed outcomes; malformed input is an error, never a
  finding and never a panic.
- **Explicitly allowed in core** — `include_str!` of frozen catalog
  input and `std::sync::LazyLock` write-once memoization (immutable
  after init). Banning them would flag the existing repair catalog, so
  the STATE detector lists only genuinely ambient state (`static mut`,
  `Mutex`/`RwLock`, `Cell`/`RefCell`/`UnsafeCell`, thread control).
- **Out of scope** — resolved-identity HIR effect lint (future dylint
  family; recorded as residual, not claimed).
