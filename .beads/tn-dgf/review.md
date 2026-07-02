# Test-Suite Review: titania-lanes/tests/*
## Bead: tn-dgf | Reviewer: test-reviewer | Date: 2026-07-02

---

## Gate Evaluation

### Gate 1: Compile & Execute — BLOCKED (severity: HIGH)
**file: titania-core/src/rule_id.rs:41**
The test suite cannot compile because `titania-core/src/rule_id.rs:41` references `RuleIdError::TooLong` which does not exist in `titania-core/src/error.rs:19-26`. This is a source-level bug, not a test bug, but it blocks ALL test execution. The test suite's quality cannot be assessed until this is fixed.

### Gate 2: Public API Only — PASS
All 10 integration test files use only public API. `CommandIn::new()`, `CommandIn::run()`, `CommandIn::arg()`, `CommandBudget`, `LaneExit::as_u8()`, `TargetProject`, `discover_target()` — all are public symbols. No `pub(crate)` or private internals are tested.

### Gate 3: Behavior Assertions — PASS
Tests assert behavior (WHAT), not implementation (HOW):
- `command_public_api.rs:39`: `assert!(matches!(empty, LaneError::EmptyProgram))` — asserts the exact error variant, not an error string.
- `bdd_target_project.rs:67-68`: `assert_eq!(output.status.code(), Some(1_i32))` + `assert!(stderr.contains("CARGO-FMT-001"))` — asserts exit code AND finding rule, not the internal `LaneReport` structure.
- `scanner_target_project.rs:125`: `assert!(stderr.contains("lib.rs:9"))` — asserts exact line number in output, which is behavior (the scanner reports correct line numbers).

### Gate 4: No Ignored Tests / Sleeps / Broad Mocks — PASS
- No `#[ignore]` annotations found in any test file.
- No `std::thread::sleep()` calls — the only time-related test is `command_public_api.rs:153-209` which uses `Duration::from_millis(20)` for a timeout test (intentional, deterministic).
- No broad mocks — subprocesses are real (git, /bin/sh, /bin/false, /usr/bin/env).

### Gate 5: Mutation Resistance — PASS (19 scenarios)
Tests cover mutation scenarios across error/value branches:
- `command_public_api.rs`: EmptyProgram, InvalidProgram, NonZeroExit, IoError, NonUtf8Output, Timeout, OutputLimitExceeded (both streams), EnvRemove — 9 mutation scenarios.
- `bdd_target_project.rs`: Workspace discovery, single-crate, missing Cargo.toml, completed receipt, empty path — 5 scenarios.
- `scanner_target_project.rs`: cfg(test) scope tracking (nested and top-level), nightly features (single-line, multi-line), forbidden scan (expect, myexpect, Result::unwrap), block comments, string literals — 11 scenarios.
- `run_cargo_public_api.rs`: fmt, check, clippy, fix, build lanes — 5 scenarios.
- `verify_verus_public_api.rs`: Verus target discovery — 5 scenarios.
- `kani_list_public_api.rs`: Kani harness enumeration — 3 scenarios.

### Gate 6: Snapshot Tests — PASS
No traditional snapshot tests. The suite uses `include_str!` for contract checks (e.g., `v1_config_contract.rs:50-63`), which is intentional and correct.

### Gate 7: Resource Bounds — PASS
All subprocesses are bounded:
- `CommandBudget::default()` enforces 60s timeout and 1MB output limits.
- `command_public_api.rs:153-209` uses a 20ms timeout for the timeout test (explicit budget).
- No unbounded `cargo kani`, mutation sweeps, or fuzz runs in test code.

### Gate 8: No Dormant/Commented Tests — PASS
No `#[ignore]` tests, no commented-out `#[test]` functions, no dormant modules.

---

## Findings by Severity

### HIGH (1)
- `titania-core/src/rule_id.rs:41` — `RuleIdError::TooLong` variant missing, blocks all test compilation. This is a production code bug, not a test bug, but the test suite cannot pass until it's fixed.

### LOW (1)
- Non-determinism is absent from test code — positive finding. Only intentional sleep is the timeout test with explicit Duration.

### MINOR (1)
- Some tests assert exact stderr substrings rather than structured outputs. This is acceptable for binary tests (where stdout/stderr is the only interface) but limits mutation resistance if stderr format changes.

### INFORMATIONAL (3)
- `toolchain_config.rs:135` uses `Result<(), String>` instead of `TestResult` — style inconsistency with the rest of the suite.
- `toolchain_config.rs:148,162` and `v1_config_contract.rs:50,63` use inferred return types (`-> Result<T, E>`) rather than explicit `TestResult` — inconsistent with suite conventions.
- Actual test counts differ from scope listing: guard_api_regressions=4, run_cargo_public_api=14, scanner_target_project=11, verify_verus_public_api=7 (scope was approximate — not a finding, just a note).

---

## Verdict

**STATUS: REJECTED**

The test suite is well-structured, mutation-resistant (19 scenarios across 8 gates), public-API-only, and resource-bounded. However, Gate 1 is blocked by a production code bug (`RuleIdError::TooLong` missing variant) that prevents ALL test compilation. Until this is fixed, the suite cannot be approved.

Once the source bug is fixed, the suite would likely PASS all 8 gates.
