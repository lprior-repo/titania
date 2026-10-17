# Baseline report — tn-vr2n

Base commit: 927893f922fe2b9c658a0c1574086afe7d6a6dc6 (worktree clean at start;
`git status --porcelain` empty).
Toolchain: pinned by rust-toolchain.toml (verified at gate time with
`cargo --version` / `rustc --version`, recorded in verification-ledger).

## Pre-change baseline (existing suites covering adjacent channels)

All expected GREEN before the change (proves adjacent channels already held;
the gap is the unvalidated `extra_args` chokepoint, which has no tests):

- `cargo test -p titania-lanes --test policy_scan_env_vars`
- `cargo test -p titania-lanes --test policy_scan_cargo_config`
- `cargo test -p titania-lanes --test command_env_scrub`

Raw logs: `.beads/tn-vr2n/logs/baseline-*.log`.

## Gap reproduction (Gate 1, failing-before)

New suite `crates/titania-lanes/tests/run_cargo_invocation.rs` written
against the planned public API
`titania_lanes::policy_scan::invocation::validate_extra_args`, plus a
`run_cargo_lane` wiring unit test against the planned
`RunCargoError::Policy` variant. Before implementation these fail
(module/variant do not exist). Raw log:
`.beads/tn-vr2n/logs/gate1-failing-before.log`.
