# Pristine Pass B: Untouched Surface Audit

**Parent:** tn-ily
**Session:** pristine-pass-b (planner.nu inoperable — inline plan)
**Created:** 2026-07-02
**Scope:** Option B — titania-lanes untouched surface

## Scope

- `titania-lanes/src/bin/*` — 30+ CLI adapters, verification tooling
- `titania-lanes/src/command/*` — command layer (reader.rs, output.rs, process.rs, helpers.rs)
- `titania-lanes/src/helpers.rs` — utility helpers
- `titania-lanes/Cargo.toml` — workspace crate config
- `titania-lanes/config/` — workspace-level config
- Workspace Cargo.toml, `.cargo/config.toml` — dependency policy

## Exclusions

- `titania-core/src/` — covered by tn-03d-core-domain rev5 (black-hat: 3 critical, 7 high, 3 medium, 3 low)
- `titania-core/tests/` — covered by tech-debt-audit worktree
- `tn-sm8` — in-progress Holzman/DDD/drift fixes (A1-D8)

## Bead Mapping

| Bead | Lane | Task ID |
|------|------|---------|
| tn-ily.1 | Black-hat | bh-surface |
| tn-ily.2 | Test-reviewer | ts-surface |
| tn-ily.3 | Proof-reviewer | pr-surface |

---

## EARS Requirements (All Three Lanes)

### Ubiquitous

- THE SYSTEM SHALL produce a review report for all files in scope
- THE SYSTEM SHALL categorize every finding by severity (CRITICAL/HIGH/MEDIUM/LOW)
- THE SYSTEM SHALL reference file:line for every finding
- THE SYSTEM SHALL exclude files already covered by tn-03d-core-domain or tech-debt-audit

### Event-Driven

- WHEN a production `.rs` file is scanned, THE SYSTEM SHALL check for forbidden constructs (unwrap/expect/panic/unsafe)
- WHEN a test file is reviewed, THE SYSTEM SHALL check for is_ok()-only assertions
- WHEN a proof model or Kani harness is found, THE SYSTEM SHALL check for vacuous execution paths
- WHEN a type is inspected, THE SYSTEM SHALL verify DDD bounded context alignment
- WHEN an assume/admit is found in proof evidence, THE SYSTEM SHALL verify it is not proof laundering

### Unwanted

- IF a finding has no file:line reference, THE SYSTEM SHALL NOT include it in the report, BECAUSE findings must be actionable with exact locations
- IF a test uses fake data or mocks for a real contract, THE SYSTEM SHALL NOT pass it as verified, BECAUSE real data is required for meaningful assertions
- IF a proof claim has no raw CLI command evidence, THE SYSTEM SHALL NOT consider it verified, BECAUSE command evidence is the only acceptable proof

---

## KIRK Contracts

### Black-hat (bh-surface / tn-ily.1)

**Preconditions:**
- Workspace compiles with `cargo check`
- All `.rs` files in scope are accessible

**Postconditions:**
- Report with findings categorized by severity (CRITICAL/HIGH/MEDIUM/LOW)
- All findings include file:line references and fix guidance

**Invariants:**
- No forbidden constructs in production code (unwrap/expect/panic/unsafe)
- Zero unsafe code without explicit waiver

### Test-reviewer (ts-surface / tn-ily.2)

**Preconditions:**
- All test files in scope are accessible
- Workspace compiles with `cargo check --all-targets`

**Postconditions:**
- Report with test coverage gaps identified
- Findings on assertion strength, determinism, mutation resistance

**Invariants:**
- No is_ok()-only assertions in test suites
- All public APIs have corresponding tests

### Proof-reviewer (pr-surface / tn-ily.3)

**Preconditions:**
- Evidence files accessible in `.evidence/` directory

**Postconditions:**
- Report with proof gaps documented
- Vacuous models identified

**Invariants:**
- No vacuous proof models accepted
- All evidence has raw command proof

---

## Test Design (ATDD)

### Black-hat Tests

**Happy Path:**
1. All production files in `titania-lanes/src/bin/*`, `src/command/*`, `src/helpers.rs` scanned for violations
2. Findings reported with `file:line:severity:problem.fix` format

**Error Path:**
1. Non-Rust files (`.toml`, `.md`, etc.) skipped gracefully
2. Missing files in scope reported as warnings (not errors)

### Test-reviewer Tests

**Happy Path:**
1. All test files in `titania-lanes/tests/*` scanned for integrity
2. Test gaps reported with `file:line` references

**Error Path:**
1. Missing test coverage for public APIs flagged
2. Non-deterministic tests reported (e.g., `thread_rng()`, `chrono::now()`)

### Proof-reviewer Tests

**Happy Path:**
1. All verus/Kani evidence files in `.evidence/verus/`, `.evidence/kani-list/` scanned
2. Gaps documented with severity

**Error Path:**
1. Missing evidence for proof claims flagged
2. Unproven claims with `assume`/`admit` identified

---

## Inversions (Munger: Invert, Always Invert)

### Security Failures
- **Failure:** False positive findings on safe code
- **Prevention:** Cross-reference findings with actual `cargo clippy` output
- **Test:** Run clippy independently and compare

### Data Integrity Failures
- **Failure:** Missing files due to glob pattern issues
- **Prevention:** Verify file count matches expected scope
- **Test:** Count `.rs` files in scope and confirm all scanned

### Integration Failures
- **Failure:** Test coverage blind spots
- **Prevention:** Map all public APIs to tests
- **Test:** Use `lsp references` to find public API usages vs test imports

### Usability Failures
- **Failure:** Reports too verbose, hard to triage
- **Prevention:** Group findings by type, sort by severity
- **Test:** Human review of report for actionability

---

## Implementation Tasks

### Phase 0: Scout (Parallel)
- [ ] Map all `.rs` files in scope (bin/*, command/*, helpers.rs)
- [ ] List all test files in scope (titania-lanes/tests/*)
- [ ] List all evidence files (.evidence/verus/, .evidence/kani-list/)
- [ ] Count files per lane for verification later

### Phase 1: Black-hat Review (tn-ily.1)
- [ ] Scan all production `.rs` files for forbidden constructs
- [ ] Check DDD bounded context alignment
- [ ] Check contract parity (types match requirements)
- [ ] Check Farley constraints (separation of concerns)
- [ ] Check Holzman Rust (Power of Ten rules)
- [ ] Write report to `.beads/tn-ily.1/black-hat-report.md`

### Phase 2: Test-review (tn-ily.2)
- [ ] Scan all test files for assertion strength
- [ ] Check determinism (no thread_rng, chrono::now)
- [ ] Check mutation resistance (would mutations survive?)
- [ ] Check public API coverage
- [ ] Write report to `.beads/tn-ily.2/test-review-report.md`

### Phase 3: Proof-review (tn-ily.3)
- [ ] Scan verus specs for vacuous models
- [ ] Scan Kani harnesses for shallow bounds
- [ ] Check assume/admit usage
- [ ] Verify raw command evidence
- [ ] Write report to `.beads/tn-ily.3/proof-review-report.md`

### Phase 4: Gate
- [ ] `cargo fmt --check` on titania-lanes
- [ ] `cargo clippy --workspace --lib --bins --all-features -D warnings`
- [ ] `cargo test --workspace --all-features --no-run`
- [ ] Verify no forbidden constructs in touched files

---

## Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| `cargo check` fails | Workspace broken | Fix build before reviewing |
| File not found | Glob pattern mismatch | Verify scope against glob |
| Report empty | Scope too narrow | Expand scope, check exclusions |
| Clippy warnings block | Pre-existing in main | Report separately, don't block |

---

## Anti-Hallucination

**Read-before-write rules:**
1. Read file contents before referencing code in findings
2. Run `cargo check` before claiming code compiles
3. Run `cargo clippy` before claiming lint status
4. Cross-reference `lsp references` before claiming API coverage

**APIs that exist:**
- `cargo check`, `cargo clippy`, `cargo test`
- `lsp references`, `grep`, `glob`
- `bd` (bead CLI)

**No placeholder values:**
- Use real file paths, real function names, real line numbers
- No "TODO", "FIXME", or "see related files" in findings

---

## Completion Checklist

- [ ] All acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] Black-hat report written to `.beads/tn-ily.1/`
- [ ] Test-review report written to `.beads/tn-ily.2/`
- [ ] Proof-review report written to `.beads/tn-ily.3/`
- [ ] Implementation uses `Result<T, Error>` throughout
- [ ] Zero `unwrap()` or `expect()` calls in production
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes

---

## Context

**Related files:**
- `titania-lanes/src/lib.rs` — workspace lib
- `titania-lanes/src/command/` — command layer
- `titania-lanes/src/bin/` — CLI binaries
- `.beads/tn-03d/black-hat-report.md` — prior black-hat (for exclusion reference)
- `.beads/tn-sm8` — prior tech debt audit

**Similar implementations:**
- `tn-03d-core-domain` black-hat report (pattern to follow)
- `tech-debt-audit` worktree (pattern to follow)

**AI Hints:**
- Do: Use functional patterns (`map`, `and_then`, `?`)
- Do: Return `Result<T, Error>` from all fallible functions
- Do: READ files before modifying them
- Do: Group findings by severity, sort CRITICAL first
- Do: Reference exact file:line for every finding
- Do NOT use `unwrap()` or `expect()`
- Do NOT use `panic!`, `todo!`, or `unimplemented!`
- Do NOT modify clippy configuration
- Do NOT touch `titania-core/src/` (covered by tn-03d)
- Do NOT touch `titania-core/tests/` (covered by tech-debt-audit)
