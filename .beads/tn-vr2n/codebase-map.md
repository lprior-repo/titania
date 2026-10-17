# Codebase map — tn-vr2n (explore, state 2)

Scope: effective compiler invocation used by the protected Moon runner
(`titania-clippy` → `titania-check run-lane clippy`).

| Area | File | Role in this bead |
|---|---|---|
| Invocation chokepoint | `crates/titania-lanes/src/run_cargo_lane.rs` (`run_lane`, `cargo_output`) | Appends unchecked `extra_args` — THE GAP, enforcement lands here |
| Static argv | `crates/titania-lanes/src/run_cargo/args.rs` | Pinned §9.2 deny argv; negative test lands in its `tests` module |
| Outcome evidence | `crates/titania-lanes/src/run_cargo/outcome.rs` | Records effective argv (`argv_for_lane`); read-only for this bead |
| Env scanner | `crates/titania-lanes/src/policy_scan/env_vars.rs` | Already rejects RUSTFLAGS / encoded flags; regression lock only |
| Config scanner | `crates/titania-lanes/src/policy_scan/cargo_config.rs` | Already rejects all rustflags + non-sccache wrappers; regression lock only |
| Env scrub | `crates/titania-lanes/src/command/env_filter.rs`, `execution.rs` | Drops hostile env at spawn; regression lock only |
| CLI gate | `crates/titania-check/src/args/parse.rs` (`parse_run_lane`) | Rejects extra CLI args; regression lock only |
| Moon runner | `.moon/tasks/all.yml` (`titania-clippy`) | Invokes the chokepoint with no extra args |
| New validator | `crates/titania-lanes/src/policy_scan/invocation.rs` | Pure `validate_extra_args`; public for behavior tests |

Risks: none outside `titania-lanes` + one CLI test in `titania-check`.
No async, no unsafe, no new deps, no manifest changes.
