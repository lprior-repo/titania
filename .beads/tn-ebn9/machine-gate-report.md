# tn-ebn9 Machine-Gate Report

All commands run in the isolated worktree
(`/home/lewis/src/titania/.muse/worktrees/subagent-v2-01a14623-5a8f-76e0-be4a-be4eb153aaf7-01a146d6-7d3f-7841-8063-c7cc56caf0b1`)
with `TMPDIR=/dev/shm/tn-ebn9-tmp`, `RUSTC_WRAPPER=""`
(env workaround for the `/tmp` disk quota; see global-readiness).

| Command | Exit |
|---|---|
| `cargo fmt --all -- --check` | 0 |
| `cargo check --workspace --all-targets --all-features` | 0 |
| source-only strict `cargo clippy` (AGENTS.md full `-D` list) | 0 |
| `cargo test -p titania-lanes -p titania-dylint -p titania-core -p titania-output --all-features` | 0 (855 passed, 0 failed, 63 suites) |
| `cargo test -p titania-check --test killer_demo --all-features` | 0 (17 passed) |
| `cargo test -p titania-check --test explain --all-features` | 0 (9 passed) |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps` | 0 |

Raw logs: `raw/cargo-test-lanes-dylint-core-output.log`,
`raw/cargo-test-check-full.log` (includes the two accepted environmental
failures below — also present on the base commit).

Not run per parent direction: `moon ci`, Kani lanes, cargo-mutants.
