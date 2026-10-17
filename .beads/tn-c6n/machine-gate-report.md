# Machine gate report — tn-c6n (State 12, formal-verifier)

Workdir for all commands: isolated worktree root.
`TMPDIR=/home/lewis/.cache/muse-tmp` (`/tmp` quota exhausted on host).

| Gate | Command | Exit | Log |
|---|---|---|---|
| fmt | `cargo fmt --all -- --check` | 0 | evidence/fmt-s12.log |
| check | `cargo check --workspace --all-targets --all-features` | 0 | evidence/check-s12.log |
| clippy (source-only strict per AGENTS.md) | `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::all -D clippy::cargo -D clippy::pedantic -D clippy::nursery -D clippy::unwrap_used -D clippy::expect_used -D clippy::unwrap_in_result -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::unreachable -D clippy::dbg_macro -D clippy::print_stdout -D clippy::print_stderr -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::integer_division -D clippy::integer_division_remainder_used -D clippy::let_underscore_must_use -D clippy::await_holding_lock -D clippy::future_not_send -D clippy::large_futures -D clippy::allow_attributes -D clippy::allow_attributes_without_reason -D clippy::disallowed_methods -D clippy::disallowed_macros -D clippy::disallowed_types -D clippy::disallowed_fields` | 0 | evidence/clippy-s12.log |
| test (touched crate, full) | `cargo test -p titania-core --all-features` | 0 | evidence/test-core-s12.log (30 binaries ok, 0 failed) |
| doc | `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps` | 0 | evidence/doc-s12.log |
| verus binding | `verus verification/verus/receipt_schema.rs` | 0 | evidence/verus-s12.log (`3 verified, 0 errors`) |

One fmt failure was met and repaired during this state (import ordering in
the two new test files; `cargo fmt` applied, re-check exit 0). No gate was
weakened, no warning allowed, no failure waived. Toolchain:
`rustc 1.97.0-nightly (ca9a134e0 2026-04-26)`, `verus 0.2026.05.05.d03e906`.
