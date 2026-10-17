# Implementation — tn-vr2n (holzman-rust, state 11)

Zero-panic Holzman discipline: no unwrap/expect/panic/todo, no indexing,
no casts, no arithmetic side effects; pure predicate + straight-line guard;
`cargo fmt` clean; source-only strict clippy exit 0. In-repo callers all
pass `&[]`, so no caller migration was needed.

## Source Coverage Matrix

| Contract clause | Production change | Behavior lock |
|---|---|---|
| C1 extra-arg ceiling rejected | `policy_scan/invocation.rs` (new): `is_cap_lints_flag`, `validate_extra_args`, `InvocationError::CapLints`; `run_cargo_lane.rs::run_lane` guard → `RunCargoError::Policy` → `LaneExit::Violations` | `run_cargo_invocation` (8 tests) + wiring unit test |
| C2 static argv ceiling-free | `run_cargo/mod.rs`: `Policy` variant docs; no argv change | `args.rs` negative unit test + exact-match pins |
| C3 established path locks | none (prove coverage) | env/config/scrub/CLI ceiling regressions |

## Files changed

- NEW `crates/titania-lanes/src/policy_scan/invocation.rs` (49 lines)
- NEW `crates/titania-lanes/tests/run_cargo_invocation.rs` (8 tests)
- `crates/titania-lanes/src/policy_scan.rs` (+1 mod line)
- `crates/titania-lanes/src/run_cargo/mod.rs` (+4: Policy variant)
- `crates/titania-lanes/src/run_cargo_lane.rs` (guard + exit/message arms + wiring test)
- `crates/titania-lanes/src/run_cargo/args.rs` (negative unit test)
- `crates/titania-lanes/tests/policy_scan_env_vars.rs` (+2 tests)
- `crates/titania-lanes/tests/policy_scan_cargo_config.rs` (+1 test)
- `crates/titania-lanes/tests/command_env_scrub.rs` (+1 test)
- `crates/titania-check/tests/cli_dispatch.rs` (+1 test)

No manifest, Moon, or scanner-rule changes. No gate weakened.
