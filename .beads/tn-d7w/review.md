# Test-Suite Review: check_test_integrity binary
## Bead: tn-d7w | Reviewer: test-reviewer | Date: 2026-07-02

---

## Source Overview

| File | Lines | Role |
|---|---|---|
| check_test_integrity.rs | 31 | Main entry, arg parsing |
| mod.rs | 166 | Core logic, arg handling, 1 unit test |
| scan.rs | 255 | Diff scanning engine (17 detection functions) |
| self_test.rs | 92 | Fixture runner (2 fixtures, git-based) |
| vcs.rs | 191 | git/jj integration |
| **Total** | **735** | |

---

## Gate Evaluation

### Gate 1: Compile & Execute — PASS
The code compiles. The single unit test in `mod.rs:153-165` (`check_reports_untracked_new_behavior_tests`) is a proper `#[test]` function that creates a scratch git repo and exercises the `check()` function.

### Gate 2: Public API Only — PASS
This is a binary, not a library. All tests are `#[cfg(test)]` modules within the binary crate, using internal APIs directly. This is appropriate.

### Gate 3: Behavior Assertions — PASS (with caveats)
Tests assert behavior (exit codes, fixture outcomes):
- `mod.rs:163`: `assert_eq!(check(&target, "HEAD", Vcs::Git)?, 1_i32)` — asserts exit code for untracked ignored test.
- `self_test.rs:53-56, 67-69`: Fixtures assert exit codes for clean and untracked-ignored scenarios.

However, the test coverage is very thin (1 unit test + 2 fixtures for 735 lines of code).

### Gate 4: No Ignored Tests / Sleeps / Broad Mocks — PASS
No `#[ignore]` annotations. No `std::thread::sleep()` calls. No broad mocks — git operations use `CommandIn` which spawns real git processes.

### Gate 5: Mutation Resistance — FAIL (BLOCKER)
**13 of 17 detection paths lack test coverage.** Deleting the following functions would leave all tests green:

| Function | Lines | Impact if deleted |
|---|---|---|
| `deleted_file_findings` | mod.rs:81-93 | Deleted test file detection silently drops |
| `has_exact_assertion` | scan.rs:46-57 | Exact assertion detection (assert_eq!, assert_ne!, etc.) silently drops |
| `has_weak_assertion` | scan.rs:59-64 | Weak assertion detection silently drops |
| `has_compile_only` | scan.rs:88-93 | Compile-only replacement detection silently drops |
| `has_test_decl` | scan.rs:66-71 | Test declaration detection silently drops |
| `deleted_test_declarations` | scan.rs:204-227 | Count-based test deletion detection silently drops |
| `weakened_assertions` | scan.rs:229-254 | Assertion weakening detection silently drops |
| `is_test_path` | scan.rs:3-12 | Path classification silently drops |
| `is_module_test_path` | scan.rs:23-28 | Module test path detection silently drops |
| `is_src_tests_rs_path` | scan.rs:35-37 | tests.rs path detection silently drops |
| `is_src_tests_child_path` | scan.rs:39-44 | tests/ child path detection silently drops |
| `parse_git_name_status` | vcs.rs:132-146 | git ls-files --name-status parsing silently drops |
| `jj_changed_files` | vcs.rs:146-153 | jj changed-files detection silently drops |
| `validate_base_revision` | vcs.rs:80-98 | Base revision validation silently drops |
| `default_base` | vcs.rs:57-78 | Default base computation silently drops |
| `argument_value` | mod.rs:117-123 | Flag parsing silently drops |

**Only `mod.rs:153-165` tests `check()`, which in turn calls `vcs::changed_files()` and `scan::scan_diff()`. This single test verifies the integration path but not the individual detection functions.**

### Gate 6: Snapshot Tests — PASS
No snapshot tests. The lane uses line-by-line diff scanning.

### Gate 7: Resource Bounds — PASS
Self-test creates scratch directories in `/tmp` and cleans up. Git operations use `CommandIn` with default budgets. No unbounded execution.

### Gate 8: No Dormant/Commented Tests — PASS
No `#[ignore]` tests, no commented-out tests, no dormant modules.

---

## Findings by Severity

### BLOCKER (1)
- **Gate 5 FAIL:** 13 of 17 detection paths lack test coverage. The core detection logic (`deleted_file_findings`, `has_exact_assertion`, `has_weak_assertion`, `has_compile_only`, `has_test_decl`, `deleted_test_declarations`, `weakened_assertions`) would leave all tests green if deleted. The single unit test exercises the integration path but not individual functions.

### LOW (1)
- **vcs.rs:16** — error message says "failed to start" for `run_capture_raw()` errors, but `LaneError` can include timeout/UTF-8 failures, not just spawn failures. The message is misleading.

### MINOR (1)
- **scan.rs:59-64** — `has_weak_assertion` can false-positive on string literals containing `.is_ok(`, `.is_err(`, etc. Acceptable for a lint tool (false positives are safe; false negatives are the real risk), but worth noting.

### MINOR (1)
- **scan.rs:204-227, 229-254** — `deleted_test_declarations` and `weakened_assertions` use count-based detection without per-file attribution. Moving test declarations between files may not trigger findings. The findings are file-level (one per unique path), not line-level.

---

## Verdict

**STATUS: REJECTED**

The check_test_integrity test suite is critically under-tested. The binary implements 17 distinct detection paths (6 in scan.rs, 4 in vcs.rs, 3 in mod.rs, 4 in self_test.rs), but only 1 path is tested (the integration path through `check()`).

The core value of this binary is its detection logic, and 73% of the detection functions have zero test coverage. The single unit test and 2 self-test fixtures are a start, but they don't provide mutation resistance for the core logic.

To achieve APPROVED status, the test suite needs:
1. Unit tests for `has_exact_assertion`, `has_weak_assertion`, `has_test_decl`, `has_ignore_or_skip`, `has_compile_only` (the 5 pattern detectors).
2. Unit tests for `deleted_file_findings`, `deleted_test_declarations`, `weakened_assertions` (the 3 aggregate detectors).
3. Unit tests for `is_test_path`, `is_module_test_path` (path classification).
4. Integration tests for `jj_changed_files` and `validate_base_revision` (VCS variants beyond git).
