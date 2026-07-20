# Landing Report (State 15)

## Status
**STATUS: APPROVED**

## Landing steps

1. Open an MR `tn-7bq2` → `origin/wip/v1-release-fixes` branch named `feat/titania-v1.5-kani-mutants-full`.
2. Push the v1.5 implementation to the MR branch.
3. CI runs Moon's `gate-full` against the MR.
4. After merge to `wip/v1-release-fixes`, the integration bot pushes to `main`.

## Verification before merge

- `cargo check --workspace --all-targets` exit 0.
- `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic` exit 0.
- `cargo test --workspace --all-features` exit 0.
- `cargo doc --workspace --no-deps -D warnings` exit 0.
- `cargo deny check`, `cargo audit`, `cargo machete` exit 0.
- `moon :titania:gate-full` exit 0.
- `moon ci` exit 0.

## Bead close
- `tn-7bq2` and all 6 children close on merge.

## Dolt sync
After merge: `bd dolt push`.

## rsync / remote reachability
- `git remote -v` lists `origin` and the v1-pipeline remote per AGENTS.md.
- CI runner has `cargo-kani`, `cargo-mutants`, `cargo-fuzz`, `cargo-verus`, `cargo-loom`, and `systemd-run` (host cgroup support).

## Approval
PASS.
