# Black-Hat Review — tn-03d: v1 Domain Model

**Bead:** tn-03d  
**Scope:** crates/titania-core/src — lane.rs, gate_scope.rs, finding.rs, failure.rs, outcome.rs, report.rs, v1_receipt.rs, diagnostic.rs  
**Test:** tests/tn_03d_domain_model.rs (157 tests)  
**Build:** fmt ✓, check ✓, cargo test 157 passed ✓

---

## Executive Summary

**Status: FAIL — 3 Critical, 7 High, 3 Medium, 3 Low findings**

19 of 19 types are implemented. Core invariants (RepairHint::Patch zero-width rejection, Report::Reject empty-collection rejection, Location::span line_start >= 1 validation, ProcessTermination::signaled 1–31 validation, CommandEvidence::argv non-empty + argv[0] == executable, LaneEvidence::exit_status == Exited(0)) are enforced. But three **critical** gaps undermine the domain model's correctness guarantees: `Report::pass()` is missing (allowing empty-per_lane deserialization), an `#[allow]`-suppressed `unwrap()` violates Holzman Rust, and `Report::Reject`/`Report::Pass` deserialize without invariant checks. Seven **high** gaps are missing smart constructors specified in the contract.

---

## Critical Findings

### C1: `Report::pass()` smart constructor is missing

**Severity:** CRITICAL  
**File:** report.rs  
**Spec:** contract.md §3 line 295-297  
**Evidence:** Contract specifies `pub fn pass(receipt: QualityReceipt, per_lane: Box<[LaneOutcome]>) -> Self` with invariant `per_lane.len() >= 1`. No such method exists in report.rs.

**Impact:** Anyone deserializing `Report::Pass` from JSON can bypass the `per_lane >= 1` invariant. The test at line 288-294 of tn_03d_domain_model.rs directly constructs `Report::Pass { receipt, per_lane: Box::new([]) }` — **empty per_lane** — and asserts `is_pass()` returns true. This validates an invariant-violating state.

**Fix:** Add `Report::pass()` constructor with `per_lane.len() >= 1` check. Replace serde's auto-derive deserialization with a custom one that routes through the constructor.

### C2: `unwrap()` in outcome.rs suppresses Holzman lint

**Severity:** CRITICAL  
**File:** outcome.rs:97-98  
**Spec:** lib.rs:10 `#![deny(clippy::unwrap_used)]`, Holzman Rust: zero unwrap/expect/panic  
**Evidence:**
```rust
#[allow(clippy::unwrap_used)]
found: argv.first().unwrap().clone(),
```

**Impact:** Despite `#![deny(clippy::unwrap_used)]` at the crate level, this `#[allow]` attribute suppresses the lint. While the unwrap is technically safe (line 91 checks `argv.is_empty()` first), it directly violates the zero-unwrap policy. The `#[allow]` creates a precedent that undermines the lint's purpose.

**Fix:** Replace `.unwrap()` with `argv[0].clone()` — but that also uses indexing. Better: destructure `argv.first().copied()` into an `Option` and use `match` to extract the value without `.unwrap()`:
```rust
match argv.first() {
    Some(first) => Err(OutcomeError::Argv0Mismatch { expected, found: first.clone() }),
    None => unreachable!(), // unreachable after empty check above
}
```
Or simply avoid the `#[allow]` and restructure to eliminate the need.

### C3: `Report` deserialization bypasses all invariants

**Severity:** CRITICAL  
**File:** report.rs:33 (derive Deserialize)  
**Spec:** contract.md §2 line 268-269  
**Evidence:** `Report` uses `#[derive(Serialize, Deserialize)]` with serde's auto-derive. Fields are private (good), but serde populates them directly, bypassing all smart constructors.

**Impact:** A JSON payload like `{"variant":"pass","receipt":{...},"per_lane":[]}` deserializes into a `Report::Pass` with empty `per_lane`, violating the invariant. Similarly, `{"variant":"reject","code_findings":[],"gate_failures":[],"per_lane":[]}` produces an invalid `Report::Reject` with both collections empty.

**Fix:** Implement custom `Deserialize` for `Report` that routes `Pass` through `pass()` and `Reject` through `reject()`, returning errors on invariant violations.

---

## High Findings

### H1: `LaneOutcome` smart constructors missing

**Severity:** HIGH  
**File:** outcome.rs  
**Spec:** contract.md §3 line 352-358  
**Evidence:** Contract specifies `LaneOutcome::clean()`, `::findings()`, `::failed()`, `::skipped()` as smart constructors. Only the `Clean` variant exists with direct construction.

**Impact:** Users must construct `LaneOutcome::Clean { evidence }` directly rather than through `clean(evidence)`. The `clean()` constructor would validate `exit_status == Exited(0)` at the outcome level. Currently this validation happens in `LaneEvidence::new()`, which is a reasonable design choice but deviates from the contract.

**Fix:** Implement `LaneOutcome::clean(evidence: LaneEvidence) -> Result<Self, OutcomeError>`, `::findings()`, `::failed()`, `::skipped()`.

### H2: `Report::policy_error()` and `Report::input_error()` missing

**Severity:** HIGH  
**File:** report.rs  
**Spec:** contract.md §3 line 303-304  
**Evidence:** Contract specifies `pub fn policy_error(diagnostics: Box<[PolicyDiagnostic]>) -> Self` and `pub fn input_error(diagnostics: Box<[InputDiagnostic]>) -> Self`. Neither exists.

**Impact:** Users must construct `Report::PolicyError { ... }` directly.

**Fix:** Add both constructors.

### H3: `Location` smart constructors missing

**Severity:** HIGH  
**File:** finding.rs  
**Spec:** contract.md §3 line 333-336  
**Evidence:** Contract specifies `Location::dependency()`, `::manifest()`, `::workspace()`, `::tool()` as smart constructors. Only `Location::span()` exists.

**Impact:** Users construct `Location::Dependency { ... }`, `Location::Manifest { ... }`, etc. directly. `Location::workspace()` is particularly important as it's a zero-argument constructor that's currently missing.

**Fix:** Add `fn dependency(crate_name, version)`, `fn manifest(file)`, `fn workspace()`, `fn tool(name, version)`.

### H4: `RepairHint` smart constructors missing

**Severity:** HIGH  
**File:** finding.rs  
**Spec:** contract.md §3 line 343-348  
**Evidence:** Contract specifies six constructors: `use_iterator_pipeline()`, `flatten_nesting()`, `use_checked_arithmetic()`, `remove_allow_attribute()`, `replace_dependency()`, `requires_human_review()`. Only `patch()` exists.

**Impact:** Users construct variant syntax directly.

**Fix:** Add all six smart constructors.

### H5: `LaneFailure` smart constructors missing

**Severity:** HIGH  
**File:** failure.rs  
**Spec:** contract.md §3 line 380-383  
**Evidence:** Contract specifies `::infra_failure()`, `::tool_failure()`, `::resource_failure()`, `::suspicious_failure()`. Only `LaneFailure::InfraFailure { ... }` direct construction is used in tests.

**Impact:** Users construct directly.

**Fix:** Add all four constructors.

### H6: `ProcessTermination` smart constructors missing

**Severity:** HIGH  
**File:** failure.rs  
**Spec:** contract.md §3 line 388-393  
**Evidence:** Contract specifies `::exited()`, `::timed_out()`, `::memory_limit_exceeded()`, `::spawn_failed()`. Only `::signaled()` and direct `Exited { code }` construction.

**Impact:** Users construct directly. `ProcessTermination::Exited { code }` is a unit-like variant that can be constructed directly.

**Fix:** Add `fn exited(code: i32)`, `fn timed_out()`, `fn memory_limit_exceeded()`, `fn spawn_failed()`.

### H7: `Report::Reject` and `Report::Pass` serde invariant violation

**Severity:** HIGH  
**File:** report.rs:33-51  
**Spec:** contract.md §2 line 268-269, H3 (Hazard #3)  
**Evidence:** `Report` derives `Deserialize`. JSON `{"variant":"reject","code_findings":[],"gate_failures":[],"per_lane":[]}` creates an invalid reject.

**Impact:** Any system receiving lane output JSON can construct invalid reports. This is the serialization vector for C1's invariant violation.

**Fix:** Custom `Deserialize` impl for `Report` (see C3).

---

## Medium Findings

### M1: `LaneEvidence::new()` deviates from contract

**Severity:** MEDIUM  
**File:** outcome.rs:44-53  
**Spec:** contract.md §3 line 362-369  
**Evidence:** Contract says `LaneEvidence::new() -> Self` with "No cross-field validation." Implementation returns `Result<Self, OutcomeError>` and validates `exit_status.is_success()`.

**Impact:** The contract expected validation at the `LaneOutcome::clean()` layer. Moving it to `LaneEvidence::new()` is more defensive (prevents invalid LaneEvidence from ever existing) but changes the API contract. The `LaneOutcome::clean()` constructor that was supposed to validate is also missing.

**Fix:** Either: (a) remove the validation from `LaneEvidence::new()` and add `LaneOutcome::clean()` per contract, or (b) keep validation here and update contract to match.

### M2: `QualityReceiptV1::new()` doc comment is misleading

**Severity:** MEDIUM  
**File:** v1_receipt.rs:59  
**Evidence:** Doc comment says "Panics if schema_version is not RECEIPT_SCHEMA_VERSION (1)." Implementation sets `schema_version: RECEIPT_SCHEMA_VERSION` directly — no panic path.

**Fix:** Remove the panic doc comment or make the constructor accept `schema_version: u16` and panic/return `Err` when it doesn't match.

### M3: Public struct fields bypass encapsulation

**Severity:** MEDIUM  
**Files:** v1_receipt.rs:14-21, :37-52; diagnostic.rs:22-26, :50-54  
**Evidence:** `LaneReceipt.lane`, `evidence_digest`, `clean`; `QualityReceiptV1` all 7 fields; `PolicyDiagnostic` all 3 fields; `InputDiagnostic` all 3 fields — all `pub`.

**Impact:** External crates can create/modify these structs directly, bypassing smart constructors. This is acceptable for serde deserialization but weakens the "constructors enforce invariants" guarantee.

**Fix:** Make fields private, add `#[serde(getter = "...", setter = "...")]` or implement `Deserialize` to route through constructors where validation is needed. For simple data carriers with no invariants (like `LaneReceipt`, `PolicyDiagnostic`), public fields are acceptable if the contract doesn't require validation.

---

## Low Findings

### L1: `Lane::name()` duplicates serde logic

**Severity:** LOW  
**File:** lane.rs:47-60  
**Evidence:** `Lane::name()` matches every variant to its PascalCase string. This is redundant with `#[serde(rename_all = "PascalCase")]`.

**Fix:** Could use a derive macro or a `strum::Display` to auto-generate, but the current approach is explicit and fast. Acceptable as-is.

### L2: Missing `LaneOutcome::is_clean()`, `::is_failed()`, `::is_skipped()` for non-match users

**Severity:** LOW  
**File:** outcome.rs:125-145  
**Evidence:** `is_pass()`, `is_findings()`, `is_failed()`, `is_skipped()` exist. `is_clean()` does not — a user checking `Clean` must match explicitly. Minor inconsistency with `is_findings()` / `is_failed()`.

**Fix:** Add `fn is_clean(&self) -> bool`.

### L3: No `#[deny(missing_docs)]` on new modules

**Severity:** LOW  
**File:** lib.rs  
**Evidence:** The crate doc is comprehensive, but individual new module files lack `///` doc comments at the module level. Most do have them (lane.rs, gate_scope.rs, finding.rs, etc.), but `diagnostic.rs` has minimal module-level docs.

**Fix:** Add module-level `///` docs to all new files. Currently all new files have module-level docs — this is already satisfied.

---

## Test Coverage Gaps

| Criterion | Covered? | Test |
|---|---|---|
| `Report::pass()` with `per_lane.len() >= 1` | **NO** | Test constructs `Pass` directly with empty `per_lane` |
| `Report::Reject` serde invariant check | **NO** | No test for rejecting empty collections via JSON |
| `Report::Pass` serde invariant check | **NO** | No test for rejecting empty per_lane via JSON |
| `LaneOutcome::clean()` constructor | **NO** | Test constructs `Clean { evidence }` directly |
| `Location::dependency()`, `manifest()`, `tool()`, `workspace()` | **NO** | Only `Span` and direct variant construction tested |
| `RepairHint` smart constructors (6 variants) | **NO** | Only `patch()` tested via constructor; others via direct syntax |
| `LaneFailure` smart constructors (4 variants) | **NO** | Only `InfraFailure` tested via direct construction |
| `ProcessTermination` smart constructors | **NO** | Only direct `Exited`, `TimedOut`, `SpawnFailed`, `MemoryLimitExceeded` tested |
| `GateScope::#[non_exhaustive]` compile check | **NO** | No `#[deny(unreachable_patterns)]` or exhaustive-match test |
| `GateScope::lan()` slice correctness for all 3 scopes | **YES** | 3 separate tests for edit (7 lanes), prepush (9 lanes), release (10 lanes) |
| `Lane::from_str` determinism | **YES** | All 10 variants + case sensitivity tested |
| `RepairHint::Patch` zero-width rejection | **YES** | `TextRange::new(5, 5)` tested |
| `Finding::new()` validation | **YES** | `make_valid_finding()` tested |
| `QualityReceipt::schema_version == 1` | **YES** | Assertion in quality_receipt_constructs |
| `CommandEvidence::argv` validation | **YES** | Empty + mismatch tests |
| `ProcessTermination::signaled` 1-31 | **YES** | Signals 0, 1, 31, 32 tested |
| Serde round-trips for all 19 types | **YES** | All types tested |

---

## Acceptance Criteria Assessment

| Criterion | Status |
|---|---|
| All 19 types implemented | PASS |
| Every type invariant enforced at construction | PARTIAL (serde bypasses some; missing `pass()` constructor) |
| No forbidden Rust constructs (unwrap/expect/panic/unsafe/indexing) | FAIL (unwrap at outcome.rs:98) |
| GateScope `#[non_exhaustive]` with deny lint | PASS |
| Report::Reject rejects empty collections | PASS (constructor) / FAIL (serde bypass) |
| GateScope::lanes() returns correct ordered slices | PASS |
| Lane::from_str is deterministic | PASS |
| Finding constructor validates all fields | PASS |
| RepairHint::Patch rejects zero-width ranges | PASS |
| QualityReceipt schema_version correct | PASS |
| All acceptance criteria from bead §4 tested | PARTIAL (missing constructors tested) |

---

## Bitter Truth

The implementation is **80% of the way to a production-grade domain model**. The type system correctly encodes most invariants, serde round-trips work for all types, and the test suite covers the happy path comprehensively. But the three critical issues are fundamental:

1. The **missing `Report::pass()` constructor** means the most important invariant (per_lane >= 1) has no constructor enforcement and no test.
2. The **`#[allow]`-suppressed `unwrap()`** is a Holzman Rust violation that creates a precedent for future violations.
3. The **serde deserialization bypass** means any system deserializing lane output can create invariant-violating states.

These are not edge cases — they affect the core judgment system's correctness. The gate passes when it shouldn't, or rejects when it shouldn't, if any of these invariants can be violated.

---

*Review completed: 2026-07-01*
