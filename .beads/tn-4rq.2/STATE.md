# Bead tn-4rq.2 - STATE

**Status:** implementation verified locally; closure blocked by mandatory cargo-vet supply-chain audit backlog.
**Owner:** Lewis
**Type:** task
**Priority:** P0

## Scope
output: implement doctor tool/version report and embedded ast-grep row

## Files Changed
- `crates/titania-output/src/doctor.rs`
- `crates/titania-check/src/doctor.rs`
- `crates/titania-check/tests/doctor.rs`
- `crates/titania-check/Cargo.toml` (removed unused `serde` after JSON renderer refactor)
- `supply-chain/audits.toml`, `supply-chain/config.toml`, `supply-chain/imports.lock` (cargo-vet metadata repair remains incomplete; 6 residual crates still unvetted)

## Implementation State
- Doctor report domain model implemented in `titania-output`.
- CLI doctor renderer implemented in `titania-check`.
- JSON output no longer exposes unsupported `embedded` field.
- `doctor_report(scope)` returns `Result<DoctorReport, OutputError>` and validates compiled output component availability.
- `report(scope)` remains the infallible pure report builder.
- Dylint ABI probe checks actual dynamic-library object headers before marker scanning.
- Empty `PATH` edit-scope behavior keeps cargo/rustfmt/clippy-driver/rg/cargo-dylint required and treats `libtitania_dylint` as informational when `cargo-dylint` is absent.
- No fake JSON fallback, no `#[expect]` bypass attributes, no production unwrap/expect/panic introduced.

## Verified Evidence
- `cargo fmt --all -- --check`: PASS.
- `cargo check -p titania-check -p titania-output --all-targets`: PASS.
- `cargo test -p titania-output doctor`: PASS, 2 suites / 6 filtered.
- `cargo test -p titania-check --test doctor`: PASS, 9 tests.
- `cargo clippy --workspace --lib --bins --examples --all-features -- ... strict source gate`: PASS.
- `cargo test --workspace --all-features`: PASS, 651 tests / 97 suites.
- `cargo machete`: PASS, no unused dependencies.
- `moon ci --force --summary normal`: PASS in `v1-combined-dispatch` worktree.
- Manual JSON smoke: `cargo run --quiet -p titania-check -- doctor --scope edit --emit json`: PASS, parsable JSON with expected tool rows.

## Blocking Gate
`cargo vet` exits 255. Current residual safe-to-deploy gaps:
- `aho-corasick 1.1.4`
- `dylint_internal 6.0.1`
- `dylint_linting 6.0.1`
- `paste 1.0.15`
- `regex 1.12.4`
- `regex-automata 0.4.14`

Audit backlog reported by cargo-vet: 115882 lines. Dedicated inspectors are running before any further certify/trust action.

## Closure State
Do not close `tn-4rq.2` or parent `tn-4rq` until `cargo vet` exits 0 or an explicit accepted waiver records the remaining risk.
