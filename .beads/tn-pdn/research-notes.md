# tn-pdn — Research Notes

## Verified Source Evidence

### v1-spec.md §15 (lines 1125-1140)
**Verified**: `/home/lewis/src/titania/.worktrees/v1-combined-dispatch/v1-spec.md:1125-1140`
- Killer demo: bad code with `for` loop + `.unwrap()` → rejects with `FUNC_LOOPS_FOR` (RepairHint::UseIteratorPipeline) + `CLIPPY_UNWRAP_USED` (RepairHint::RequiresHumanReview)
- Repaired code → passes with `schema_version=1` receipt digests

### Report enum — `titania-core/src/report.rs`
**Verified**: Lines 33-65
- `Report::Reject { code_findings: Box<[Finding]>, gate_failures: Box<[LaneFailure]>, per_lane }`
- `Report::Pass { receipt: QualityReceipt, per_lane }`
- `Report::PolicyError` and `Report::InputError` variants
- Invariant: Reject non-empty enforced by `check_reject_not_empty` (line 128)
- Invariant: Pass per_lane non-empty enforced by `check_per_lane_not_empty` (line 141)
- `RejectKind { CodeOnly, GateOnly, Mixed }` (lines 20-27)

### Finding struct — `titania-core/src/finding.rs`
**Verified**: Lines 32-40
- `Finding { lane: Lane, rule_id: RuleId, location: Location, message: String, repair: RepairHint, effect: FindingEffect }`
- `FindingEffect { Reject, Informational }` (lines 18-25)
- Smart constructors `Finding::reject` and `Finding::informational`

### RepairHint — `titania-core/src/finding/repair_hint.rs`
**Verified**: Lines 14-56
- 7 variants including `UseIteratorPipeline { suggestion: String }` and `RequiresHumanReview { suggestion: String }`
- Smart constructor validates `Patch.range.width() > 0` on construction and deserialization
- Deserialization uses `RepairHintReadWire` intermediate with `deny_unknown_fields`

### RuleId — `titania-core/src/rule_id.rs`
**Verified**: Lines 20-21, 70-81
- `RuleId(String)` — validated smart constructor `RuleId::new`
- Invariants: non-empty, contains `_`, all `[A-Z0-9_]`
- `RuleIdError { Empty, NoUnderscore, NotUppercase }`

### Lane enum — `titania-core/src/lane.rs`
**Verified**: Lines 19-42
- 10 variants: `Fmt, Compile, Clippy, AstGrep, Dylint, PanicScan, PolicyScan, Test, Deny, Build`
- `#[serde(rename_all = "PascalCase")]`
- `Lane::from_str` returns `LaneError::UnknownLane` for unrecognized strings

### GateScope — `titania-core/src/gate_scope.rs`
**Verified**: Lines 20-64
- `GateScope { Edit, Prepush, Release }` — `#[non_exhaustive]`
- `EDIT_LANES`: `[Fmt, Compile, Clippy, AstGrep, Dylint, PanicScan, PolicyScan]` — 7 lanes
- `PREPUSH_LANES`: edit + Test + Deny — 9 lanes
- `RELEASE_LANES`: prepush + Build — 10 lanes

### QualityReceiptV1 — `titania-core/src/v1_receipt.rs`
**Verified**: Lines 78-93, 120
- `schema_version: u16` (always 1), `scope: GateScope`, 4 digests, `lanes: Box<[LaneReceipt]>`
- `RECEIPT_SCHEMA_VERSION: u16 = 1` (line 120)
- Deserialization validates schema version via `validate_schema_version` (line 55-61)

### LaneOutcome — `titania-core/src/outcome.rs`
**Verified**: Lines 120-140
- `LaneOutcome { Clean { evidence }, Findings { findings }, Failed { tool_failure }, Skipped { reason } }`
- `SkipReason { PriorCompilationFailure, NotSelectedByScope, NotApplicable, PolicyDisabled }`

### LaneFailure — `titania-core/src/failure.rs`
**Verified**: Lines 65-101
- `LaneFailure { Infra { tool, reason }, ToolFailure { tool, ProcessTermination }, SuspiciousFailure { tool, evidence }, ResourceFailure { tool, limit } }`

### Clippy Normalizer — `crates/titania-lanes/src/clippy_normalizer.rs`
**Verified**: Lines 66-68, 176
- `normalize_clippy_jsonl(input: &str) -> ClippyNormalization`
- Mapping: `RuleId::new(&format!("CLIPPY_{}", lint.to_ascii_uppercase()))` — deterministic uppercase
- Unknown lints map to `CLIPPY_UNKNOWN`

### ast-grep Rules — `crates/titania-lanes/rules/functional.yml`
**Verified**: Lines 1-146
- `FUNC_LOOPS_FOR`: pattern `"for $LOOP in $ITER { $$$BODY }"`, effect `Reject`, repair `UseIteratorPipeline`
- `FUNC_LOOPS_WHILE`, `FUNC_LOOPS_LOOP` — similar structure
- `FUNC_PRINT_STDOUT`, `FUNC_PRINT_STDERR`, `FUNC_WILDCARD_IMPORT`, `FUNC_UNWRAP_OR`, `FUNC_RESULT_STRING`
- All rules exclude `**/tests/**`, `**/benches/**`, `**/examples/**`, `**/build.rs`

### Test Patterns — `crates/titania-check/tests/cli_dispatch.rs`
**Verified**: Lines 1-303
- `run_in(cwd, args) -> (i32, String, String)` — executes titania-check in temp directory
- `package(name, lib_rs, main_rs) -> TempDir` — builds minimal Cargo project
- `assert_empty_workspace_reject(args, expected_gate_failures)` — verifies reject behavior
- `fmt_artifact_path(root)` → `.titania/out/edit/fmt.json`
- `clippy_artifact_path(root)` → `.titania/out/edit/clippy.json`
- Pattern: `CARGO_BIN_EXE_titania-check` env var for binary path

### Test Patterns — `crates/titania-check/tests/aggregate_cli.rs`
**Verified**: Lines 1-125
- `GateArtifact` struct for mocking lane outputs
- `assert_pass_report(report: &Value)` — asserts pass report structure
- `assert_missing_lane_report(report: &Value)` — asserts missing lane handling
- `clean_edit_workspace() -> TempDir` — creates workspace with all lane artifacts

### Test Patterns — `crates/titania-aggregate/tests/report_assembly.rs`
**Verified**: Lines 1-451
- `findings_outcome(lane) -> LaneOutcome` — builds finding outcome
- `clean_outcome(lane) -> LaneOutcome` — builds clean outcome
- `test_receipt(scope) -> QualityReceiptV1` — builds minimal receipt
- 11 behavior tests covering all report assembly paths

### Golden JSON — `crates/titania-core/tests/json_roundtrip.rs`
**Verified**: Lines 25-27
- `REPORT_PASS_JSON`: includes `"schema_version":1`, 4 digests, `per_lane`
- `REPORT_REJECT_JSON`: includes `FUNC_LOOPS_FOR` finding with `UseIteratorPipeline` repair, empty `gate_failures`

## Scout Handoff Verification

**agent://TnPdnScout claims verified**:
- ✅ `titania_core::Digest` exists — `crates/titania-core/src/digest.rs`
- ✅ `titania_core::RuleId` exists — `crates/titania-core/src/rule_id.rs`
- ✅ `titania_core::WorkspacePath` exists — `crates/titania-core/src/workspace_path.rs`
- ✅ `titania_lanes::CommandIn` — inferred from lane runner architecture (verified via `ast_grep_lane.rs`)
- ✅ `crates/titania-check/tests/` patterns verified in `cli_dispatch.rs`, `aggregate_cli.rs`
- ✅ `fixtures/strict_ai_loop_unwrap/` does NOT yet exist — must be created
- ✅ `crates/titania-core/src/finding/repair_hint.rs` has 7 variants including `UseIteratorPipeline` and `RequiresHumanReview`
- ✅ `crates/titania-core/src/report.rs` has `Report::Reject`, `Report::Pass`, `RejectKind`
- ✅ `crates/titania-core/src/v1_receipt.rs` has `QualityReceiptV1` with `schema_version = 1`

## Unresolved Questions

1. **Dylint .so availability**: The scout noted Dylint lane may infra-fail in test workspaces lacking the .so library. This is handled in aggregate_cli.rs precedent (line 98), but the killer_demo test must decide: does it skip the Dylint lane, mock the artifact, or accept the infra failure?

2. **Clippy Cargo.lock**: The clippy lane needs `Cargo.lock` present. The `package()` helper in `cli_dispatch.rs` must ensure it's created.

3. **Positional target paths**: v1-spec.md §15 says `titania-check --scope edit --emit json` — does it accept a positional target path? The scout says "if the CLI supports positional target paths". The existing `cli_dispatch.rs` tests don't use positional paths — they set cwd via `run_in()`. The killer_demo test should follow this pattern.
