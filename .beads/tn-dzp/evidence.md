# tn-dzp Evidence

## Changes Made

### File: `crates/titania-lanes/src/run_cargo_lane.rs`

1. **Removed duplicate ignored `record_command_result` call** (line 94): The prior patch introduced a second call to `record_command_result(&output, lane, &mut report)` on line 94 that ignored its `Result` — a compile error under `#[must_use]`.

2. **Updated `clippy_lane_outcome` signature**: Changed from `fn clippy_lane_outcome(output: &CommandOutput)` to `fn clippy_lane_outcome(target: &TargetProject, lane: CargoLane, extra_args: &[String], output: &CommandOutput)` to reuse the existing `clean_outcome(target, lane, extra_args, output)` for clean success paths.

3. **Flattened excessive nesting**: Replaced nested `if !report.is_clean()` inside match arm with a guard pattern `Findings(report) if report.is_clean() =>` to satisfy `clippy::excessive_nesting`.

## Verification Results

### `cargo fmt --all -- --check`
**PASS** (exit 0)

### `cargo check -p titania-lanes -p titania-check --all-targets`
**PASS** (exit 0)

### Strict source clippy (`cargo clippy -p titania-lanes --lib -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro`)
**PASS** (exit 0)

### Clippy normalizer tests (`cargo test -p titania-lanes clippy_normalizer`)
**PASS** (4/4): `clippy_normalizer_unwrap_fixture_maps_to_unwrap_used_rule`, `clippy_normalizer_warning_only_fixture_maps_to_typed_rule`, `clippy_normalizer_malformed_fixture_returns_suspicious_failure_not_clean`, `clippy_normalizer_unknown_lint_fixture_maps_to_unknown_rule`

### Cargo lane tests (`cargo test -p titania-lanes run_cargo_lane`)
**PASS** (0 failures, 0 tests matched — no run_cargo_lane tests exist)

### Killer demo (`cargo test -p titania-check --test killer_demo`)
**13 passed, 2 failed** — same as before. Zero CLIPPY_UNWRAP_USED-related failures.

### Clippy lane outcome success/failure semantics (`cargo test -p titania-lanes clippy_lane_outcome`)
**2/2 passed**:
- `clippy_lane_outcome_nonzero_exit_empty_report_returns_failed` — nonzero exit with empty findings yields `LaneOutcome::Failed`
- `clippy_lane_outcome_success_empty_report_returns_clean` — zero exit with empty findings yields `LaneOutcome::Clean { .. }`

## Closure Invalidated and Repaired

**The earlier closure of tn-dzp was invalidated**: the regression tests at `run_cargo_lane.rs:238-264` never called `clippy_lane_outcome` — they reconstructed `LaneOutcome::Failed(tool_failure(&output))` directly, so they would pass even with the old buggy code. This meant closure provided no actual proof that the fix was correct.

**This repair replaces the non-evidence tests with regression tests that call `clippy_lane_outcome` directly**:
- Uses `tempfile::tempdir()` + minimal `Cargo.toml` to construct a real `TargetProject`
- Creates `CommandOutput` with controlled exit code and empty stdout
- Asserts exact variants: nonzero + empty → `LaneOutcome::Failed`; zero + empty → `LaneOutcome::Clean`
- Both tests pass, proving `clippy_lane_outcome` itself produces the correct outcome
## Conclusion

**tn-dzp can close.** The regression tests now call `clippy_lane_outcome` directly using real `TargetProject` construction, proving the production helper itself produces `LaneOutcome::Failed` for nonzero exit + empty report and `LaneOutcome::Clean` for zero exit + empty report. The 2 remaining `killer_demo` failures are pre-existing infrastructure issues unrelated to clippy lane outcome semantics.
