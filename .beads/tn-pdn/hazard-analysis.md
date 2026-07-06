# tn-pdn — Hazard Analysis

## H1: Code Finding / Gate Failure Confusion

**Type**: Semantic invariant violation
**Severity**: Critical
**Scenario**: A finding from the AstGrep lane appears in `gate_failures` instead of `code_findings`, or a lane infrastructure failure appears in `code_findings`. The killer demo asserts `gate_failures` is empty for the bad fixture.
**Prevention**: `assemble_report` categorizes by outcome variant: `LaneOutcome::Findings { findings }` → `code_findings`; `LaneOutcome::Failed { LaneFailure }` → `gate_failures`. These are enum-discriminant-based — impossible to cross-pollute at the type level.
**Detection**: Test asserts `gate_failures` is empty and `code_findings` contains exactly `FUNC_LOOPS_FOR` + `CLIPPY_UNWRAP_USED`.
**Proof seed**: `code_findings_gate_failures_separation`

## H2: Empty Reject Invariant Violation

**Type**: Rust-core invariant
**Severity**: High
**Scenario**: `Report::Reject` produced with both `code_findings` and `gate_failures` empty. The aggregator would emit a semantically meaningless report.
**Prevention**: `check_reject_not_empty` in `report.rs` returns `ReportError::EmptyReject`. `Report::reject()` calls this before constructing the variant.
**Detection**: Compile-time — the `Report` constructor is private; only `Report::reject` and `assemble_report` can produce it, both guarded.
**Proof seed**: `reject_non_empty_invariant`

## H3: Schema Version Drift

**Type**: Release/API
**Severity**: Medium
**Scenario**: A receipt serializes with `schema_version != 1`, causing downstream consumers to reject it.
**Prevention**: `QualityReceiptV1::new` hardcodes `schema_version = 1`. Deserialization via `Wire` validates `schema_version == RECEIPT_SCHEMA_VERSION` and returns a serde error if not. Callers cannot override.
**Detection**: `json_roundtrip.rs` golden fixture `REPORT_PASS_JSON` contains `"schema_version":1`.
**Proof seed**: `receipt_schema_version_constant`

## H4: Dylint Lane Infra-Failure in Test Workspaces

**Type**: Infrastructure
**Severity**: Medium
**Scenario**: The Dylint lane cannot find the `.so` library in a test `TempDir` workspace, producing `LaneFailure::Infra` that lands in `gate_failures` and makes the bad fixture report `Mixed` instead of `CodeOnly`.
**Prevention**: `aggregate_cli.rs` precedent at line 98 — missing lane artifacts produce infra failure reports. The test must account for this by either ensuring Dylint loads or asserting that infra failures from Dylint are acceptable in the bad fixture's `gate_failures` count.
**Detection**: Test observes actual Dylint lane behavior; if infra failure appears, the killer demo contract must be updated to allow it.
**Proof seed**: `dylint_infra_tolerance`

## H5: Clippy Cargo.lock Dependency

**Type**: Infrastructure
**Severity**: Low
**Scenario**: The clippy lane requires a `Cargo.lock` file; if missing, clippy may error, producing a gate failure instead of a code finding.
**Prevention**: `cli_dispatch.rs` lines 198-203 show Cargo.lock is written as part of workspace packaging. The test's `package()` helper must ensure a lock file exists.
**Detection**: Test verifies clippy lane produces `CLIPPY_UNWRAP_USED` finding, not a `LaneFailure`.
**Proof seed**: `clippy_cargo_lock_present`

## H6: RepairHint Mismatch

**Type**: Semantic invariant violation
**Severity**: High
**Scenario**: `FUNC_LOOPS_FOR` maps to the wrong `RepairHint` variant (e.g., `RequiresHumanReview` instead of `UseIteratorPipeline`), causing the killer demo assertion to fail.
**Prevention**: ast-grep YAML `metadata.repair_hint` field drives `repair_hint()` in `rules/mod.rs`. The rule catalog is compile-time embedded via `include_str!`. Each rule's repair hint is fixed in YAML.
**Detection**: `embedded_rule_ids()` + `rules/mod.rs::repair_hint(rule_id)` test verifies mapping.
**Proof seed**: `repair_hint_mapping`

## H7: False Positive — Repaired Fixture Triggers a Finding

**Type**: Hostile input
**Severity**: Medium
**Scenario**: The repaired fixture (iterator pipeline, no unwrap) accidentally triggers another rule — e.g., `FUNC_PRINT_STDOUT` if it contains a `println!`, or `FUNC_UNWRAP_OR` if it uses `.unwrap_or()`.
**Prevention**: Fixture `lib.rs` must be minimal: only the iterator pipeline. No `println!`, no `unwrap_or`, no wildcards. Only the essential code.
**Detection**: Repaired fixture produces `Report::Pass`. If any finding appears, it's a fixture defect.
**Proof seed**: `repaired_fixture_minimality`

## H8: RuleId Invalid Character

**Type**: Rust-core invariant
**Severity**: Low
**Scenario**: A rule ID produced by normalization contains lowercase letters or no underscore, failing `RuleId::new`.
**Prevention**: `RuleId::new` returns `Result`. `clippy_normalizer` uses `RuleId::new(&format!("CLIPPY_{}", lint.to_ascii_uppercase()))` — always valid prefix + uppercase. Ast-grep rules use hardcoded uppercase IDs.
**Detection**: Compile-time — `RuleIdError` is returned, not panicked.
**Proof seed**: `rule_id_validation`

## H9: Per-Lane Evidence Gap

**Type**: Bounded state
**Severity**: Medium
**Scenario**: `Report::Pass` produced with empty `per_lane`, violating the `check_per_lane_not_empty` invariant.
**Prevention**: `Report::reject` / `Report::pass` smart constructors call `check_per_lane_not_empty`. `assemble_report` always includes all scope lane outcomes in `per_lane`.
**Detection**: `json_roundtrip.rs` golden fixture `REPORT_PASS_JSON` has `per_lane` array with entries.
**Proof seed**: `pass_per_lane_non_empty`

## H10: RejectKind Misclassification

**Type**: Semantic invariant violation
**Severity**: Low
**Scenario**: The bad fixture produces one code finding and one gate failure, making it `Mixed` instead of `CodeOnly`. The test asserts `CodeOnly`.
**Prevention**: `reject_kind_from_empty` is a `const fn` computed from collection emptiness — no logic, just boolean checks.
**Detection**: Test asserts `report.reject_kind() == RejectKind::CodeOnly`.
**Proof seed**: `reject_kind_correctness`

## Hazard Summary

| Hazard | Severity | Lane Profile |
|--------|----------|-------------|
| H1: Finding/failure confusion | Critical | Rust-local implementation |
| H2: Empty reject | High | Rust-local implementation |
| H3: Schema version drift | Medium | Rust-local implementation |
| H4: Dylint infra failure | Medium | Hostile input / test environment |
| H5: Clippy Cargo.lock | Low | Hostile input / test environment |
| H6: RepairHint mismatch | High | Rust-local implementation |
| H7: False positive repaired | Medium | Hostile input |
| H8: RuleId invalid char | Low | Rust-local implementation |
| H9: Per-lane evidence gap | Medium | Rust-local implementation |
| H10: RejectKind misclassification | Low | Rust-local implementation |
