# tn-zuv Evidence — Typed per-lane entry migration complete

## Changes Made

### 1. `crates/titania-core/src/report.rs`
- `Report::per_lane` fields use `Box<[PerLaneEntry]>` (already migrated by previous bead)
- `PerLaneEntry = { lane: Lane, outcome: LaneOutcome }` — already in place

### 2. `crates/titania-aggregate/src/report_assembly.rs`
- `assemble_report` signature updated: `outcomes: Box<[PerLaneEntry]>`
- Added `LaneIdentityMismatch` error variant to `ReportAssemblyError`
- Added `check_lane_identity()` function that validates:
  - No duplicate lanes in per_lane entries
- Uses `try_for_each` iterator pipeline (no imperative `for` loop), `zip` for position-aware comparison
- No `HashSet` — duplicates are caught by positional mismatch; count check remains after identity to preserve original error precedence
- `check_single_entry` helper removed (dead code after loop removal)
- `Lane` import removed from `titania_core` re-export (no longer needed)

### 3. `crates/titania-aggregate/tests/report_assembly.rs`
- Updated all 13 test helpers to return `PerLaneEntry` instead of bare `LaneOutcome`:
  - `clean_entry(lane)` → `PerLaneEntry { lane, outcome: LaneOutcome::Clean { ... } }`
  - `findings_entry(lane)` → `PerLaneEntry { lane, outcome: LaneOutcome::Findings { ... } }`
  - `informational_entry(lane)` → same pattern
  - `infra_failure_entry(lane, tool, reason)` → same pattern
  - `skipped_entry(lane)` → same pattern
- Updated all test calls to pass `Box<[PerLaneEntry]>` instead of `Box<[LaneOutcome]>`
- Fixed `is_pass()` call to use `entry.outcome.is_pass()`
- Added Test 12: `duplicate_lane_entries_rejected` — verifies `LaneIdentityMismatch` for duplicate lanes
- Added Test 13: `shuffled_lane_entries_rejected` — verifies `LaneIdentityMismatch` for out-of-order lanes

### 4. `crates/titania-core/tests/tn_03d_domain_model.rs`
- Removed unused variable `p` (line 351) and its dependent `receipt` variable (line 350)

### 5. `crates/titania-core/tests/json_roundtrip.rs`
- Updated golden JSON fixtures to use `outcome` wrapper in `per_lane` entries:
  - `REPORT_PASS_JSON`: `{"lane":"Fmt","outcome":{"variant":"clean",...}}`
  - `REPORT_REJECT_JSON`: same pattern
  - `EMPTY_REJECT_COLLECTIONS_JSON`: same pattern

### 6. `crates/titania-check/tests/cli_dispatch.rs`
- Updated variant access: `entry["outcome"]["variant"]` instead of `entry["variant"]`

### 7. `crates/titania-check/tests/killer_demo.rs`
- Updated variant access in per_lane iterations:
  - `outcome["outcome"]["variant"]` instead of `outcome["variant"]`

## Command Evidence

```
$ cargo check -p titania-core -p titania-aggregate -p titania-check --all-targets
OK

$ cargo test -p titania-core
169 passed (11 suites, 0.00s)

$ cargo test -p titania-aggregate
33 passed (5 suites, 0.00s)

$ cargo test -p titania-check --test aggregate_cli
3 passed (1 suite, 0.00s)

$ cargo test -p titania-check --test cli_dispatch
23 passed (1 suite, 1.23s)

$ cargo test -p titania-check --test killer_demo
15 passed (1 suite, 0.39s)

$ cargo fmt --all -- --check
OK
```

## No Forbidden Constructs Introduced
- No `unwrap()` added to production code
- No `expect()` added to production code
- `#[expect(clippy::excessive_nesting)]` removed from `check_lane_identity`; flattened via three helpers (`lane_identity_error`, `lane_identity_mismatch`, plus the main function) to keep nesting below clippy threshold — no `allow`/`expect` attributes remain in this file

## Final Controller Verification

- `cargo fmt --all -- --check` exited 0.
- `cargo check -p titania-core -p titania-aggregate -p titania-check --all-targets` exited 0.
- `cargo clippy -p titania-core -p titania-aggregate --lib --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic` exited 0.
- `cargo clippy -p titania-check --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic` exited 0 after removing redundant `per_lane.clone()` in `crates/titania-check/src/aggregate.rs`.
- `cargo test -p titania-core` passed 169 tests.
- `cargo test -p titania-aggregate` passed 33 tests.
- `cargo test -p titania-check --test aggregate_cli` passed 3 tests.
- `cargo test -p titania-check --test cli_dispatch` passed 23 tests.
- `cargo test -p titania-check --test killer_demo` passed 15 tests.

## Remaining Blockers
None.

## Can tn-zuv close? YES

All acceptance criteria met:
- `cargo fmt --all -- --check` exits 0
- `cargo check -p titania-aggregate -p titania-core -p titania-check --all-targets` exits 0
- `cargo clippy -p titania-aggregate --lib -- -D warnings` exits 0
- `cargo test -p titania-aggregate duplicate_lane_entries_rejected` — 1 passed
- `cargo test -p titania-aggregate shuffled_lane_entries_rejected` — 1 passed
- `cargo test -p titania-check --test killer_demo` — 15 passed
- No production `for` loop remains in `check_lane_identity` (replaced with `try_for_each` iterator pipeline)
- No `HashSet` used — duplicates caught by positional mismatch
- `check_single_entry` helper removed (dead code)
- All 33 titania-aggregate tests pass

### 8. `crates/titania-aggregate/src/report_assembly.rs` — `#[expect]` removal (tn-zuv follow-up)
- Removed `#[expect(clippy::excessive_nesting)]` from `check_lane_identity`
- Flattened nesting by extracting two helpers:
  - `lane_identity_error(expected, entry) -> Option<()>` — pure comparison, no nesting
  - `lane_identity_mismatch(scope, expected, entry) -> ReportAssemblyError` — error construction, no nesting
- `check_lane_identity` body: one `try_for_each` with `ok_or_else` calling a bare function — 2 levels, below clippy threshold
- Added `Lane` to `titania_core` imports (previously removed; now needed by helper signatures)
- No `#[allow]` or `#[expect]` remains anywhere in `report_assembly.rs`
- No production `for` loop in identity check
