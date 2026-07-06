# Evidence Bundle - tn-4rq.2

## Requirement Map

| Requirement | Source artifact / code | Evidence | Disposition |
| --- | --- | --- | --- |
| Doctor exposes required/optional tool matrix by scope | `crates/titania-output/src/doctor.rs`, `crates/titania-check/src/doctor.rs` | `cargo test -p titania-output doctor`; `cargo test -p titania-check --test doctor` | PASS |
| Doctor human output has Tool/Required/Installed/Version/Path/Status columns | `crates/titania-check/src/doctor.rs` | manual `cargo run --quiet -p titania-check -- doctor --scope edit` smoke | PASS |
| Doctor JSON output is parsable and scope-aware | `crates/titania-check/tests/doctor.rs` | `cargo test -p titania-check --test doctor`; manual `--emit json` smoke | PASS |
| Missing required tools produce status `MissingRequiredTools` and exit 3 | `crates/titania-check/tests/doctor.rs` | empty-PATH edit-scope test | PASS |
| Dylint ABI mismatch does not pass as installed | `crates/titania-output/src/doctor.rs`, `crates/titania-check/tests/doctor.rs` | `doctor_abi_mismatch_yields_missing_required_library` | PASS |
| Doctor does not depend on external `nm` path | `crates/titania-output/src/doctor.rs`, `crates/titania-check/tests/doctor.rs` | `doctor_abi_probe_no_nm_dependency` | PASS |
| Embedded ast-grep row has no external path/version contract | `crates/titania-check/tests/doctor.rs` | JSON contract assertions | PASS |
| Production source remains strict-lint clean | changed Rust sources | strict clippy source gate | PASS |
| Workspace behavior remains green | workspace | `cargo test --workspace --all-features`; `moon ci --force --summary normal` | PASS |
| Mandatory supply-chain vet gate | `supply-chain/audits.toml`, `supply-chain/config.toml`, `supply-chain/imports.lock` | `cargo vet` | BLOCKED |

## Accepted Commands Run

1. `cargo fmt --all -- --check`
   - Result: PASS.
2. `cargo check -p titania-check -p titania-output --all-targets`
   - Result: PASS.
3. `cargo test -p titania-output doctor`
   - Result: PASS, 2 suites / 6 filtered.
4. `cargo test -p titania-check --test doctor`
   - Result: PASS, 9 tests.
5. `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::all -D clippy::cargo -D clippy::pedantic -D clippy::nursery -D clippy::unwrap_used -D clippy::expect_used -D clippy::unwrap_in_result -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::unreachable -D clippy::dbg_macro -D clippy::print_stdout -D clippy::print_stderr -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::integer_division -D clippy::integer_division_remainder_used -D clippy::let_underscore_must_use -D clippy::await_holding_lock -D clippy::future_not_send -D clippy::large_futures -D clippy::allow_attributes -D clippy::allow_attributes_without_reason -D clippy::disallowed_methods -D clippy::disallowed_macros -D clippy::disallowed_types -D clippy::disallowed_fields`
   - Result: PASS.
6. `cargo test --workspace --all-features`
   - Result: PASS, 651 tests / 97 suites.
7. `cargo machete`
   - Result: PASS, no unused dependencies.
8. `moon ci --force --summary normal`
   - Result: PASS in `/home/lewis/src/titania/.worktrees/v1-combined-dispatch`.
9. `cargo run --quiet -p titania-check -- doctor --scope edit --emit json`
   - Result: PASS, parsable JSON.
10. `cargo vet`
   - Result: BLOCKED, exit 255.

## Cargo-vet Residuals

`cargo vet` reports 6 unvetted safe-to-deploy dependencies and an estimated 115882-line inspect backlog:

- `aho-corasick 1.1.4`
- `dylint_internal 6.0.1`
- `dylint_linting 6.0.1`
- `paste 1.0.15`
- `regex 1.12.4`
- `regex-automata 0.4.14`

## Decision

Implementation is locally verified, but the bead is not closed. Closure requires cargo-vet exit 0 or an explicit accepted waiver for the remaining safe-to-deploy gaps.
