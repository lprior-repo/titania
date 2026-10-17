# Machine gate report — tn-vr2n (formal-verifier, state 12)

Worktree: `/home/lewis/src/titania/.muse/worktrees/subagent-v2-01a14623-5a8f-76e0-be4a-be4eb153aaf7-01a146d6-7fd4-7c43-85a2-6055a7e3cfce`
Toolchain: cargo 1.97.0-nightly (eb9b60f1f 2026-04-24),
rustc 1.97.0-nightly (ca9a134e0 2026-04-26).
Raw logs: `.beads/tn-vr2n/logs/`.

| Gate | Command | Exit | Log |
|---|---|---|---|
| fmt | `cargo fmt --all -- --check` | 0 | (clean tree state; rerun at close) |
| check | `cargo check --workspace --all-targets --all-features` | 0 | `gate-check.log` |
| clippy (source-only strict, AGENTS.md list) | `cargo clippy --workspace --lib --bins --examples --all-features -- -D ...` | 0 | `gate-clippy.log` |
| test titania-lanes (full, all features) | `cargo test -p titania-lanes --all-features` | 0 | `gate-test-lanes.log` |
| test titania-check cli_dispatch | `cargo test -p titania-check --test cli_dispatch` (56 tests incl. new ceiling test) | 0 | in `gate-test-check.log` (suite line ok) |
| doc | `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps` | 0 | `gate-doc.log` |
| failing-before | `cargo test -p titania-lanes --test run_cargo_invocation` pre-impl | 101 (E0432, expected) | `gate1-failing-before.log` |

Excluded by delegation (not evidence, residuals): `moon ci`, kani,
mutants. Pre-existing environmental failure: `titania-check
--test template_prepush` (2 smoke tests) fails identically on the
untouched base (log `base-template-prepush.log`, exit 101) — classified
BLOCK_GLOBAL, unrelated to this diff (fails in moon tool-install phase
before any lane code runs).
