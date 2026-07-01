# tn-03d: v1 Domain Model — Test Plan

> **Bead:** tn-03d
> **Spec:** v1-spec.md §4 (Lane DAG) + §10 (Domain Model)
> **Contract:** contract.md (19 types)
> **Testing Trophy target:** ~60% integration, ~30% unit, ~5% e2e, ~5% static

---

## Test File Layout

| File | Category | Purpose |
|------|----------|---------|
| `crates/titania-core/tests/tn_03d_serde_roundtrip.rs` | **integration** | Serde round-trip for ALL 19 types |
| `crates/titania-core/tests/tn_03d_unit.rs` | **unit** | Constructor validation, accessor correctness, smart constructor invariants |
| `crates/titania-core/tests/tn_03d_properties.rs` | **unit** | Proptest properties for Lane, Report, Finding, Location invariants |
| `crates/titania-core/tests/tn_03d_deser_rejection.rs` | **integration** | Deserialization rejection for validated types |
| `crates/titania-core/tests/tn_03d_acceptance.rs` | **integration** | Acceptance criteria from bead §4 |
| `crates/titania-core/tests/tn_03d_e2e.rs` | **e2e** | Minimal end-to-end: build a Report::Reject from pieces, round-trip, classify |

Total: ~100 tests across 6 files.

---

## 1. `Lane` — 10 variants, unit enum, FromStr only

### 1.1 Unit: `Lane` (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `lane_all_10_variants_constructible` | Every variant name maps to a Lane | `Lane::Fmt` etc. construct; assert `matches!` |
| U2 | `lane_from_str_exact_pascal_case` | `"Fmt" -> Ok(Fmt)`, `"compile" -> Err`, `"FMT" -> Err` | Exact `Ok(Lane::Fmt)` / `Err(_)` |
| U3 | `lane_from_str_empty_string_rejected` | `""` → `Err` | `matches!(result, Err(LaneError::Unknown(..)))` |
| U4 | `lane_from_str_whitespace_rejected` | `" Fmt "`, `"Fmt\n"` → `Err` | Exact `Err` variant |
| U5 | `lane_to_string_round_trip_all_10` | `from_str(to_string(l)) == Ok(l)` for all 10 | `prop_assert_eq!` in property |
| U6 | `lane_copy_eq_hash_traits` | Lane derives Copy, Eq, Hash | `let h = std::hash::Hasher::finish(&mut h); prop_assert_eq!(h1, h2)` |
| U7 | `lane_debug_format` | `format!("{:?}", Lane::Compile)` contains `"Compile"` | `assert_eq!(format!("{:?}", Lane::Compile), "Compile")` |

### 1.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `lane_serde_all_10_variants_round_trip` | Every lane serializes to PascalCase string, deserializes back equal |
| S2 | `lane_json_is_string_form` | `serde_json::to_value(Lane::Dylint)` is `String("Dylint")` |
| S3 | `lane_serde_deterministic` | Two identical lanes serialize to identical JSON strings |

### 1.3 Deserialization rejection (`tn_03d_deser_rejection.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| R1 | `lane_deserialize_rejects_unknown_variant` | `"NonExistent"` → `Err` |
| R2 | `lane_deserialize_rejects_lowercase_variant` | `"fmt"`, `"compile"` → `Err` |
| R3 | `lane_deserialize_rejects_numeric_variant` | `"123"`, `"Fmt1"` → `Err` |

### 1.4 Property test (`tn_03d_properties.rs`)

| # | Property | Invariant |
|---|----------|-----------|
| P1 | `lane_from_str_to_string_round_trip_all` | `∀ l: Lane → from_str(&to_string(l)) == Ok(l)` |
| P2 | `lane_serde_round_trip_all` | `∀ l: Lane → serde_round_trip(l) == l` |
| P3 | `lane_variants_are_unique_strings` | No two variants share the same serialized string |

---

## 2. `GateScope` — 3 variants + #[non_exhaustive], lanes() method

### 2.1 Unit: `GateScope` (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `gate_scope_all_3_variants_constructible` | Edit, Prepush, Release all construct | `GateScope::Edit`, etc. |
| U2 | `gate_scope_from_str_edit` | `"edit"` → `Ok(Edit)` | Exact `Ok`/`Err` |
| U3 | `gate_scope_from_str_prepush` | `"prepush"` → `Ok(Prepush)` | Exact `Ok`/`Err` |
| U4 | `gate_scope_from_str_release` | `"release"` → `Ok(Release)` | Exact `Ok`/`Err` |
| U5 | `gate_scope_from_str_rejects_unknown` | `"full"`, `"unknown"` → `Err` | Exact `Err` |
| U6 | `gate_scope_lanes_edit_returns_7_in_order` | `edit` = [Fmt, Compile, Clippy, AstGrep, Dylint, PanicScan, PolicyScan] | `assert_eq!(lanes.len(), 7)` + exact ordering |
| U7 | `gate_scope_lanes_prepush_returns_9_in_order` | `prepush` = edit + [Test, Deny] | `assert_eq!(lanes.len(), 9)` + exact ordering |
| U8 | `gate_scope_lanes_release_returns_10_in_order` | `release` = prepush + [Build] | `assert_eq!(lanes.len(), 10)` + exact ordering |
| U9 | `gate_scope_lanes_stable_across_calls` | Same scope → same lane slice each call | `ptr_eq` on the returned `&[Lane]` or `assert_eq!(s1, s2)` |
| U10 | `gate_scope_non_exhaustive_compiles_with_default_arm` | Code compiles with `_ => ...` default match arm | Compilation test (compiletest) |

### 2.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `gate_scope_serde_all_3_variants_round_trip` | Edit/Prepush/Release serialize to snake_case, round-trip equal |
| S2 | `gate_scope_json_is_string_form` | `to_value(GateScope::Release)` is `String("release")` |

### 2.3 Deserialization rejection (`tn_03d_deser_rejection.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| R1 | `gate_scope_deserialize_rejects_unknown_scope` | `"full"`, `"deep"` → `Err` |

---

## 3. `Report` — Pass/Reject/PolicyError/InputError

### 3.1 Unit: `Report` (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `report_pass_constructs_with_valid_args` | Valid receipt + per_lane → `Pass` | `matches!(r, Report::Pass { .. })` + inspect fields |
| U2 | `report_pass_rejects_empty_per_lane` | `per_lane = []` → `Err` | `matches!(result, Err(ReportError::EmptyPerLane))` |
| U3 | `report_reject_with_code_and_gate_constructs` | Non-empty both → `Reject` | `matches!(r, Report::Reject { .. })` |
| U4 | `report_reject_with_only_code_findings` | Non-empty findings, empty failures → `Reject` | `matches!(r, Report::Reject { .. })` |
| U5 | `report_reject_with_only_gate_failures` | Empty findings, non-empty failures → `Reject` | `matches!(r, Report::Reject { .. })` |
| U6 | `report_reject_rejects_both_empty` | `[], []` → `Err` | `matches!(result, Err(ReportError::BothEmpty))` |
| U7 | `report_policy_error_constructs` | Non-empty diagnostics → `PolicyError` | `matches!(r, Report::PolicyError { .. })` |
| U8 | `report_input_error_constructs` | Non-empty diagnostics → `InputError` | `matches!(r, Report::InputError { .. })` |
| U9 | `reject_kind_code_only` | findings non-empty, failures empty → `CodeOnly` | `assert_eq!(r.reject_kind(), Some(RejectKind::CodeOnly))` |
| U10 | `reject_kind_gate_only` | findings empty, failures non-empty → `GateOnly` | Exact match |
| U11 | `reject_kind_mixed` | Both non-empty → `Mixed` | Exact match |
| U12 | `reject_kind_pass_returns_none` | `Pass` → `reject_kind()` returns `None` | `assert!(r.reject_kind().is_none())` — but also assert exact variant |
| U13 | `reject_kind_policy_error_returns_none` | `PolicyError` → `None` | Exact assertion |
| U14 | `reject_kind_input_error_returns_none` | `InputError` → `None` | Exact assertion |

### 3.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `report_pass_serde_round_trip` | Full `Pass` with receipt + 3 per_lane lanes → round-trips |
| S2 | `report_reject_serde_round_trip` | `Reject` with 1 finding + 1 failure → round-trips |
| S3 | `report_policy_error_serde_round_trip` | `PolicyError` with 2 diagnostics → round-trips |
| S4 | `report_input_error_serde_round_trip` | `InputError` with 1 diagnostic → round-trips |

### 3.3 Property test (`tn_03d_properties.rs`)

| # | Property | Invariant |
|---|----------|-----------|
| P1 | `report_reject_never_both_empty` | `∀ c, g: reject(c, g).is_ok() → (!c.is_empty() ∨ !g.is_empty())` |
| P2 | `reject_kind_classification_complete` | `∀ c, g: reject(c, g).ok() → reject_kind() ∈ {Some(CodeOnly), Some(GateOnly), Some(Mixed)}` when at least one is non-empty |
| P3 | `report_variants_are_exhaustive` | Every Report value is exactly one variant |

---

## 4. `Finding` — struct with 6 fields

### 4.1 Unit: `Finding` (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `finding_constructs_with_all_fields` | Valid inputs → Finding | Field-level `assert_eq!(f.lane, Lane::Fmt)` etc. |
| U2 | `finding_lane_accessor` | `f.lane()` returns the constructed lane | `assert_eq!(f.lane(), &Lane::AstGrep)` |
| U3 | `finding_rule_id_accessor` | `f.rule_id()` returns the constructed RuleId | `assert_eq!(f.rule_id().as_str(), "CLIPPY_UNWRAP_USED")` |
| U4 | `finding_location_accessor` | Returns constructed location | `matches!(f.location(), Location::Span { .. })` |
| U5 | `finding_message_accessor` | Returns constructed string | `assert_eq!(f.message(), "use iterators")` |
| U6 | `finding_repair_accessor` | Returns constructed RepairHint | `matches!(f.repair(), RepairHint::UseIteratorPipeline { .. })` |
| U7 | `finding_effect_accessor` | Returns constructed effect | `assert!(matches!(f.effect(), FindingEffect::Reject))` |
| U8 | `finding_clone_preserves_all_fields` | Clone all 6 fields identical | `assert_eq!(f, f.clone())` |

### 4.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `finding_serde_round_trip_all_fields` | Full Finding → JSON → equal Finding |
| S2 | `finding_json_structure` | JSON has keys: lane, rule_id, location, message, repair, effect |
| S3 | `finding_informational_serde_round_trip` | Finding with `Informational` effect round-trips |

### 4.3 Property test (`tn_03d_properties.rs`)

| # | Property | Invariant |
|---|----------|-----------|
| P1 | `finding_serde_round_trip` | `∀ f: serde_round_trip(f) == f` |
| P2 | `finding_lane_matches_constructed_lane` | `f.lane == lane` for any constructed finding |

---

## 5. `FindingEffect` — Reject / Informational (unit enum)

### 5.1 Unit (`tn_03d_unit.rs`)

| # | Test name | Assertion |
|---|-----------|-----------|
| U1 | `finding_effect_both_variants` | Construct both, assert `matches!` |

### 5.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | Assertion |
|---|-----------|-----------|
| S1 | `finding_effect_serde_round_trip` | Both variants serialize to `reject`/`informational`, round-trip |

### 5.3 Property test (`tn_03d_properties.rs`)

| # | Property | Invariant |
|---|----------|-----------|
| P1 | `finding_effect_serde_round_trip_all` | Both variants round-trip |

---

## 6. `Location` — Span / Dependency / Manifest / Workspace / Tool

### 6.1 Unit: `Location` (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `location_span_constructs` | Valid path + lines/cols → `Span` | `matches!(loc, Location::Span { file, .. })` |
| U2 | `location_span_line_start_min_is_1` | `line_start = 0` → `Err` | `matches!(result, Err(LocationError::LineStartZero))` |
| U3 | `location_span_line_start_accepts_1` | `line_start = 1` → `Ok` | Exact `Ok` |
| U4 | `location_span_col_start_zero_accepted` | 0-based columns are valid | Exact `Ok` |
| U5 | `location_span_col_end_zero_accepted` | `col_end = 0` (empty span at start) → valid | Exact `Ok` |
| U6 | `location_dependency_constructs` | `("serde", "1.0")` → `Dependency` | `matches!(loc, Location::Dependency { .. })` |
| U7 | `location_manifest_constructs` | WorkspacePath → `Manifest` | `matches!(loc, Location::Manifest { .. })` |
| U8 | `location_workspace_constructs` | `workspace()` → `Workspace` | `assert!(matches!(loc, Location::Workspace))` |
| U9 | `location_tool_constructs` | `("ast-grep", "0.25")` → `Tool` | `matches!(loc, Location::Tool { .. })` |
| U10 | `location_span_accessor_file` | `loc.span_file()` returns the path | `assert_eq!(loc.as_file(), Ok(&path))` |
| U11 | `location_span_accessor_lines` | Accessors return correct line/col values | `assert_eq!(loc.line_start(), 1)` etc. |

### 6.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `location_span_serde_round_trip` | Span → JSON → equal Span |
| S2 | `location_dependency_serde_round_trip` | Dependency → JSON → equal |
| S3 | `location_manifest_serde_round_trip` | Manifest → JSON → equal |
| S4 | `location_workspace_serde_round_trip` | Workspace → JSON → equal |
| S5 | `location_tool_serde_round_trip` | Tool → JSON → equal |
| S6 | `location_all_variants_json_structure` | Each variant serializes to expected JSON shape |

### 6.3 Deserialization rejection (`tn_03d_deser_rejection.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| R1 | `location_span_deserialize_rejects_line_start_zero` | `"line_start": 0` → `Err` |

### 6.4 Property test (`tn_03d_properties.rs`)

| # | Property | Invariant |
|---|----------|-----------|
| P1 | `location_span_line_start_ge_1` | `∀ span: location.line_start() >= 1` |
| P2 | `location_span_col_non_negative` | `∀ span: location.col_start() >= 0 && col_end() >= 0` |
| P3 | `location_serde_round_trip_all_variants` | Every variant round-trips |
| P4 | `location_variants_distinguishable` | No two variants produce identical JSON |

---

## 7. `RepairHint` — 7 variants

### 7.1 Unit: `RepairHint` (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `repair_hint_all_7_variants_constructible` | Each variant constructs | `matches!` |
| U2 | `repair_hint_patch_with_valid_range_succeeds` | `range.width() > 0` → `Ok` | `assert!(result.is_ok())` + exact variant |
| U3 | `repair_hint_patch_with_zero_width_range_rejected` | `range.width() == 0` → `Err` | `matches!(result, Err(RepairHintError::ZeroWidth))` |
| U4 | `repair_hint_patch_accessor_fields` | `file`, `range`, `replacement` accessible | `assert_eq!` on each |
| U5 | `repair_hint_use_iterator_pipeline_accessor` | `suggestion` accessible | `assert_eq!(hint.as_suggestion(), "use .into_iter()")` |
| U6 | `repair_hint_flatten_nesting_accessor` | `suggestion` accessible | `assert_eq!` |
| U7 | `repair_hint_use_checked_arithmetic_accessor` | `op` accessible | `assert_eq!` |
| U8 | `repair_hint_remove_allow_attribute_accessor` | `attr` accessible | `assert_eq!` |
| U9 | `repair_hint_replace_dependency_accessor` | `from`, `to` accessible | `assert_eq!` |
| U10 | `repair_hint_requires_human_review_accessor` | `note` accessible | `assert_eq!` |

### 7.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `repair_hint_patch_serde_round_trip` | Patch → JSON → equal Patch |
| S2 | `repair_hint_use_iterator_pipeline_serde_round_trip` | All 7 variants round-trip |

### 7.3 Property test (`tn_03d_properties.rs`)

| # | Property | Invariant |
|---|----------|-----------|
| P1 | `repair_hint_serde_round_trip_all` | All 7 variants round-trip |

---

## 8. `LaneOutcome` — Clean / Findings / Failed / Skipped

### 8.1 Unit: `LaneOutcome` (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `lane_outcome_clean_with_valid_exit_constructs` | `Exited { code: 0 }` → `Clean` | `matches!(o, LaneOutcome::Clean { .. })` |
| U2 | `lane_outcome_clean_rejects_nonzero_exit` | `Exited { code: 1 }` → `Err` | `matches!(result, Err(LaneOutcomeError::NonZeroExit))` |
| U3 | `lane_outcome_clean_rejects_timed_out` | `TimedOut` → `Err` | Exact `Err` |
| U4 | `lane_outcome_clean_rejects_spawn_failed` | `SpawnFailed` → `Err` | Exact `Err` |
| U5 | `lane_outcome_findings_with_empty_findings` | `[]` → `Findings` | `matches!(o, LaneOutcome::Findings(f) if f.is_empty())` |
| U6 | `lane_outcome_findings_with_findings` | Non-empty → `Findings` | Exact variant match |
| U7 | `lane_outcome_failed_infra_failure` | `InfraFailure` → `Failed` | `matches!(o, LaneOutcome::Failed(LaneFailure::InfraFailure { .. }))` |
| U8 | `lane_outcome_failed_tool_failure` | `ToolFailure` → `Failed` | Exact variant match |
| U9 | `lane_outcome_failed_resource_failure` | `ResourceFailure` → `Failed` | Exact variant match |
| U10 | `lane_outcome_failed_suspicious_failure` | `SuspiciousFailure` → `Failed` | Exact variant match |
| U11 | `lane_outcome_skipped_prior_compilation_failure` | `PriorCompilationFailure` → `Skipped` | Exact variant match |
| U12 | `lane_outcome_skipped_not_selected_by_scope` | `NotSelectedByScope` → `Skipped` | Exact variant match |
| U13 | `lane_outcome_skipped_not_applicable` | `NotApplicable` → `Skipped` | Exact variant match |
| U14 | `lane_outcome_skipped_policy_disabled` | `PolicyDisabled` → `Skipped` | Exact variant match |

### 8.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `lane_outcome_clean_serde_round_trip` | Clean → JSON → equal Clean |
| S2 | `lane_outcome_findings_serde_round_trip` | Findings → JSON → equal |
| S3 | `lane_outcome_failed_infra_serde_round_trip` | Failed(InfraFailure) → JSON → equal |
| S4 | `lane_outcome_failed_tool_serde_round_trip` | Failed(ToolFailure) → JSON → equal |
| S5 | `lane_outcome_skipped_serde_round_trip_all` | All 4 SkipReason variants round-trip |

---

## 9. `SkipReason` — 4 variants (+ future CacheHit)

### 9.1 Unit (`tn_03d_unit.rs`)

| # | Test name | Assertion |
|---|-----------|-----------|
| U1 | `skip_reason_all_4_variants` | All 4 construct and `matches!` |

### 9.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | Assertion |
|---|-----------|-----------|
| S1 | `skip_reason_all_variants_round_trip` | All 4 serialize/deserialize |

---

## 10. `LaneEvidence` — struct (command, tool_version, exit_status, parsed_result_digest)

### 10.1 Unit (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `lane_evidence_constructs` | Valid fields → struct | `assert_eq!(e.tool_version, "rustfmt 1.84.0")` |
| U2 | `lane_evidence_accessors` | All 4 fields accessible | `assert_eq` on each |

### 10.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `lane_evidence_serde_round_trip` | Full evidence → JSON → equal |

---

## 11. `CommandEvidence` — executable + argv

### 11.1 Unit (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `command_evidence_constructs` | Valid executable + argv → `Ok` | `assert!(result.is_ok())` + exact |
| U2 | `command_evidence_rejects_empty_argv` | `[]` → `Err` | `matches!(result, Err(CommandEvidenceError::EmptyArgv))` |
| U3 | `command_evidence_rejects_argv0_mismatch` | `argv[0] != executable` → `Err` | `matches!(result, Err(CommandEvidenceError::Argv0Mismatch))` |
| U4 | `command_evidence_rejects_single_empty_string_argv` | `[""]` with executable `"cargo"` → `Err` | Exact `Err` |
| U5 | `command_evidence_accessors` | `executable()` and `argv()` return correct values | `assert_eq!` |

### 11.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `command_evidence_serde_round_trip` | Full command evidence → JSON → equal |

---

## 12. `LaneFailure` — InfraFailure / ToolFailure / ResourceFailure / SuspiciousFailure

### 12.1 Unit (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `lane_failure_infra_failure` | `("cargo-fmt", "missing")` → `InfraFailure` | `matches!(f, LaneFailure::InfraFailure { .. })` |
| U2 | `lane_failure_tool_failure` | `("clippy", Exited { 1 })` → `ToolFailure` | Exact variant |
| U3 | `lane_failure_resource_failure` | `("dylint", "timeout")` → `ResourceFailure` | Exact variant |
| U4 | `lane_failure_suspicious_failure` | `("ast-grep", "tampered output")` → `SuspiciousFailure` | Exact variant |
| U5 | `lane_failure_tool_accessor_infra` | `f.tool()` returns `"cargo-fmt"` | `assert_eq!` |

### 12.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `lane_failure_all_4_variants_round_trip` | All 4 variants round-trip |

---

## 13. `ProcessTermination` — Exited / Signaled / TimedOut / MemoryLimitExceeded / SpawnFailed

### 13.1 Unit (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `process_termination_exited_code_0` | `Exited { code: 0 }` constructs | `matches!(t, ProcessTermination::Exited { code: 0 })` |
| U2 | `process_termination_exited_code_1` | `Exited { code: 1 }` constructs | Exact |
| U3 | `process_termination_exited_negative_code` | `Exited { code: -1 }` constructs (any i32 valid) | Exact `Ok` |
| U4 | `process_termination_signaled_valid_signal` | Signal 9 → `Signaled { signal: 9 }` | Exact |
| U5 | `process_termination_signaled_rejects_signal_0` | Signal 0 → `Err` | `matches!(result, Err(ProcessTerminationError::InvalidSignal))` |
| U6 | `process_termination_signaled_rejects_signal_32` | Signal 32 → `Err` (> 31) | Exact `Err` |
| U7 | `process_termination_timed_out` | `TimedOut` constructs | `matches!(t, ProcessTermination::TimedOut)` |
| U8 | `process_termination_memory_limit_exceeded` | Constructs | Exact variant |
| U9 | `process_termination_spawn_failed` | Constructs | Exact variant |
| U10 | `process_termination_exited_accessor` | `t.exit_code()` returns Some(code) | `assert_eq!(t.exit_code(), Some(42))` |
| U11 | `process_termination_exited_accessor_none_for_other_variants` | `TimedOut.exit_code()` returns `None` | Exact |
| U12 | `process_termination_signal_accessor` | `t.signal()` returns Some(9) for Signaled | Exact |

### 13.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `process_termination_all_5_variants_round_trip` | All 5 variants serialize/deserialize |
| S2 | `process_termination_exited_serde_preserves_code` | Code value preserved through round-trip |

---

## 14. `RejectKind` — CodeOnly / GateOnly / Mixed

### 14.1 Unit (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `reject_kind_all_3_variants` | All 3 constructible | `matches!` |

### 14.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `reject_kind_all_3_variants_round_trip` | All 3 serialize/deserialize |

---

## 15. `QualityReceipt` — v1 spec version (schema_version: u16)

### 15.1 Unit (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `quality_receipt_constructs_with_valid_args` | Valid schema_version + scope + digests + lanes → `Ok` | `matches!(r, QualityReceipt { .. })` |
| U2 | `quality_receipt_rejects_wrong_schema_version` | `schema_version != 1` → `Err` | `matches!(result, Err(QualityReceiptError::UnsupportedSchemaVersion))` |
| U3 | `quality_receipt_schema_version_accessor` | Returns `1u16` | `assert_eq!(r.schema_version(), 1)` |
| U4 | `quality_receipt_scope_accessor` | Returns constructed scope | `assert_eq!(r.scope(), &GateScope::Edit)` |
| U5 | `quality_receipt_lanes_accessor` | Returns the lanes slice | `assert_eq!(r.lanes().len(), 3)` |
| U6 | `quality_receipt_source_digest_accessor` | Returns digest | Exact match |
| U7 | `quality_receipt_empty_lanes_allowed` | Empty LaneReceipt list → `Ok` (no lanes ran yet) | `assert!(result.is_ok())` + exact variant |

### 15.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `quality_receipt_serde_round_trip` | Full receipt → JSON → equal |
| S2 | `quality_receipt_schema_version_in_json` | `"schema_version": 1` in JSON |

---

## 16. `LaneReceipt` — lane + evidence_digest + clean

### 16.1 Unit (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `lane_receipt_constructs` | Lane + digest + clean=true → struct | `assert_eq!(lr.lane, Lane::Fmt)` etc. |
| U2 | `lane_receipt_accessors` | All 3 fields accessible | `assert_eq!` |

### 16.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `lane_receipt_serde_round_trip` | Full receipt → JSON → equal |

---

## 17. `PolicyDiagnostic` — message + file + severity

### 17.1 Unit (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `policy_diagnostic_error_constructs` | Severity::Error → `PolicyDiagnostic` | `matches!(d, PolicyDiagnostic { .. })` |
| U2 | `policy_diagnostic_warning_constructs` | Severity::Warning → `PolicyDiagnostic` | Exact |
| U3 | `policy_diagnostic_with_file` | With WorkspacePath → includes file | `assert!(d.file().is_some())` |
| U4 | `policy_diagnostic_without_file` | None file → `file()` returns `None` | Exact |
| U5 | `policy_diagnostic_message_accessor` | Returns constructed message | `assert_eq!` |

### 17.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `policy_diagnostic_serde_round_trip` | With/without file → JSON → equal |

---

## 18. `InputDiagnostic` — message + tool + severity

### 18.1 Unit (`tn_03d_unit.rs`)

| # | Test name | What it proves | Assertion style |
|---|-----------|----------------|-----------------|
| U1 | `input_diagnostic_error_constructs` | Severity::Error → `InputDiagnostic` | `matches!` |
| U2 | `input_diagnostic_with_tool` | Tool name → `Some("cargo")` | `assert_eq!(d.tool(), Some("cargo"))` |
| U3 | `input_diagnostic_without_tool` | None → `tool()` returns `None` | Exact |

### 18.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | What it proves |
|---|-----------|----------------|
| S1 | `input_diagnostic_serde_round_trip` | With/without tool → JSON → equal |

---

## 19. `DiagnosticSeverity` — Error / Warning

### 19.1 Unit (`tn_03d_unit.rs`)

| # | Test name | Assertion |
|---|-----------|-----------|
| U1 | `diagnostic_severity_both_variants` | `Error` and `Warning` constructible |

### 19.2 Serde round-trip (`tn_03d_serde_roundtrip.rs`)

| # | Test name | Assertion |
|---|-----------|-----------|
| S1 | `diagnostic_severity_serde_round_trip` | Both serialize to `error`/`warning` |

---

## Acceptance Criteria Mapping (bead §4)

### A1. Happy path: `Lane::AstGrep` serde round-trip

| Test | File | Line ref |
|------|------|----------|
| `lane_serde_all_10_variants_round_trip` | `tn_03d_serde_roundtrip.rs` S1 | Includes `Lane::AstGrep` |

### A2. Happy path: `GateScope::Release` serde round-trip

| Test | File | Line ref |
|------|------|----------|
| `gate_scope_serde_all_3_variants_round_trip` | `tn_03d_serde_roundtrip.rs` S1 | Includes `GateScope::Release` |

### A3. Happy path: `Report::Reject` with one Finding returns `RejectKind::CodeOnly`

| Test | File | Line ref |
|------|------|----------|
| `report_reject_with_only_code_findings` | `tn_03d_unit.rs` U4 | Constructs Reject |
| `reject_kind_code_only` | `tn_03d_unit.rs` U9 | Exact `assert_eq!(r.reject_kind(), Some(RejectKind::CodeOnly))` |

### A4. Error path: `Report::Reject` with empty `code_findings` and empty `gate_failures` rejected

| Test | File | Line ref |
|------|------|----------|
| `report_reject_rejects_both_empty` | `tn_03d_unit.rs` U6 | `matches!(result, Err(ReportError::BothEmpty))` |
| `report_reject_never_both_empty` | `tn_03d_properties.rs` P1 | Property: `∀ c, g: reject(c, g).is_ok() → (!c.is_empty() ∨ !g.is_empty())` |

### A5. Error path: unknown lane name returns typed error

| Test | File | Line ref |
|------|------|----------|
| `lane_from_str_exact_pascal_case` | `tn_03d_unit.rs` U2 | `"compile" -> Err` |
| `lane_deserialize_rejects_unknown_variant` | `tn_03d_deser_rejection.rs` R1 | `"NonExistent" -> Err` |
| `lane_from_str_empty_string_rejected` | `tn_03d_unit.rs` U3 | `"" -> Err` |

### A6. Edge case: `GateScope::lanes` returns 7 edit lanes in stable order

| Test | File | Line ref |
|------|------|----------|
| `gate_scope_lanes_edit_returns_7_in_order` | `tn_03d_unit.rs` U6 | Exact length + ordering |
| `gate_scope_lanes_stable_across_calls` | `tn_03d_unit.rs` U9 | Deterministic ordering across invocations |

### A7. Contract: Reject cannot contain two empty collections

| Test | File | Line ref |
|------|------|----------|
| `report_reject_rejects_both_empty` | `tn_03d_unit.rs` U6 | Constructor validation |
| `report_reject_never_both_empty` | `tn_03d_properties.rs` P1 | Property invariant |

### A8. Contract: Finding owns Lane/RuleId/Location/RepairHint/FindingEffect

| Test | File | Line ref |
|------|------|----------|
| `finding_constructs_with_all_fields` | `tn_03d_unit.rs` U1 | All 6 fields |
| `finding_clone_preserves_all_fields` | `tn_03d_unit.rs` U8 | Ownership verified via clone |
| `finding_lane_accessor` | `tn_03d_unit.rs` U2 | Exact value assertion |
| `finding_rule_id_accessor` | `tn_03d_unit.rs` U3 | Exact value assertion |
| `finding_location_accessor` | `tn_03d_unit.rs` U4 | Exact variant assertion |
| `finding_repair_accessor` | `tn_03d_unit.rs` U6 | Exact variant assertion |
| `finding_effect_accessor` | `tn_03d_unit.rs` U7 | Exact variant assertion |

---

## reject_kind() Classification Logic — Dedicated Test Matrix

| code_findings | gate_failures | Expected `reject_kind()` | Test |
|---------------|---------------|-------------------------|------|
| non-empty | empty | `Some(RejectKind::CodeOnly)` | U9 |
| empty | non-empty | `Some(RejectKind::GateOnly)` | U10 |
| non-empty | non-empty | `Some(RejectKind::Mixed)` | U11 |
| (any) | (any) | `None` on Pass | U12 |
| (any) | (any) | `None` on PolicyError | U13 |
| (any) | (any) | `None` on InputError | U14 |
| empty | empty | `Err(ReportError::BothEmpty)` | U6 |

---

## Proptest Properties Summary

| Property | File | Type | Invariant |
|----------|------|------|-----------|
| P1 (Lane) | `tn_03d_properties.rs` | `Lane` | `from_str(&to_string(l)) == Ok(l)` for all `l` |
| P2 (Lane) | `tn_03d_properties.rs` | `Lane` | `serde_round_trip(l) == l` for all `l` |
| P3 (Lane) | `tn_03d_properties.rs` | `Lane` | All 10 serialized strings are unique |
| P4 (Report) | `tn_03d_properties.rs` | `Report` | `reject(c, g).is_ok() → (!c.is_empty() ∨ !g.is_empty())` |
| P5 (Report) | `tn_03d_properties.rs` | `Report` | Non-empty collections → `reject_kind() ∈ {CodeOnly, GateOnly, Mixed}` |
| P6 (Finding) | `tn_03d_properties.rs` | `Finding` | `serde_round_trip(f) == f` for all valid `f` |
| P7 (Finding) | `tn_03d_properties.rs` | `Finding` | `f.lane == lane` for any constructed finding |
| P8 (Location) | `tn_03d_properties.rs` | `Location` | `span.line_start() >= 1` |
| P9 (Location) | `tn_03d_properties.rs` | `Location` | `span.col_start() >= 0 && col_end() >= 0` |
| P10 (Location) | `tn_03d_properties.rs` | `Location` | `serde_round_trip(loc) == loc` for all variants |
| P11 (Location) | `tn_03d_properties.rs` | `Location` | No two variants produce identical JSON |

---

## Test Count Summary

| Test File | Unit | Integration | Property | Total |
|-----------|------|-------------|----------|-------|
| `tn_03d_serde_roundtrip.rs` | — | 48 | — | 48 |
| `tn_03d_unit.rs` | 85 | — | — | 85 |
| `tn_03d_properties.rs` | — | — | 11 | 11 |
| `tn_03d_deser_rejection.rs` | — | 15 | — | 15 |
| `tn_03d_acceptance.rs` | — | 8 | — | 8 |
| `tn_03d_e2e.rs` | — | 3 | — | 3 |
| **Total** | **85** | **74** | **11** | **170** |

**Trophy alignment:**
- Unit (unit_tests + properties): 85 + 11 = 96 → ~56%
- Integration (serde + rejection + acceptance): 74 → ~43%
- E2E: 3 → ~2%
- Static (compile-test for `#[non_exhaustive]`): ~1 test → ~1%

*Note: Proptest counts as unit (they test invariants of domain types). The integration weight comes from serde round-trips, rejection tests, and acceptance tests which exercise the full JSON wire format.*

---

## No `is_ok()`-Only Assertions Guarantee

Every test uses **exact value assertions**:

| Violation pattern | Replacement |
|-------------------|-------------|
| `assert!(result.is_ok())` | `matches!(result, Ok(ExactVariant))` + field-level `assert_eq!` |
| `assert!(result.is_err())` | `matches!(result, Err(ExpectedErrorVariant))` |
| `assert!(variant.matches!(Some(..)))` | `assert_eq!(result, Some(ExpectedValue))` |
| `assert!(vec.len() > 0)` | `assert_eq!(vec.len(), ExactCount)` |

**Exception**: Property tests may use `prop_assert!` when the invariant is a predicate (e.g., `all(hex_chars)`), but unit and integration tests always assert exact values.

---

## Testing Trophy Breakdown by Type

| Type | Unit | Integration | Property | E2E |
|------|------|-------------|----------|-----|
| `Lane` | 7 | 3 + 3 | 3 | — |
| `GateScope` | 10 | 2 + 1 | — | — |
| `Report` | 14 | 4 + 1 | 2 | — |
| `RejectKind` | 1 | 1 | — | — |
| `Finding` | 8 | 3 + 1 | 2 | — |
| `FindingEffect` | 1 | 1 | 1 | — |
| `Location` | 11 | 6 + 1 | 4 | — |
| `RepairHint` | 10 | 2 | 1 | — |
| `LaneOutcome` | 14 | 5 | — | — |
| `SkipReason` | 1 | 1 | — | — |
| `LaneEvidence` | 2 | 1 | — | — |
| `CommandEvidence` | 5 | 1 | — | — |
| `LaneFailure` | 5 | 1 | — | — |
| `ProcessTermination` | 12 | 2 | — | — |
| `QualityReceipt` | 7 | 2 | — | — |
| `LaneReceipt` | 2 | 1 | — | — |
| `PolicyDiagnostic` | 5 | 1 | — | — |
| `InputDiagnostic` | 3 | 1 | — | — |
| `DiagnosticSeverity` | 1 | 1 | — | — |
| **Totals** | **104** | **38** | **14** | **3** |

*Note: Counts include all listed tests (U1..U14, S1..S6, P1..P11, acceptance, e2e). Some unit tests are shared across types via composition.*

---

## Dependencies on Existing Types

These tests depend on already-existing types from `titania-core`:

| Existing Type | Used In | File |
|--------------|---------|------|
| `Digest` | `QualityReceipt`, `LaneReceipt`, `LaneEvidence` | `digest.rs` |
| `RuleId` | `Finding::rule_id` | `rule_id.rs` |
| `WorkspacePath` | `Location::Span::file`, `Location::Manifest::file`, `PolicyDiagnostic::file` | `workspace_path.rs` |
| `TextRange` | `RepairHint::Patch::range` | `text_range.rs` |
| `CoreError` | Error type wrapping all new error types | `error.rs` (new variants) |

The new tests will import new types from `titania_core` alongside these existing ones.

---

## Test Configuration

All test files use the crate-level exemptions:

```rust
#![allow(clippy::needless_borrow)]
#![allow(clippy::useless_vec)]
#![allow(clippy::as_conversions)] // for ASCII in test strings
#![allow(clippy::arithmetic_side_effects)] // for proptest bounds
#![allow(clippy::type_complexity)] // for proptest strategies
#![allow(clippy::map_identity)]
```

These mirror the exemptions already present in `json_roundtrip.rs`, `unit_tests.rs`, and `properties.rs`.

---

## E2E Test Design (`tn_03d_e2e.rs`)

Three minimal end-to-end scenarios:

| # | Scenario | What it exercises |
|---|----------|-------------------|
| E1 | `e2e_build_report_reject_round_trip` | Construct a `Report::Reject` with 2 findings (Fmt, Clippy) + 1 gate failure, serialize to JSON, deserialize, verify `reject_kind()` → `Mixed`, verify all lanes in `per_lane` |
| E2 | `e2e_build_report_pass_round_trip` | Construct `Report::Pass` with `QualityReceipt` (3 lanes), serialize, deserialize, verify `scope` matches, verify `lanes` count matches |
| E3 | `e2e_lane_failure_propagates_through_gate` | Construct `Report::Reject` with `LaneFailure::ToolFailure` containing `ProcessTermination::TimedOut`, serialize, deserialize, verify the failure details are preserved |

These e2e tests exercise the full pipeline: construction → serialization → deserialization → classification, proving the domain model works end-to-end as a data format.

---

## Compile-Test for `#[non_exhaustive]` on GateScope

A dedicated compile-test ensures the `#[non_exhaustive]` attribute on `GateScope` actually prevents exhaustive matching:

| # | Test name | File | What it proves |
|---|-----------|------|----------------|
| C1 | `gate_scope_non_exhaustive_requires_default_arm` | `tests/compiletests/` | Code that matches all 3 variants WITHOUT a `_` arm fails to compile |
| C2 | `gate_scope_with_default_arm_compiles` | `tests/compiletests/` | Code with `GateScope::Edit | GateScope::Prepush | GateScope::Release | _ => ...` compiles |

This is the ~5% static portion of the Testing Trophy.
