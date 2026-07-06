# Implementation Notes - tn-4rq.2

## Production Behavior Implemented

### `titania-output::doctor`
- Added doctor domain types:
  - `DoctorStatus::{Ok, MissingRequiredTools}`
  - `ToolRow { name, required, installed, version, path }`
  - `DoctorReport { scope, tools, missing_required, status }`
- Added scope-based tool matrix via `tool_configs_for_scope(scope)`.
- Added PATH probing for external tools (`cargo`, `rustfmt`, `clippy-driver`, `rg`, `cargo-dylint`, `cargo-deny`, `sccache`).
- Added `probe_dylint_library(required, cargo_dylint_path)` for co-located `libtitania_dylint` detection.
- Added dynamic-library object-header guard before Dylint ABI marker scanning; arbitrary text containing marker names is rejected as `abi:mismatch`.
- Added `doctor_report(scope) -> Result<DoctorReport, OutputError>` for output-component availability failures.
- Kept `report(scope) -> DoctorReport` as infallible pure report construction for normal probing.

### `titania-check::doctor`
- Added `render(scope, emit)` returning `Result<CliDisposition, OutputError>`.
- Added human table output with columns: `Tool`, `Required`, `Installed`, `Version`, `Path`, `Status`.
- Added JSON output using typed serializable mirror rows.
- Removed unsupported JSON `embedded` field; embedded ast-grep remains represented as installed with null external version/path.
- Mapped doctor statuses to exit codes:
  - OK -> 0
  - MissingRequiredTools -> 3
  - output component unavailable -> internal error
- Wired `doctor_scope` dispatch through `main.rs`.

### Tests
- Replaced stub doctor dispatch tests with real CLI dispatch tests.
- Added empty-PATH edit-scope test for required tool detection.
- Added Dylint ABI mismatch test with a real PATH fixture and fake marker library.
- Added no-`nm` dependency test proving marker fixture can satisfy the ABI probe without an external symbol tool.
- Added JSON contract tests asserting absence of `embedded` field and expected required/installed/null path/version semantics.

## Verification Evidence
- `cargo fmt --all -- --check` -> PASS.
- `cargo check -p titania-check -p titania-output --all-targets` -> PASS.
- `cargo test -p titania-output doctor` -> PASS.
- `cargo test -p titania-check --test doctor` -> PASS, 9 tests.
- `cargo clippy --workspace --lib --bins --examples --all-features -- ... strict source gate` -> PASS.
- `cargo test --workspace --all-features` -> PASS, 651 tests / 97 suites.
- `cargo machete` -> PASS.
- `moon ci --force --summary normal` -> PASS.
- `cargo run --quiet -p titania-check -- doctor --scope edit --emit json` -> PASS and emits parsable JSON.

## Supply-chain Gate
Cargo-vet metadata now contains imported audit sources and local delta audit entries for 9 dependencies, but `cargo vet` still exits 255. Current residuals are 6 inspect-only crates:
- `aho-corasick 1.1.4`
- `dylint_internal 6.0.1`
- `dylint_linting 6.0.1`
- `paste 1.0.15`
- `regex 1.12.4`
- `regex-automata 0.4.14`

No broad unreviewed trust or `--accept-all` is accepted. Closure is blocked until those reports are approved and metadata is updated, or an explicit waiver is recorded.
