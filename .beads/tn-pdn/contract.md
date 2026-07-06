# tn-pdn — Contract

## User-Visible Workflow

The user runs `titania-check --scope edit --emit json` against a Rust project. The tool:

1. Parses `--scope edit` → runs the 7 Edit lanes (Fmt, Compile, Clippy, AstGrep, Dylint, PanicScan, PolicyScan)
2. Each lane produces a typed `LaneOutcome` (Clean, Findings, Failed, or Skipped)
3. The aggregate combines outcomes into a `Report`
4. The report is emitted as JSON

The killer demo proves two behaviors:
- **Bad code** (for-loop + `.unwrap()`) → `Report::Reject` with specific code findings
- **Repaired code** (iterator pipeline, no unwrap) → `Report::Pass` with schema_version=1 receipt

## Acceptance Behaviors

### AC-1: Bad Fixture Rejects with Code Findings

**Behavior**: A Rust file containing a `for` loop and `.unwrap()` is rejected by `titania-check --scope edit`.

**Source refs**:
- `titania-lanes/rules/functional.yml` line 5-20: `FUNC_LOOPS_FOR` rule (ast-grep)
- `titania-lanes/src/clippy_normalizer.rs` line 176: `CLIPPY_UNWRAP_USED` mapping
- `titania-core/src/report.rs` line 47-54: `Report::Reject { code_findings, gate_failures }`
- `titania-core/src/finding.rs` line 33-40: `Finding` struct with `lane`, `rule_id`, `repair`

**Test obligation**: `crates/titania-check/tests/killer_demo.rs` — bad fixture test asserts:
- `report.variant == "reject"`
- `code_findings` contains finding with `rule_id == "FUNC_LOOPS_FOR"` and `effect == "reject"`
- `code_findings` contains finding with `rule_id == "CLIPPY_UNWRAP_USED"` and `effect == "reject"`
- `gate_failures` is empty
- `report.reject_kind() == RejectKind::CodeOnly`

**Evidence command**: `cargo test -p titania-check --test killer_demo bad_fixture_rejects_with_code_findings`

### AC-2: Bad Fixture Repair Hints Correct

**Behavior**: `FUNC_LOOPS_FOR` finding has `RepairHint::UseIteratorPipeline`; `CLIPPY_UNWRAP_USED` has `RepairHint::RequiresHumanReview`.

**Source refs**:
- `titania-lanes/rules/functional.yml` line 20: `repair_hint: UseIteratorPipeline`
- `titania-lanes/src/clippy_normalizer.rs` line 176: maps to `CLIPPY_UNWRAP_USED` with default `RequiresHumanReview`
- `titania-core/src/finding/repair_hint.rs` lines 24-28, 52-55: `UseIteratorPipeline` and `RequiresHumanReview` variants

**Test obligation**: killer_demo test asserts each finding's `repair` variant matches.

**Evidence command**: Same as AC-1 (repair hints are part of the finding assertion)

### AC-3: Repaired Fixture Passes with Receipt

**Behavior**: A Rust file with an iterator pipeline and no `.unwrap()` passes with `Report::Pass`.

**Source refs**:
- `titania-core/src/report.rs` lines 37-42: `Report::Pass { receipt, per_lane }`
- `titania-core/src/v1_receipt.rs` lines 78-93: `QualityReceiptV1` with `schema_version: u16`
- `titania-core/src/v1_receipt.rs` line 120: `RECEIPT_SCHEMA_VERSION: u16 = 1`
- `titania-core/src/gate_scope.rs` lines 31-39: `EDIT_LANES` = 7 lanes

**Test obligation**: killer_demo test asserts:
- `report.variant == "pass"`
- `receipt.schema_version == 1`
- `receipt.scope == GateScope::Edit`
- `receipt` contains `source_digest`, `cargo_lock_digest`, `policy_digest`, `toolchain_digest`
- `per_lane` contains exactly 7 `LaneOutcome` entries (one per Edit lane)

**Evidence command**: `cargo test -p titania-check --test killer_demo repaired_fixture_passes_with_receipt`

### AC-4: Findings in code_findings, Not gate_failures

**Behavior**: The two code findings (`FUNC_LOOPS_FOR`, `CLIPPY_UNWRAP_USED`) appear in `Report::Reject.code_findings`, NOT in `gate_failures`. Gate failures remain empty for the bad fixture.

**Source refs**:
- `titania-core/src/report.rs` lines 47-54: `Reject` has separate `code_findings` and `gate_failures` fields
- `titania-core/src/outcome.rs` lines 122-140: `LaneOutcome` discriminates `Findings` from `Failed`

**Test obligation**: killer_demo test asserts `report.gate_failures.is_empty()` for the bad fixture.

**Evidence command**: Same as AC-1

### AC-5: RejectKind Classification

**Behavior**: Bad fixture with only code findings → `RejectKind::CodeOnly`.

**Source refs**:
- `titania-core/src/report.rs` lines 17-27: `RejectKind { CodeOnly, GateOnly, Mixed }`
- `titania-core/src/report.rs` lines 156-161: `reject_kind_for()` const fn

**Test obligation**: killer_demo test asserts `report.reject_kind() == RejectKind::CodeOnly`.

**Evidence command**: Same as AC-1

### AC-6: Receipt Contains All 4 Digests

**Behavior**: The pass receipt contains `source_digest`, `cargo_lock_digest`, `policy_digest`, `toolchain_digest`.

**Source refs**:
- `titania-core/src/v1_receipt.rs` lines 84-90: all 4 digest fields
- `titania-core/src/digest.rs`: `Digest` type (Blake3)

**Test obligation**: killer_demo test asserts each digest field is present and non-zero.

**Evidence command**: Same as AC-3

### AC-7: Per-Lane Outcomes for All 7 Edit Lanes

**Behavior**: The pass report's `per_lane` contains outcomes for all 7 Edit lanes: Fmt, Compile, Clippy, AstGrep, Dylint, PanicScan, PolicyScan.

**Source refs**:
- `titania-core/src/gate_scope.rs` lines 31-39: `EDIT_LANES` array

**Test obligation**: killer_demo test asserts `per_lane.len() == 7` and all 7 `Lane` variants are present.

**Evidence command**: Same as AC-3

### AC-8: Fixture File Scope

**Behavior**: Only the 5 allowed files are created/modified:
- `fixtures/strict_ai_loop_unwrap/bad/Cargo.toml`
- `fixtures/strict_ai_loop_unwrap/bad/src/lib.rs`
- `fixtures/strict_ai_loop_unwrap/repaired/Cargo.toml`
- `fixtures/strict_ai_loop_unwrap/repaired/src/lib.rs`
- `crates/titania-check/tests/killer_demo.rs`

**Source refs**: None (policy constraint, not code)

**Test obligation**: No code test; implementation discipline. The contract specifies this file set as the ONLY allowed changes.

## Report Invariants

1. **Reject non-empty**: `Report::Reject` must have at least one `code_finding` or `gate_failure` (enforced by `check_reject_not_empty`)
2. **Pass per_lane non-empty**: `Report::Pass` must have non-empty `per_lane` (enforced by `check_per_lane_not_empty`)
3. **Finding ownership**: Analysis lane findings → `code_findings`; infrastructure failures → `gate_failures`
4. **RepairHint applicability**: Each finding's `RepairHint` must be applicable to its `RuleId`
5. **Schema version**: `QualityReceiptV1` always has `schema_version = 1`
6. **RejectKind consistency**: `RejectKind` is derived from collection emptiness, never manually set

## Evidence Commands

| Command | Purpose |
|---------|---------|
| `cargo test -p titania-check --test killer_demo` | Run all killer demo tests |
| `cargo test -p titania-check --test killer_demo bad_fixture` | Bad fixture tests only |
| `cargo test -p titania-check --test killer_demo repaired_fixture` | Repaired fixture tests only |

## Non-Goals

- No production Rust source changes in `crates/titania-core/`, `crates/titania-lanes/`, `crates/titania-aggregate/`, `crates/titania-check/src/`
- No changes to existing tests in `crates/titania-check/tests/cli_dispatch.rs`, `crates/titania-check/tests/aggregate_cli.rs`, `crates/titania-aggregate/tests/report_assembly.rs`
- No proof artifacts, no Verus/Kani/Flux code, no test changes beyond `killer_demo.rs`
- No CLI binary changes beyond what the existing dispatch shell supports
- No policy configuration changes
