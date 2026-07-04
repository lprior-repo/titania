# Test Review Report: titania-lanes

**Bead:** tn-ily.2
**Session:** pristine-pass-b
**Scope:** All test files in `crates/titania-lanes/` — integration tests, `#[cfg(test)]` modules, test harnesses
**Excluded:** `titania-core/tests/` (tech-debt-audit), `titania-core/src/` (tn-03d-core-domain)

---

## Test Files Scanned

### Integration tests (`tests/`)
| File | Test Count |
|------|-----------|
| `verify_verus_public_api.rs` | 8 |
| `v1_config_contract.rs` | 3 |
| `rust_verification_gauntlet_target.rs` | 1 |
| `run_cargo_public_api.rs` | 12 |
| `guard_api_regressions.rs` | 4 |
| `command_public_api.rs` | 10 |
| `bdd_target_project.rs` | 5 |
| `scanner_target_project.rs` | 11 |
| `kani_list_public_api.rs` | 3 (plus 1 `#[cfg(unix)]`-gated) |
| `toolchain_config.rs` | 3 |
| **Subtotal** | **60** |

### `#[cfg(test)]` modules (`src/`)
| File | Test Count |
|------|-----------|
| `lib.rs` | 1 (LaneExit) |
| `source_line.rs` | 5 (SourceLine parser) |
| `bin/check_source_length.rs` | 2 |
| `bin/fuzz_minimization.rs` | 1 |
| `bin/check_test_integrity/mod.rs` | 1 (VCS fixture) |
| `bin/check_nightly_features/tests.rs` | 6 |
| `bin/check_workspace_assertions/mod.rs` | 1 |
| `bin/check_workspace_assertions/toml_scan.rs` | 6 |
| `bin/forbidden_scan/tests.rs` | 8 |
| **Subtotal** | **31** |

**Total test count: 91**

---

## Assertion Strength

### PASSED — No `is_ok()`-only assertions

Every assertion in the test suite checks a specific value, string, or structural invariant. No test relies solely on `result.is_ok()` or `result.unwrap()` without further verification.

Examples of strong assertions:
- `command_public_api.rs:84-91`: Matches `LaneError::NonZeroExit` and checks `program`, `code`, and `stderr` fields exactly
- `command_public_api.rs:164-167`: Checks `timeout_ms` field is exactly `20`
- `verify_verus_public_api.rs:115-123`: Checks exit code `1`, stderr contains `VERUS-TARGET-001`, AND summary file contains both expected and unexpected strings
- `scanner_target_project.rs:128`: Negative assertion — confirms inner `cfg(test)` assert is NOT reported
- `toolchain_config.rs:157-158`: Checks exact error string `"rust-toolchain.toml channel stable != nightly-2026-04-27"`
- `v1_config_contract.rs:64-68`: Mutates the Cargo.toml to weaken a lint and checks the weakened value is `allow` not `deny`
- `forbidden_scan/tests.rs:53-58`: Two negative assertions — `myexpect()` and `myexpect` are not matched

### FINDING: toolchain_config.rs — `.unwrap()` in test helpers (LOW)

```
crates/titania-lanes/tests/toolchain_config.rs:152: LOW: .unwrap() on parse_rust_toolchain result.
crates/titania-lanes/tests/toolchain_config.rs:155: LOW: .unwrap() on parse_moon_rust result.
crates/titania-lanes/tests/toolchain_config.rs:166: LOW: .unwrap() on parse_rust_toolchain result.
crates/titania-lanes/tests/toolchain_config.rs:169: LOW: .unwrap() on parse_moon_rust result.
```

These are in test-only code with hardcoded valid input strings, so they are safe. However, for consistency with the crate's `#![deny(clippy::unwrap_used)]` ethos, replace with `expect("fixture data is valid")` or `?` with a `Result<(), String>` return.

---

## Determinism

### PASSED — No random or time-dependent test logic

No test uses `thread_rng()`, `chrono::now()`, `SystemTime::now()` for test assertions, `rand::`, or any other source of non-determinism.

All tests use:
- `tempfile::TempDir` for isolated filesystem fixtures (deterministic content)
- Inline string fixtures (deterministic content)
- `Command::new(env!("CARGO_BIN_EXE_*"))` for binary invocation (deterministic path)
- `assert_eq!` and `assert!` on exact values or string containment

### FINDING: scanner_target_project.rs — `SystemTime::now()` in scratch repo helper (LOW)

```
crates/titania-lanes/src/bin/check_test_integrity/self_test.rs:86: LOW: SystemTime::now() used to generate unique temp directory names for scratch git repos.
```

This is test fixture setup code, not test assertion logic. The timestamp is used only to avoid directory name collisions when the self-test is invoked multiple times in the same session. It does not affect test assertions or outcomes. This is acceptable.

### FINDING: command_public_api.rs — timeout test with `sleep 2` / 20ms (LOW)

```
crates/titania-lanes/tests/command_public_api.rs:156: LOW: Test uses `sleep 2` with a 20ms timeout. On extremely slow CI, the OS scheduler may delay the timeout, though the assertion only checks the configured `timeout_ms` value (20), not the measured elapsed time.
```

The test asserts `timeout_ms == 20` (the configured budget, not a measured wall-clock value), so it is functionally deterministic. The `sleep 2` just needs to exceed 20ms, which is trivially true on any system.

---

## Mutation Resistance

### PASSED — Extensive negative assertions present

Tests include negative assertions that would catch mutations (false positives, missed detections, wrong error variants):

| Test | Negative Assertion | Line |
|------|-------------------|------|
| `scanner_target_project.rs` | `cfg(test)` internals do NOT leak to violations | 128 |
| `scanner_target_project.rs` | inner `assert!` inside `cfg(test)` NOT reported | 153 |
| `scanner_target_project.rs` | user identifiers `myexpect()`, `myunwrap()` NOT flagged | 236 |
| `scanner_target_project.rs` | block comment `assert!` NOT flagged | 268 |
| `scanner_target_project.rs` | string literal `assert!` NOT flagged | 282 |
| `verify_verus_public_api.rs` | `VERUS_REGISTRY_OK` absent from failure summary | 123, 191, 217, 268 |
| `guard_api_regressions.rs` | rustup log file NOT created for non-vb packages | 140 |
| `command_public_api.rs` | spawn error carries exact `Io` variant with `NotFound` | 119 |
| `command_public_api.rs` | env remove actually removes the variable | 223 |
| `forbidden_scan/tests.rs` | `mypanic!` NOT matched by `panic!` token | 23 |
| `forbidden_scan/tests.rs` | `unwrap_or_default` NOT matched by `unwrap` token | 32 |
| `forbidden_scan/tests.rs` | bare identifier `unwrap` NOT matched | 65 |
| `forbidden_scan/tests.rs` | `x.unwrap` (no paren) NOT matched | 66 |
| `check_nightly_features/tests.rs` | `is_perf_scoped_path` rejects non-perf paths | 64 |
| `check_workspace_assertions/mod.rs` | `serde` NOT in forbidden list | 75 |
| `v1_config_contract.rs` | weakened lint value is `allow` not `deny` | 68 |

Tests cover both positive assertions (bug IS detected) and negative assertions (bug is NOT falsely detected), providing strong mutation resistance.

---

## Public API Coverage

### Covered
| Public Type | Covered By |
|-------------|-----------|
| `LaneExit` | `lib.rs#tests`, `verify_verus_public_api.rs`, `rust_verification_gauntlet_target.rs` |
| `LaneReport` | `check_source_length.rs#tests`, `forbidden_scan/tests.rs` |
| `Finding` | `check_source_length.rs#tests`, `forbidden_scan/tests.rs` (via LaneReport) |
| `CommandIn` | `command_public_api.rs` (10 tests covering new, run, run_capture, run_capture_raw, spawn, budget, env, env_remove, inherit_env, run_status_raw) |
| `CommandBudget` | `command_public_api.rs:157-161` (timeout, max_stdout, max_stderr) |
| `LaneError` | `command_public_api.rs` (EmptyProgram, InvalidProgram, Io, NonZeroExit, Timeout, OutputLimitExceeded, NonUtf8Output) |
| `OutputStream` | `command_public_api.rs:133-134, 199-202` (Stdout, Stderr) |
| `SourceLine` | `source_line.rs#tests` (5 tests: line comment, block comment, multi-line block, string blanking, escaped quote) |
| `TargetProject` | `bdd_target_project.rs`, `command_public_api.rs`, `scanner_target_project.rs` |
| `EnvPolicy` | `command_public_api.rs:65-74` (Clear vs Inherit behavior) |
| `CommandOutput` | `command_public_api.rs` (stdout_str, stderr_str, success via exit code checks) |

### FINDING: helpers.rs — no dedicated tests (MEDIUM)

```
crates/titania-lanes/src/helpers.rs: MEDIUM: 11 public items without direct test coverage.
```

Public items in `helpers.rs` with no dedicated `#[cfg(test)]` module:

| Item | Line | Notes |
|------|------|-------|
| `line_no_from_idx` | 22 | Converts usize index to u32 line number |
| `saturating_add_usize` | 30 | Checked add returning usize::MAX on overflow |
| `brace_delta` | 35 | Counts net brace balance in text |
| `strip_leading_whitespace` | 48 | trim_start wrapper |
| `strip_whitespace` | 53 | trim wrapper |
| `normalize_slashes` | 58 | Path backslash-to-forward-slash |
| `relative_path` | 63 | Strips root prefix, normalizes slashes |
| `walk_rs_files` | 79 | Recursive .rs file walker |
| `LineNo` struct | 82 | u32 wrapper with Debug, Clone, Copy, Ord |
| `line_diff` | 96 | `saturating_sub` wrapper |
| `for_each_byte` | 109 | Byte-level iterator with early-exit via bool |

These are simple pure functions. The most critical ones to test are `saturating_add_usize` (overflow edge case) and `brace_delta` (balanced/imbanced brace counting).

### FINDING: `LineNo` ordering not tested (LOW)

```
crates/titania-lanes/src/helpers.rs:82: LOW: `LineNo` derives `Ord` but ordering is never tested.
```

The derive is straightforward (u32 comparison), but explicit tests would catch accidental derive changes.

### FINDING: `CurrentTargetError` not directly tested (LOW)

```
crates/titania-lanes/src/lib.rs:40: LOW: `CurrentTargetError` variants (CurrentDir, Target) not tested directly.
```

The error types are exercised through `discover_target` in integration tests, but the specific `CurrentDir` error path (unreadable CWD) and `Target` error path (no Cargo.toml) are not tested as direct unit assertions.

### FINDING: `CommandOutput::into_result()` not directly tested (LOW)

```
crates/titania-lanes/src/command/output.rs:57: LOW: `into_result()` method not tested directly.
```

The method is exercised indirectly through `CommandIn::run()`, but the specific error conversion (NonZeroExit with stderr capture) is not tested as a standalone unit assertion.

---

## Summary

| Category | Status | Notes |
|----------|--------|-------|
| Assertion strength | **PASSED** | No `is_ok()`-only assertions; all assertions check specific values |
| Determinism | **PASSED** | No thread_rng/chrono/OS-time in assertions |
| Mutation resistance | **PASSED** | Extensive negative assertions (14+) catching false positives |
| Public API coverage | **NEEDS WORK** | helpers.rs (11 items) has no tests; LineNo/CurrentTargetError/into_result not directly tested |

### Findings by Severity

| Count | Severity | Description |
|-------|----------|-------------|
| 0 | CRITICAL | None |
| 1 | HIGH | helpers.rs has 11 public items with no dedicated test coverage |
| 1 | LOW | toolchain_config.rs uses `.unwrap()` in test code (4 instances) |
| 1 | LOW | SystemTime::now() in test fixture setup (self_test.rs) |
| 1 | LOW | Timeout test uses `sleep 2` (20ms budget) — acceptable but fragile on extreme CI |
| 1 | LOW | `LineNo` ordering not tested |
| 1 | LOW | `CurrentTargetError` variants not directly tested |
| 1 | LOW | `CommandOutput::into_result()` not directly tested |
| 2 | LOW | `exit()` wrapper function not directly tested — only as_u8() checked, not ExitCode::from (confirmed with black-hat lane) |
| 1 | LOW | SourceLine raw string handling gap — no test passes r#"..."# or r##"..."## as input to the parser (confirmed with black-hat lane) |
| 1 | LOW | `LaneReport::push` public mutable not used as test helper — all tests invoke lane functions which push internally (confirmed with black-hat lane) |

### Recommendations

1. **HIGH**: Add a `#[cfg(test)]` module to `helpers.rs` with tests for at minimum `saturating_add_usize` (overflow at usize::MAX), `brace_delta` (balanced, imbalanced, empty text), and `relative_path` (prefix match, no-prefix match).

2. **LOW**: Replace `.unwrap()` calls in `toolchain_config.rs` tests with `expect("fixture data is valid")` or `?` for consistency with crate conventions.

3. **LOW**: Add a direct unit test for `LineNo` ordering and `CommandOutput::into_result()`.

4. **LOW**: Add a direct unit test for `exit()` that calls `exit(LaneExit::Clean)` and `exit(LaneExit::NotApplicable)` and asserts on the resulting `ExitCode` value, not just `as_u8()`.

5. **LOW**: Add a `SourceLine` parser test that passes raw string literal code (e.g. `r#"assert!(x)"#`) as input to verify correct handling — currently tests only use regular string literals or `r#...#` as test harness syntax.
