# tn-pdn — Boundary Map

## Functional Core (Pure, No Side Effects)

| Module | Responsibility | Side Effects |
|--------|---------------|--------------|
| `titania-core/src/rule_id.rs` | `RuleId` validation | None |
| `titania-core/src/finding.rs` | `Finding`, `FindingEffect` types | None |
| `titania-core/src/finding/repair_hint.rs` | `RepairHint` with smart constructor | None |
| `titania-core/src/finding/location.rs` | `Location` types | None |
| `titania-core/src/report.rs` | `Report`, `RejectKind`, `assemble_report` | None |
| `titania-core/src/outcome.rs` | `LaneOutcome`, `LaneEvidence`, `SkipReason` | None |
| `titania-core/src/lane.rs` | `Lane` enum + `from_str` | None |
| `titania-core/src/gate_scope.rs` | `GateScope`, lane slices | None |
| `titania-core/src/v1_receipt.rs` | `QualityReceiptV1` with schema validation | None |
| `titania-core/src/failure.rs` | `LaneFailure`, `ProcessTermination` | None |
| `titania-core/src/digest.rs` | `Digest` (Blake3) | None (pure hash) |
| `titania-core/src/error.rs` | All `Error` types | None |
| `titania-aggregate/tests/report_assembly.rs` | In-memory report assembly tests | None |

**Core invariants enforced**: RuleId validation, Report::Reject non-empty, QualityReceiptV1 schema_version, RepairHint patch range, LaneOutcome total enum, Lane::from_str exhaustiveness.

## Imperative Shell (I/O, Subprocesses)

| Module | Responsibility | Side Effects |
|--------|---------------|--------------|
| `crates/titania-check/` | CLI parsing, argument validation | Filesystem, stdout/stderr |
| `titania-lanes/src/ast_grep_lane.rs` | Embedded ast-grep rules, file reading | Filesystem (source files) |
| `titania-lanes/src/clippy_normalizer.rs` | JSONL parsing, Clippy → CLIPPY_* mapping | None (pure data conversion) |
| `titania-lanes/src/run_cargo/` | Cargo subprocess execution | Process spawn, stdout capture |
| `titania-lanes/rules/functional.yml` | Embedded ast-grep rule YAML | None (compile-time data) |

**Shell responsibilities**: Subprocess execution, file I/O, JSON artifact writing, CLI argument parsing.

## Boundary Points (External Input → Core Validation)

### 1. CLI Arguments → InputDiagnostic
**Boundary**: CLI parser (`titania-check` args)
**Input**: `--scope`, `--emit`, `--out`, positional paths
**Validation**: `GateScope::from_str` for scope; unknown → `InputError`
**Core type**: `InputDiagnostic { message, tool, severity }`
**Test pattern**: `cli_args_unknown_scope_rejected` in `cli_dispatch.rs`

### 2. Policy File → PolicyDiagnostic
**Boundary**: Policy TOML loader
**Input**: Policy file content
**Validation**: TOML parse + semantic validation
**Core type**: `PolicyDiagnostic { message, file, severity }`
**Test pattern**: `REPORT_POLICY_ERROR_JSON` golden fixture

### 3. Lane JSON Artifacts → LaneOutcome
**Boundary**: Lane artifact deserialization (`.titania/out/edit/<lane>.json`)
**Input**: JSON from lane runners
**Validation**: `#[serde(tag = "variant", deny_unknown_fields)]` on `LaneOutcome`
**Core type**: `LaneOutcome` (total enum)
**Test pattern**: `aggregate_cli_reads_edit_lane_outputs`

### 4. Clippy JSONL → CLIPPY_* RuleIds
**Boundary**: `normalize_clippy_jsonl(input)`
**Input**: Raw Clippy JSONL diagnostics
**Validation**: `RuleId::new(&format!("CLIPPY_{}", lint.to_ascii_uppercase()))`
**Core type**: `Finding` with `RuleId`
**Mapping**: `clippy::unwrap_used` → `CLIPPY_UNWRAP_USED`, unknown lints → `CLIPPY_UNKNOWN`

### 5. ast-grep YAML Rules → RuleId + RepairHint
**Boundary**: `rules/mod.rs` embedded rule catalog
**Input**: YAML documents from `functional.yml`
**Validation**: `RuleId::new(id)`, `RepairHint` smart constructor on deserialization
**Core type**: `RuleDef { id, language, severity, pattern, metadata }`
**Mapping**: `FUNC_LOOPS_FOR` → `RepairHint::UseIteratorPipeline`

### 6. Report JSON → Report
**Boundary**: `ReportWire` deserialization → `Report`
**Input**: Aggregated JSON report
**Validation**: `deny_unknown_fields`, smart constructor checks
**Core type**: `Report` (total enum)
**Test pattern**: `json_roundtrip.rs` golden fixtures

## Storage Boundaries

- **Lane artifacts**: `.titania/out/edit/<lane>.json` — written by lane runners
- **Final report**: `--out <path>` or stdout — written by aggregate
- **No persistent state**: Each `titania-check` invocation is stateless (no databases, no caches)

## Network/Time Boundaries

- None. The system is fully offline and deterministic (given fixed source inputs).
- No network calls. No wall-clock timing (except timeout as a `ProcessTermination` variant).

## Unsafe Code

- None. `#![forbid(unsafe_code)]` by project policy.
- Blake3 digest uses safe Rust `blake3` crate.
- YAML parsing uses safe `serde_yaml`.
