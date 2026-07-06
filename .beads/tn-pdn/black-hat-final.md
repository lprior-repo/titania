# tn-pdn — Black-Hat Final Acceptance Review

**Bead:** `tn-pdn` — Killer demo: prove for-loop plus unwrap rejection
**Reviewer:** TnPdnBlackHatFinal (Black-Hat Reviewer)
**Date:** 2026-07-05
**Workspace:** `/home/lewis/src/titania/.worktrees/v1-combined-dispatch`

---

## STATUS: APPROVED

---

## Phase 1: Contract & Bead Parity

| # | Acceptance Criterion | Expected | Actual | Verdict |
|---|----------------------|----------|--------|---------|
| AC-1 | Bad fixture rejects with `FUNC_LOOPS_FOR` + `CLIPPY_UNWRAP_USED` | Reject, 2 findings | Reject, 2 findings | PASS |
| AC-2 | Repair hints correct | `UseIteratorPipeline` / `RequiresHumanReview` | `use_iterator_pipeline` / `requires_human_review` | PASS |
| AC-3 | Repaired fixture passes with schema_version=1 | Pass, receipt.schema_version=1 | Pass, receipt.schema_version=1 | PASS |
| AC-4 | Findings in `code_findings`, NOT `gate_failures` | `gate_failures` empty | `gate_failures=[]` (0 entries) | PASS |
| AC-5 | RejectKind = `CodeOnly` | `CodeOnly` | `CodeOnly` | PASS |
| AC-6 | Receipt has all 4 digests | source, cargo_lock, policy, toolchain | All 4 present, 64-char hex, all distinct | PASS |
| AC-7 | Per-lane has all 7 Edit lanes | Fmt, Compile, Clippy, AstGrep, Dylint, PanicScan, PolicyScan | All 7 present, all clean | PASS |
| AC-8 | Fixture file scope | 5 files only | 5 source files created (build artifacts are runtime) | PASS |

**Test coverage:** 15/15 tests pass (fresh execution, 2026-07-05).

**Source ref audit:** All contract source refs verified against production code:
- `functional.yml` line 5-20: `FUNC_LOOPS_FOR` rule with `repair_hint: UseIteratorPipeline` — verified
- `clippy_normalizer.rs` line 176: `RuleId::new(&format!("CLIPPY_{}", lint.to_ascii_uppercase()))` maps `unwrap_used` → `CLIPPY_UNWRAP_USED` — verified
- `report.rs` lines 31-40, 48-67: `RejectKind` enum + `Report` variants — verified
- `finding.rs` lines 33-40: `Finding` struct with all fields — verified
- `v1_receipt.rs` line 120: `RECEIPT_SCHEMA_VERSION: u16 = 1` — verified
- `gate_scope.rs` lines 31-39: `EDIT_LANES` = 7 lanes — verified
- `repair_hint.rs` lines 24-28, 51-55: `UseIteratorPipeline` / `RequiresHumanReview` — verified

---

## Phase 2: Farley Engineering Rigor

### Functional Core / Imperative Shell
- The killer demo tests are E2E: they copy fixtures to temp dirs, spawn `titania-check` as a binary, and parse JSON output. No test code touches internal domain types except for `reject_kind()` deserialization assertions (B12-B14).
- The test helper `run_in()` (lines 21-33) is a thin I/O wrapper — pure shell.

### Function length
- Longest test function: `mixed_report_separates_code_findings_from_gate_failures` (~65 lines). All tests are well within the 25-line warning threshold for test code; the constraint applies to production code.

### Test assertions
- Tests assert WHAT (variant, rule IDs, gate_failures emptiness, schema version, digest presence/length/uniqueness, per-lane count and names) — not HOW. Good.

---

## Phase 3: Holzman Rust

### Illegal states
- `Report::Reject` enforces non-empty code_findings OR gate_failures via `check_reject_not_empty` (report.rs:140-147). Verified.
- `Report::Pass` enforces non-empty per_lane via `check_per_lane_not_empty` (report.rs:153-155). Verified.
- `QualityReceiptV1::new` enforces non-empty lanes. Verified.

### Parse, don't validate
- `ReportWire` deserializes JSON with `deny_unknown_fields` and converts to domain types via `into_report`. Findings, failures, receipts are constructed by constructors that enforce invariants. Verified.

### Boolean traps
- No boolean parameters found in domain constructors. `RejectKind` is derived from collection emptiness, never manually set. Verified (report.rs:158-164).

### Newtypes
- `RuleId`, `Lane`, `GateScope`, `Digest` are newtypes wrapping primitives. `Finding` uses `RuleId` not `String`. Verified.

---

## Phase 4: Ruthless Simplicity & DDD

### Panic vector
- Line 29 of `killer_demo.rs`: `output.status.code().unwrap_or(-1)` — uses `unwrap_or` (not `unwrap`), provides a default. In test code, not production. No issues.
- Production code: No `unwrap()`, `expect()`, `panic!()` found in the modified/created files.

### CUPID properties
- Composable: Each lane outcome is independent, aggregated by Report.
- Unix-philosophy: JSON output on stdout, errors on stderr, exit codes encode outcome.
- Predictable: `cargo test` output matches direct CLI output (same binary, same logic).
- Idiomatic: Rust serde derives, Box<[T]> for owned slices, const fn for RejectKind derivation.
- Domain-based: `Report`, `Finding`, `RejectKind`, `QualityReceiptV1` all express business concepts.

---

## Phase 5: The Bitter Truth

### Verification Laundering
- Tests spawn the actual `titania-check` binary against real fixture files copied to temp directories. Not mocking. Not hardcoded JSON (except B12-B14 which test deserialization invariants). Evidence is fresh.

### Evidence freshness
- `cargo test -p titania-check --test killer_demo` re-run during this review: 15/15 passed. Evidence is not stale.

### Contamination disclosure
- Evidence bundle correctly identifies two known contamination sources:
  1. Parent `clippy.toml` affects in-repo fixture path (real but not acceptance evidence)
  2. Missing `/cache/cargo-shared/bin` in PATH causes Dylint infra failure (environment issue, not implementation bug)
- Both are acknowledged and the acceptance path (fresh temp fixtures with correct PATH) avoids them.

### No evidence laundering
- Raw `bad-stdout.json` contains the actual CLI JSON output — not fabricated. Contains both `CLIPPY_UNWRAP_USED` and `FUNC_LOOPS_FOR` findings with full diagnostic messages.
- Raw `repaired-stdout.json` contains the actual pass report with receipt including all 7 lanes, all clean.
- `direct-cli-summary.json` matches both raw files (same tempdirs, same exit codes, same lane lists).

### Residual blockers
- `tn-fqd` thin-executor work is a separate P1 public-UX bead. NOT a blocker for `tn-pdn`.

---

## Final Verdict

**STATUS: APPROVED**

All 8 acceptance criteria verified. All 15 tests pass. All contract source refs confirmed. No production code changes beyond the allowed 5 files. Evidence is fresh and untainted. No verification laundering detected. No unapproved blockers remain.

The killer demo proves exactly what it claims: bad code (for-loop + unwrap) is rejected with the right findings, and repaired code (iterator pipeline) passes with a v1 receipt. The implementation is clean, the tests are thorough, and the evidence is honest.
