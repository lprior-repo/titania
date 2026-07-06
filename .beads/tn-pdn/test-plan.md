# Test Plan: tn-pdn — Killer Demo (Bad-Code Reject / Repaired-Pass)

## Summary

| Item | Count |
|------|-------|
| Acceptance criteria (contract) | 8 (AC-1 through AC-8) |
| Behavior tests (BDD) | 14 |
| Integration tests (via `cli_dispatch`/`aggregate_cli` patterns) | 3 |
| Proptest invariants | 2 |
| Fuzz targets | 0 (not in scope — no parsing/deserialization of untrusted input) |
| Kani harnesses | 0 (not in scope — v1 does not use Kani; deferred to v1.5+) |
| Mutation checkpoints | 2 |

**Trophy allocation**: ~60% integration/E2E (CLI-driven behavior tests via `titania-check` binary), ~25% unit (domain-value construction for `Report`/`Finding`/`RepairHint`), ~10% property (proptest invariants), ~5% static (golden JSON).

**Strategy**: All killer-demo behaviors are exercised via **integration/E2E tests** that exercise the public CLI (`titania-check --scope edit --emit json`) against controlled fixture workspaces. Unit-level domain-value tests reuse the existing `json_roundtrip.rs` pattern from `titania-core`. No Kani, fuzz, or mutation testing is in scope for this bead per §15.18 (deferred to v1.5+).

**RED-before-GREEN**: Every planned test in `killer_demo.rs` must compile (the test file is written first), then the fixtures are written, then the production code paths that make them pass are verified.

---

## 1. Behavior Inventory

Each behavior is described as: *[Subject] [action] [outcome] when [condition]*

| # | Behavior | Contract |
|---|----------|----------|
| B1 | `titania-check --scope edit` rejects a fixture containing a `for` loop and `.unwrap()` | AC-1 |
| B2 | The reject report contains exactly two code findings: `FUNC_LOOPS_FOR` and `CLIPPY_UNWRAP_USED` | AC-1 |
| B3 | Each code finding has the correct `RepairHint` variant (`UseIteratorPipeline` / `RequiresHumanReview`) | AC-2 |
| B4 | The reject report's `gate_failures` collection is empty for the bad fixture | AC-4 |
| B5 | The reject report's `reject_kind()` returns `RejectKind::CodeOnly` | AC-5 |
| B6 | A repaired fixture (iterator pipeline, no `.unwrap()`) produces `Report::Pass` | AC-3 |
| B7 | The pass receipt has `schema_version == 1` and `scope == GateScope::Edit` | AC-3 |
| B8 | The pass receipt contains all four digests (`source`, `cargo_lock`, `policy`, `toolchain`) | AC-6 |
| B9 | The pass report's `per_lane` contains exactly 7 `LaneOutcome` entries (one per Edit lane) | AC-7 |
| B10 | All 7 Edit-lane names appear in `per_lane`: Fmt, Compile, Clippy, AstGrep, Dylint, PanicScan, PolicyScan | AC-7 |
| B11 | Missing `Cargo.toml` input error exits with code 3 and `InputDiagnostic` | (implicit from CLI contract) |
| B12 | `Report::Reject` in Mixed case separates code findings from gate failures (no cross-contamination) | AC-4 |
| B13 | `Report::Reject` with zero code findings and one gate failure → `RejectKind::GateOnly` | AC-5 |
| B14 | `Report::Reject` with both code findings and gate failures → `RejectKind::Mixed` | AC-5 |
---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| **E2E (CLI)** | 6 | Killer-demo is inherently an end-to-end CLI behavior: the user invokes `titania-check --scope edit --emit json` and inspects JSON output. These are the primary tests. |
| **Integration (aggregate)** | 3 | Report assembly from lane artifacts: verify `code_findings` vs `gate_failures` separation, `RejectKind` classification, and empty-reject invariant. These complement the CLI tests by exercising `assemble_report` directly. |
| **Property (proptest)** | 2 | Invariants on `reject_kind_from_empty` exhaustive mapping and `RepairHint` variant round-trip. |
| **Static (golden JSON)** | 1 | Reuse existing `json_roundtrip.rs` golden `REPORT_REJECT_JSON` which already encodes the bad-fixture JSON shape. No new golden tests needed — the killer demo will produce output that matches this shape. |

---

## 3. BDD Scenarios

All scenarios target `crates/titania-check/tests/killer_demo.rs`. Each is a `#[test]` fn named per the pattern `fn [subject]_[outcome]_when_[condition]()`.

### Behavior B1: Bad Fixture Rejects with Report::Reject

```
### Test: bad_fixture_rejects_with_code_findings

Given: A fixture workspace at `fixtures/strict_ai_loop_unwrap/bad/` containing:
  - `Cargo.toml` with `[package] name = "strict_ai_loop_unwrap_bad" version = "0.1.0" edition = "2024"`
  - `src/lib.rs` containing a `for` loop and `.unwrap()`:
      ```rust
      fn bad_function(items: Vec<Option<i32>>) -> Vec<i32> {
          for item in items {
              let value = item.unwrap();
              vec![value]
          }
      }
      ```
  - `Dylint.lane-artifact.json` at `.titania/out/edit/Dylint.lane-artifact.json` with variant `"clean"` (mock artifact to prevent Dylint infra-fail)

When: `titania-check --scope edit --emit json` is executed in the fixture directory

Then:
- Exit code is `1` (reject)
- `stdout` parses as JSON with `variant == "reject"`
- `code_findings` array length is `2`
- `gate_failures` array length is `0`
- `per_lane` array length is `7` (all Edit lanes attempted)
- `code_findings[*].rule_id` contains `"FUNC_LOOPS_FOR"` (first finding) and `"CLIPPY_UNWRAP_USED"` (second finding) — asserts exact rule identity, not just count
```

### Behavior B2: Two Code Findings with Exact Rule IDs

```
### Test: bad_fixture_has_func_loops_for_finding

Given: The bad fixture workspace (same as B1)

When: `titania-check --scope edit --emit json` produces a `Report::Reject`

Then:
- Exactly one finding has `rule_id == "FUNC_LOOPS_FOR"`
- That finding's `lane == "AstGrep"`
- That finding's `effect == "reject"`
- That finding's `repair.variant == "use_iterator_pipeline"`
- That finding's `message` contains "for loop" or "loop" text

### Test: bad_fixture_has_clippy_unwrap_used_finding

Given: The bad fixture workspace (same as B1)

When: `titania-check --scope edit --emit json` produces a `Report::Reject`

Then:
- Exactly one finding has `rule_id == "CLIPPY_UNWRAP_USED"`
- That finding's `lane == "Clippy"`
- That finding's `effect == "reject"`
- That finding's `repair.variant == "requires_human_review"`
```

### Behavior B3: Repair Hints Correct

```
### Test: bad_fixture_findings_have_correct_repair_hints

Given: The bad fixture workspace (same as B1)

When: `titania-check --scope edit --emit json` produces a `Report::Reject`

Then:
- Finding with `rule_id == "FUNC_LOOPS_FOR"` has `repair.variant == "use_iterator_pipeline"`
- Finding with `rule_id == "CLIPPY_UNWRAP_USED"` has `repair.variant == "requires_human_review"`
```

### Behavior B4: Gate Failures Empty for Bad Fixture

```
### Test: bad_fixture_gate_failures_empty

Given: The bad fixture workspace (same as B1)

When: `titania-check --scope edit --emit json` produces a `Report::Reject`

Then:
- `gate_failures` array is empty (length 0)
- Every entry in `per_lane` is either `"clean"`, `"findings"`, or `"skipped"` (none are `"failed"`)
```

### Behavior B5: RejectKind is CodeOnly

```
### Test: bad_fixture_reject_kind_is_code_only

Given: The bad fixture workspace (same as B1)

When: `titania-check --scope edit --emit json` produces a `Report::Reject`

Then:
- `reject_kind` field (computed from collections) equals `"code_only"`
  (JSON key: the report may not serialize `reject_kind` directly; instead assert
   `code_findings` length > 0 AND `gate_failures` length == 0, which is the
   invariant that `reject_kind_for` maps to `CodeOnly`)
```

### Behavior B6: Repaired Fixture Passes with Pass Report

```
### Test: repaired_fixture_passes_with_receipt

Given: A fixture workspace at `fixtures/strict_ai_loop_unwrap/repaired/` containing:
  - `Cargo.toml` with `[package] name = "strict_ai_loop_unwrap_repaired" version = "0.1.0" edition = "2024"`
  - `src/lib.rs` containing an iterator pipeline, no `.unwrap()`:
      ```rust
      fn good_function(items: Vec<Option<i32>>) -> Vec<i32> {
          items.iter().map(|item| item.unwrap_or(0)).collect()
      }
      ```
  - `Dylint.lane-artifact.json` at `.titania/out/edit/Dylint.lane-artifact.json` with variant `"clean"` (mock artifact to prevent Dylint infra-fail)

When: `titania-check --scope edit --emit json` is executed in the fixture directory

Then:
- Exit code is `0` (pass)
- `stdout` parses as JSON with `variant == "pass"`
- `receipt.schema_version == 1`
- `receipt.scope == "Edit"`
```

### Behavior B7: Receipt Schema Version and Scope

```
### Test: repaired_fixture_receipt_has_schema_version_one

Given: The repaired fixture workspace (same as B6)

When: `titania-check --scope edit --emit json` produces a `Report::Pass`

Then:
- `receipt.schema_version` equals `1` (number, not string)
- `receipt.scope` equals `"Edit"`
```

### Behavior B8: Receipt Contains All Four Digests

```
### Test: repaired_fixture_receipt_contains_all_digests

Given: The repaired fixture workspace (same as B6)

When: `titania-check --scope edit --emit json` produces a `Report::Pass`

Then:
- `receipt.source_digest` is a 64-character hex string (non-zero)
- `receipt.cargo_lock_digest` is a 64-character hex string (non-zero)
- `receipt.policy_digest` is a 64-character hex string (non-zero)
- `receipt.toolchain_digest` is a 64-character hex string (non-zero)
- All four digest values are different from each other
```

### Behavior B9: Per-Lane Contains Exactly 7 Entries

```
### Test: repaired_fixture_per_lane_has_seven_entries

Given: The repaired fixture workspace (same as B6)

When: `titania-check --scope edit --emit json` produces a `Report::Pass`

Then:
- `per_lane` array length is exactly `7`
- Every entry has `"variant"` key set to `"clean"` (or `"skipped"` for Dylint if .so unavailable)
```

### Behavior B10: All 7 Edit Lanes Present

```
### Test: repaired_fixture_per_lane_contains_all_edit_lanes

Given: The repaired fixture workspace (same as B6)

When: `titania-check --scope edit --emit json` produces a `Report::Pass`

Then:
- `per_lane` entries contain exactly these `lane` values:
  `"Fmt"`, `"Compile"`, `"Clippy"`, `"AstGrep"`, `"Dylint"`, `"PanicScan"`, `"PolicyScan"`
- No extra lanes (e.g., `"Test"`, `"Deny"`, `"Build"`) are present
```

### Behavior B11: Missing Cargo.toml Produces InputError

```
### Test: missing_cargo_toml_produces_input_error

Given: An empty temporary directory with no `Cargo.toml`

When: `titania-check --scope edit --emit json` is executed

Then:
- Exit code is `3` (InputError)
- `stdout` is empty
- `stderr` contains `"InputError"` and mentions the missing file or directory
```
### Behavior B12: Mixed Case Separates Code Findings from Gate Failures (Integration)

```
### Test: mixed_report_separates_code_findings_from_gate_failures

Given: In-memory `Finding` and `LaneFailure` constructs:
  - One `Finding` with `rule_id == "FUNC_LOOPS_FOR"`, `lane == "AstGrep"`, `effect == "reject"`
  - One `Finding` with `rule_id == "CLIPPY_UNWRAP_USED"`, `lane == "Clippy"`, `effect == "reject"`
  - One `LaneFailure` with `lane == "Dylint"` (infrastructure lane, functional lane absent)
  - A `Report::Reject` assembled from these collections

When: `reject_kind()` is computed on the assembled `Report::Reject`

Then:
- `reject_kind` equals `"mixed"` (both `code_findings` and `gate_failures` non-empty)
- `code_findings` entries have lanes `"AstGrep"` and `"Clippy"` only (functional lanes)
- `gate_failures` entries have lane `"Dylint"` only (infrastructure lane)
- No `code_findings` entry has an infrastructure-only lane (Fmt, Compile, Dylint, etc.)
- No `gate_failures` entry has a functional rule ID
```

### Behavior B13: GateOnly RejectKind (Integration)

```
### Test: report_reject_gate_only

Given: In-memory `Finding` and `LaneFailure` constructs:
  - Zero `Finding` entries (no code findings)
  - One `LaneFailure` with `lane == "Dylint"` (infrastructure lane)
  - A `Report::Reject` assembled from these collections

When: `reject_kind()` is computed on the assembled `Report::Reject`

Then:
- `reject_kind` equals `"gate_only"` (`code_findings` empty, `gate_failures` non-empty)
- `code_findings` array length is `0`
- `gate_failures` array length is `1`
- The single gate failure has `lane == "Dylint"`
```

### Behavior B14: Mixed RejectKind (Integration)

```
### Test: report_reject_mixed

Given: In-memory `Finding` and `LaneFailure` constructs:
  - One `Finding` with `rule_id == "FUNC_LOOPS_FOR"`, `lane == "AstGrep"`, `effect == "reject"`
  - One `LaneFailure` with `lane == "Compile"` (infrastructure lane)
  - A `Report::Reject` assembled from these collections

When: `reject_kind()` is computed on the assembled `Report::Reject`

Then:
- `reject_kind` equals `"mixed"` (both `code_findings` and `gate_failures` non-empty)
- `code_findings` array length is `1` with `lane == "AstGrep"`
- `gate_failures` array length is `1` with `lane == "Compile"`
```

---

## 4. Proptest Invariants

These proptests exercise the pure domain functions referenced in the contract. They are planned as `#[test]` fn in `killer_demo.rs` or alongside it.

### Proptest P2: reject_kind_from_empty Covers All Combinations

```
Invariant: The 4 input combinations of (code_empty, gate_empty) map to exactly 4 outcomes:
  (false, true)  → CodeOnly
  (true, false)  → GateOnly
  (false, false) → Mixed
  (true, true)   → None

Strategy: Exhaustive enumeration of `bool x bool` (4 cases).
Anti-invariant: N/A — exhaustive covers all cases.
```

### Proptest P3: RepairHint Deserialization Preserves Variant

```
Invariant: Any `RepairHint` serializable by `Serialize` round-trips through JSON
  with the same `variant` field:
    let hint = <construct RepairHint>;
    let json = serde_json::to_string(&hint).unwrap();
    let deserialized: RepairHint = serde_json::from_str(&json).unwrap();
    assert_eq!(hint.variant(), deserialized.variant());

Strategy: Construct each of the 7 RepairHint variants with valid fields,
  serialize, deserialize, compare variant.
Anti-invariant: `RepairHintReadWire` with unknown `variant` field should reject.
```

---

## 5. Fuzz Targets

Not in scope for this bead. The v1 killer demo exercises no untrusted input parsing
(only controlled fixture files). Fuzz targets for `normalize_clippy_jsonl` and
`repair_hint_from_wire` are deferred to v1.5+ (see §15.18 roadmap).

---

## 6. Kani Harnesses

Not in scope for this bead. Per v1-spec §15.18, Kani lanes are deferred to v1.5+.
No bounded model checking harnesses are planned.

---

## 7. Mutation Checkpoints

The following production functions have critical branches that mutation testing should
catch. These are **checkpoint assertions** — not to be run by the test-writer, but
documented so `cargo mutants` can be run after delivery.

### MC1: `reject_kind_from_empty` branch coverage

```
Critical mutation: flip `code_empty` condition in the match arm
  (false, true) → should return CodeOnly, not GateOnly
Must be caught by: bad_fixture_reject_kind_is_code_only (B5)

Critical mutation: swap the two bool parameters
  (code_empty, gate_empty) → (gate_empty, code_empty)
Must be caught by: bad_fixture_reject_kind_is_code_only (B5)
```

### MC2: `check_reject_not_empty` guard

```
Critical mutation: remove the guard that rejects empty-reject reports
  (delete the `check_reject_not_empty` call in `Report::reject`)
Must be caught by: report_reject_separates_code_findings_from_gate_failures (B12)
  — this test constructs a Reject with known non-empty collections.
```

---

## 8. Combinatorial Coverage Matrix

### Unit Tests: Domain Value Construction (in `killer_demo.rs` or `titania-core/tests/`)

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| `report_reject_code_only` | 2 findings + 0 failures | `Report::Reject` with `reject_kind() == Some(CodeOnly)` | Unit |
| `report_reject_gate_only` | 0 findings + 1 failure | `Report::Reject` with `reject_kind() == Some(GateOnly)` | Unit |
| `report_reject_mixed` | 2 findings + 1 failure | `Report::Reject` with `reject_kind() == Some(Mixed)` | Unit |
| `report_reject_empty_both` | 0 findings + 0 failures | `Err(ReportError::EmptyReject)` | Unit |
| `finding_construct_with_ruleid` | Valid RuleId string | `Ok(Finding)` with correct `rule_id` | Unit |
| `finding_reject_vs_informational` | Same inputs, different effect | Two findings differ only in `effect` | Unit |
| `repair_hint_use_iterator_pipeline` | Valid suggestion string | `Ok(RepairHint::UseIteratorPipeline { suggestion })` | Unit |
| `repair_hint_requires_human_review` | Valid note string | `Ok(RepairHint::RequiresHumanReview { note })` | Unit |

### Integration Tests: CLI Behavior (in `killer_demo.rs`)

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| `bad_fixture_rejects_with_code_findings` | `for` loop + `.unwrap()` fixture | Exit 1, JSON `variant == "reject"`, 2 code findings, 0 gate failures | E2E |
| `bad_fixture_has_func_loops_for_finding` | Bad fixture, find by rule_id | Finding at AstGrep lane, `effect == "reject"`, repair == `UseIteratorPipeline` | E2E |
| `bad_fixture_has_clippy_unwrap_used_finding` | Bad fixture, find by rule_id | Finding at Clippy lane, `effect == "reject"`, repair == `RequiresHumanReview` | E2E |
| `bad_fixture_findings_have_correct_repair_hints` | Bad fixture, check repair hints | Both findings have expected `RepairHint` variants | E2E |
| `bad_fixture_gate_failures_empty` | Bad fixture | `gate_failures` empty, no `Failed` outcomes in `per_lane` | E2E |
| `bad_fixture_reject_kind_is_code_only` | Bad fixture | `code_findings` > 0 AND `gate_failures` == 0 → `CodeOnly` | E2E |
| `repaired_fixture_passes_with_receipt` | Iterator pipeline fixture | Exit 0, JSON `variant == "pass"` | E2E |
| `repaired_fixture_receipt_has_schema_version_one` | Repaired fixture | `receipt.schema_version == 1`, `receipt.scope == "Edit"` | E2E |
| `repaired_fixture_receipt_contains_all_digests` | Repaired fixture | 4 non-empty hex digests, all distinct | E2E |
| `repaired_fixture_per_lane_has_seven_entries` | Repaired fixture | `per_lane.len() == 7` | E2E |
| `repaired_fixture_per_lane_contains_all_edit_lanes` | Repaired fixture | All 7 lane names present, no extras | E2E |
| `missing_cargo_toml_produces_input_error` | Empty directory | Exit 3, empty stdout, stderr contains "InputError" | E2E |
| `report_reject_gate_only` | In-memory 0 findings + 1 Dylint gate failure | `gate_failures` len 1, `code_findings` len 0, `reject_kind` == `"gate_only"` | Integration |
| `report_reject_mixed` | In-memory 1 code finding + 1 Compile gate failure | `code_findings` len 1 (AstGrep), `gate_failures` len 1 (Compile), `reject_kind` == `"mixed"` | Integration |
| `mixed_report_separates_code_findings_from_gate_failures` | In-memory 2 code findings (AstGrep + Clippy) + 1 Dylint gate failure | `code_findings` lanes are functional only; `gate_failures` lanes are infrastructure only; no cross-contamination | Integration |

### Property Tests: Invariants
| Scenario | Property | Strategy | Layer |
|----------|----------|----------|-------|
| `reject_kind_from_empty_exhaustive` | All 4 bool combinations map correctly | Exhaustive `bool x bool` | Property |
| `repair_hint_variant_roundtrip` | Serialized variant survives JSON round-trip | One per variant (7 total) | Property |

---

## 9. Pre-Conditions: RED Before Fixture/Test Implementation

### Tests that MUST be RED before fixture/test code is written:

| Test | What must compile-fail first | Command to verify RED |
|------|------------------------------|----------------------|
| `bad_fixture_rejects_with_code_findings` | `killer_demo.rs` doesn't exist; fixtures don't exist | `cargo test -p titania-check --test killer_demo` → compile error (file not found) |
| `bad_fixture_has_func_loops_for_finding` | Same — fixture directory absent | `cargo test -p titania-check --test killer_demo` → compile error |
| `bad_fixture_has_clippy_unwrap_used_finding` | Same | `cargo test -p titania-check --test killer_demo` → compile error |
| `repaired_fixture_passes_with_receipt` | Same | `cargo test -p titania-check --test killer_demo` → compile error |
| `repaired_fixture_receipt_contains_all_digests` | Same | `cargo test -p titania-check --test killer_demo` → compile error |

### Tests that can be written GREEN (compile) immediately:

| Test | Why | Command to verify |
| `report_reject_gate_only` | Uses in-memory `Finding`/`LaneFailure` constructs | `cargo test -p titania-check --test killer_demo report_reject_gate_only` → FAILS (RejectKind classification not wired) |
| `report_reject_mixed` | Uses in-memory `Finding`/`LaneFailure` constructs | `cargo test -p titania-check --test killer_demo report_reject_mixed` → FAILS (RejectKind classification not wired) |
| `mixed_report_separates_code_findings_from_gate_failures` | Uses in-memory `Finding`/`LaneFailure` constructs | `cargo test -p titania-check --test killer_demo mixed_report_separates` → FAILS (assembly not yet wired) |

---

## 10. Open Questions

1. ~~**Dylint lane behavior in test workspaces**~~: ~~Resolved by F4~~. ~~Dylint artifact mock added to B1 and B6 Given clauses.~~ **RESOLVED**: Mock the Dylint artifact file at `.titania/out/edit/Dylint.lane-artifact.json` with variant `"clean"` in both B1 and B6 fixtures. See B1 and B6 Given clauses.
2. **Cargo.lock presence**: The clippy lane requires `Cargo.lock` to be present. The `package()` helper in `cli_dispatch.rs` does not generate one. The killer demo fixtures must either:
   - Create `Cargo.lock` (even empty/minimal), or
   - Run `cargo generate-lockfile` before invoking `titania-check`, or
   - The `titania-check` CLI itself handles missing `Cargo.lock` gracefully.
   
   **Recommendation**: Write fixtures with a minimal `Cargo.lock` or rely on `titania-check` to call `cargo generate-lockfile` if needed (already a known gap noted in research-notes.md line 123).

3. **Positional target path semantics**: The contract says `titania-check --scope edit --emit json`. The existing `cli_dispatch.rs` tests use `run_in(&cwd, args)` to set the working directory. The killer demo should follow the same pattern — no positional target path needed; cwd determines the workspace root.

4. ~~**Dylint lane artifact for repaired fixture**~~: ~~Resolved~~. **RESOLVED**: Dylint mock added to B6 Given clause (same pattern as Q1). See B6.

5. **Fixtures location**: The contract (AC-8) specifies `fixtures/strict_ai_loop_unwrap/bad/` and `fixtures/strict_ai_loop_unwrap/repaired/`. These should be at the crate root: `crates/titania-check/fixtures/strict_ai_loop_unwrap/`. Confirm this path with the contract team before writing fixtures.

6. **Policy digest computation**: The policy digest requires a policy config file to exist. The fixture workspaces must have a `.titania/` directory with the policy profile. Confirm the exact policy file path expected by `titania-policy` in test workspaces.

---

## Evidence Commands

| Command | Purpose |
|---------|---------|
| `cargo test -p titania-check --test killer_demo` | Run all killer demo tests |
| `cargo test -p titania-check --test killer_demo bad_fixture` | Bad fixture tests only |
| `cargo test -p titania-check --test killer_demo repaired_fixture` | Repaired fixture tests only |
| `cargo test -p titania-check --test killer_demo missing_cargo_toml` | Input error test only |
| `cargo test -p titania-check --test killer_demo report_reject_gate_only` | GateOnly rejection test only |
| `cargo test -p titania-check --test killer_demo report_reject_mixed` | Mixed rejection test only |
| `cargo test -p titania-check --test killer_demo mixed_report_separates` | Mixed-case separation test only |

---

## Non-Goals

- No production Rust source changes (`crates/titania-core/`, `crates/titania-lanes/`, `crates/titania-aggregate/`, `crates/titania-check/src/`)
- No changes to existing tests (`cli_dispatch.rs`, `aggregate_cli.rs`, `report_assembly.rs`)
- No proof artifacts (Verus, Kani, Flux)
- No fuzz targets or mutation testing execution (documented as checkpoints only)
- No changes to policy configuration files
- No changes to moon configuration or task graphs

---

## Repair Notes (F1–F4 → Plan Changes)

| Finding | Severity | Plan Change |
|---------|----------|-------------|
| **F1** GateOnly/Mixed RejectKind untested | BLOCKER | Added **B13** (`report_reject_gate_only`) and **B14** (`report_reject_mixed`) integration scenarios with in-memory `Finding`/`LaneFailure` constructs. Updated BDD Scenarios (Section 3), Behavior Inventory (Section 1, now B13–B14), Combinatorial Coverage Matrix (Integration + Property Tests), Pre-Conditions, and Evidence Commands. Updated Summary count: behavior tests 11→14, integration tests 6→3. |
| **F2** B12 redundant with B1 | ERROR | Reframed **B12** (`mixed_report_separates_code_findings_from_gate_failures`) to test the **Mixed** case instead of CodeOnly: in-memory `Finding` + `LaneFailure` constructs asserting that functional lanes stay in `code_findings` and infrastructure lanes stay in `gate_failures`. Updated BDD Scenarios (Section 3), Behavior Inventory (B12), Combinatorial Coverage Matrix, and Pre-Conditions. |
| **F3** P1 proptest strategy generates wrong type (`Vec<u8>`) | ERROR | **Removed P1** entirely. P2's exhaustive enumeration of all 4 `(code_empty, gate_empty)` boolean combinations subsumes any random sampling. Updated Proptest Invariants (Section 4, now P2 and P3 only), Summary count: proptest invariants 3→2. |
| **F4** Dylint mock not in Given clauses | MINOR | Added Dylint artifact mock instruction to **B1 Given** (line 76) and **B6 Given** (line 175). Marked Open Questions Q1 and Q4 as resolved (Section 10). Updated Behavior Inventory count to reflect integration scope. |
