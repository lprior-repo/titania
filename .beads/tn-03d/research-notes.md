# tn-03d: Core Domain Model — Research Notes

> Bead: `tn-03d` — add v1 Lane, GateScope, Report, Finding domain model to `crates/titania-core`

---

## 1. Source-of-Truth: v1-spec.md

### §4 — Lane enum, GateScope enum, scope composition

| Type | File in spec | Line |
|------|-------------|------|
| `Lane` enum (9 variants) | v1-spec.md | 133–146 |
| `GateScope` enum (3 variants, `#[non_exhaustive]`) | v1-spec.md | 175–184 |
| Scope composition (edit/prepush/release) | v1-spec.md | 166–170 |

### §10 — Domain Model (all types)

| Type | v1-spec.md line |
|------|----------------|
| `Digest` (newtype, `from_bytes` via blake3) | 531–537 |
| `RuleId` (newtype, uppercase+underscore) | 542–543 |
| `WorkspacePath` (newtype, validated) | 547–548 |
| `TextRange` (pub fields, Copy) | 551–555 |
| `Report` (enum: Pass/Reject/PolicyError/InputError) | 561–595 |
| `RejectKind` (enum) | 597 |
| `LaneOutcome` (enum) | 603–608 |
| `SkipReason` (enum) | 610–615 |
| `LaneEvidence` (struct) | 618–623 |
| `CommandEvidence` (struct) | 625–628 |
| `Finding` (struct) | 634–641 |
| `FindingEffect` (enum) | 643–646 |
| `Location` (enum) | 648–654 |
| `RepairHint` (enum) | 662–670 |
| `LaneFailure` (enum) | 676–681 |
| `ProcessTermination` (enum) | 683–689 |
| `QualityReceipt` (struct) | 698–713 |
| `LaneReceipt` (struct) | 708–712 |
| `PolicyDiagnostic` / `InputDiagnostic` (structs) | 722–732 |
| `DiagnosticSeverity` (enum) | 734–737 |

---

## 2. Existing titania-core Source Map

### Module structure (`crates/titania-core/src/lib.rs` #23–47)

```
lib.rs
├── mod digest;        → pub use Digest
├── mod discover;      → pub use discover_target
├── mod error;         → pub use CoreError, DigestError, ReceiptError, RuleIdError, TargetProjectError, TextRangeError, WorkspacePathError
├── mod kani;          → cfg(kani) only
├── mod receipt;       → pub use LaneDigest, LaneName, QualityReceipt, RECEIPT_SCHEMA_VERSION, ReceiptDigests, ReceiptLaneExit, ReceiptPeriod, RecordedTargetRoot
├── mod rule_id;       → pub use RuleId
├── mod target_project; → pub use TargetProject
├── mod text_range;    → pub use TextRange
└── mod workspace_path; → pub use WorkspacePath
```

**Lints enforced** (`lib.rs` #10–21): `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`, `indexing_slicing`, `string_slice`, `get_unwrap`, `arithmetic_side_effects`, `dbg_macro`, `as_conversions`; `forbid(unsafe_code)`.

### Existing primitive types (already done — no changes needed)

| Type | File | Key patterns |
|------|------|-------------|
| `Digest` | `digest.rs` | Smart ctor `from_hex()` → `Result<Self, DigestError>`, `from_bytes()` infallible (blake3), `as_hex()`, `Display`, `Debug`, `FromStr`, custom `Serialize`/`Deserialize` (string form, rejects via ctor) |
| `RuleId` | `rule_id.rs` | Smart ctor `new()` → `Result<Self, RuleIdError>`, `normalize()` → `Result`, `MAX_LEN=96`, `as_str()`, `prefix()`, `has_prefix()`, custom `Serialize`/`Deserialize` (string form) |
| `WorkspacePath` | `workspace_path.rs` | Smart ctor `new()` → `Result<Self, WorkspacePathError>`, `as_str()`, `segment_count()`, `starts_with_segment()`, custom `Serialize`/`Deserialize`, `TryFrom<&str>` |
| `TextRange` | `text_range.rs` | Smart ctor `new(start, end)` → `Result<Self, TextRangeError>`, private fields with getters, `Copy`, methods: `start()`, `end()`, `width()`, `is_empty()`, `contains_byte()`, `overlaps()`, custom `Serialize`/`Deserialize` |
| `QualityReceipt` | `receipt.rs` | Smart ctor `new()` → `Result<Self, ReceiptError>`, private fields with getters, schema_version=u32 (NOT u16!), `LaneDigest`, `ReceiptLaneExit`, `ReceiptPeriod`, sub-modules: `digests/`, `lane_name/`, `schema/`, `serde_support/`, `target_root/` |
| `LaneName` | `receipt/lane_name.rs` | Smart ctor `new()` → `Result<Self, ReceiptError>`, string newtype |

### Error types (`crates/titania-core/src/error.rs` #1–113)

| Error enum | Variants | Lines |
|------------|----------|-------|
| `DigestError` | `WrongLength(usize)`, `NonHexChar(usize)` | 10–15 |
| `RuleIdError` | `Empty`, `NoUnderscore`, `NotUppercase(char, usize)` | 19–26 |
| `WorkspacePathError` | `Empty`, `LeadingSlash`, `ContainsDotDot`, `ContainsBackslash`, `ContainsNull`, `ControlByte(u8)` | 30–43 |
| `TextRangeError` | `EndBeforeStart { start: u32, end: u32 }` | 47–50 |
| `TargetProjectError` | 10 variants (Empty, NonAbsolute, NotUtf8, NotFound, NotADirectory, NoCargoToml, CargoTomlNotFile, MalformedCargoToml, Io) | 55–74 |
| `ReceiptError` | 8 variants (UnsupportedSchemaVersion, EmptyLaneName, InvalidLaneName, PassedExceedsScanned, FinishedBeforeStarted, TargetRootEmpty, TargetRootNonAbsolute, TargetRootContainsNul) | 79–96 |
| `CoreError` | 6 transparent variants wrapping the above | 100–113 |

---

## 3. Pattern Analysis

### Smart constructor pattern (all newtypes)

1. Private inner field (`String`, `u32`, etc.)
2. `pub fn new(...) -> Result<Self, XxxError>` — validates input
3. `#[must_use]` on constructors
4. Public accessor methods (`as_str()`, `start()`, etc.) returning references
5. Never expose inner value directly

### Serde pattern (all newtypes + enums)

1. **Structs**: Custom `Serialize` (writes inner value) + custom `Deserialize` (parses inner, passes through smart constructor, maps errors via `serde::de::Error::custom`)
2. **Enums**: `#[derive(Serialize, Deserialize)]` + `#[serde(rename_all = "snake_case")]`
3. **Non-exhaustive enums**: `#[non_exhaustive]` on the derive
4. **Structs with private fields**: Derive `Serialize` only if all fields are serializable; for validation on deserialization, use `#[derive(Deserialize)]` on a wire struct in a private module, then convert through the smart constructor

### TextRange serde (struct with validation)

`text_range.rs` has private fields but derives `Serialize` directly. For `Deserialize`, it must reject `end < start`. The spec shows `pub start_byte: u32, pub end_byte: u32` but existing code has private fields — this is a **design divergence** the bead should follow existing convention (private fields, public getters).

### Enum pattern (ReceiptLaneExit precedent)

`receipt.rs` #25–32 — `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]` + `#[serde(rename_all = "snake_case")]`. Simple unit-variant enums get full derives.

### Non-exhaustive enum pattern (GateScope requirement)

Spec §4 requires `#[non_exhaustive]` on `GateScope`. Existing codebase has no non-exhaustive enums yet — this is a new pattern to add.

Also note: `v1-spec.md` #376 sets lint `non_exhaustive_omitted_patterns = "deny"`. This means **every** exhaustive match on a `#[non_exhaustive]` enum must have a default `_` arm — the compiler will reject code like `match scope { GateScope::Edit => ..., }` without a catch-all. This is a deliberate design decision to force forward-compatible match arms.

### Box<[T>] for owned slices

Spec uses `Box<[LaneOutcome]>`, `Box<[Finding]>`, `Box<[LaneFailure]>` throughout — this is the convention for owned homogeneous collections.

### QualityReceipt structure divergence

The spec (§10, lines 698–713) defines:
```rust
QualityReceipt { schema_version: u16, scope: GateScope, source_digest: Digest, cargo_lock_digest: Digest, policy_digest: Digest, toolchain_digest: Digest, lanes: Box<[LaneReceipt]> }
```

The existing code (`receipt.rs` #144–154) has:
```rust
QualityReceipt { schema_version: u32, target_root: RecordedTargetRoot, started_at: u64, finished_at: u64, lane_results: Vec<LaneDigest>, source_digest: Digest, lock_digest: Digest, policy_digest: Digest, toolchain_digest: Digest }
```

This is a **major divergence**: the spec's QualityReceipt does NOT have `target_root`, `started_at`, `finished_at`, and uses `u16` for version. The bead needs to decide: keep existing or follow spec. Given this is a "domain model" bead focused on Lane/GateScope/Report/Finding, the receipt structure is likely out of scope unless it directly touches these new types.

---

## 4. Gap Analysis

### What EXISTS (no new code)
- `Digest` — fully implemented, matches spec
- `RuleId` — fully implemented, matches spec
- `WorkspacePath` — fully implemented, matches spec
- `TextRange` — fully implemented, fields are private (spec says pub) — divergence but consistent with repo convention
- `CoreError` aggregate — needs new variants for new errors

### What needs to be CREATED (new files or new types)

#### New modules (one per type or type group):

| New file | Types | Rationale |
|----------|-------|-----------|
| `lane.rs` | `Lane` enum | Standalone enum, no validation needed (unit variants) |
| `gate_scope.rs` | `GateScope` enum, `scope_lanes()` | Non-exhaustive enum + helper |
| `finding.rs` | `Finding`, `FindingEffect`, `Location`, `RepairHint` | Finding is a struct + 3 supporting types |
| `failure.rs` | `LaneFailure`, `ProcessTermination` | Failure-related types |
| `outcome.rs` | `LaneOutcome`, `SkipReason`, `LaneEvidence`, `CommandEvidence` | Lane outcome types |
| `report.rs` | `Report`, `RejectKind` | Report is the aggregator type |
| `receipt_v1.rs` | `QualityReceipt` (v1), `LaneReceipt` | v1 spec QualityReceipt (may conflict with existing) |
| `diagnostic.rs` | `PolicyDiagnostic`, `InputDiagnostic`, `DiagnosticSeverity` | Diagnostic types |

#### New error types (add to `error.rs`):

| Error enum | Types it protects |
|------------|-------------------|
| `LaneError` | `Lane` — none needed (unit variants are always valid) |
| `GateScopeError` | `GateScope` — none needed (unit variants) |
| `FindingError` | `Finding` — if validation needed (probably not for struct) |
| `ReportError` | `Report` — `Reject` invariant (at least one of code_findings/gate_failures non-empty) |
| `LocationError` | `Location` — none needed (enum) |

Actually, most of the new types are enums or structs with no invariants beyond what's enforced by the variant selection. The main validation is:
- `Report::Reject` invariant: at least one of `code_findings` or `gate_failures` is non-empty → enforce in constructor or as assertion
- `GateScope::scope_lanes()` returns correct lane set (compile-time or runtime assertion)

### What the spec defines but existing code has differently

| Spec type | Existing code | Action |
|-----------|--------------|--------|
| `QualityReceipt.schema_version: u16` | `schema_version: u32` | Spec wins for v1 domain; existing receipt module stays separate |
| `QualityReceipt` fields | `target_root`, `started_at`, `finished_at` in existing | Existing receipt.rs stays as-is (different schema/version); new v1 types in separate module |
| `TextRange` pub fields | private fields | Follow existing convention (private fields + getters) |

---

## 5. Implementation Plan

### File layout (all under `crates/titania-core/src/`)

```
lib.rs                           ← add: mod lane, mod gate_scope, mod finding, mod failure, mod outcome, mod report, mod diagnostic; pub use ...
lane.rs                          ← NEW: Lane enum
gate_scope.rs                    ← NEW: GateScope enum + scope_lanes()
finding.rs                       ← NEW: Finding struct + FindingEffect enum + Location enum + RepairHint enum
failure.rs                       ← NEW: LaneFailure enum + ProcessTermination enum
outcome.rs                       ← NEW: LaneOutcome enum + SkipReason enum + LaneEvidence struct + CommandEvidence struct
report.rs                        ← NEW: Report enum + RejectKind enum
diagnostic.rs                    ← NEW: PolicyDiagnostic struct + InputDiagnostic struct + DiagnosticSeverity enum
error.rs                         ← ADD: LaneError, GateScopeError, FindingError, ReportError (if needed)
```

### Dependencies between new types

```
lane.rs           ← (none)
gate_scope.rs     ← lane.rs (scope_lanes() returns & [Lane])
finding.rs        ← lane.rs, rule_id.rs (already exists), workspace_path.rs (already exists)
failure.rs        ← (none)
outcome.rs        ← lane.rs, failure.rs, digest.rs (already exists), finding.rs
report.rs         ← finding.rs, failure.rs, outcome.rs, receipt.rs (QualityReceipt), diagnostic.rs
diagnostic.rs     ← workspace_path.rs (already exists)
```

### Implementation order (dependencies respected)

1. `lane.rs` — no deps, simplest type
2. `gate_scope.rs` — depends on lane.rs
3. `failure.rs` — no deps, simple enums
4. `finding.rs` — depends on lane.rs, rule_id.rs (existing), workspace_path.rs (existing)
5. `outcome.rs` — depends on lane.rs, failure.rs, digest.rs (existing), finding.rs
6. `diagnostic.rs` — depends on workspace_path.rs (existing)
7. `report.rs` — depends on finding.rs, failure.rs, outcome.rs, diagnostic.rs, receipt.rs (existing)
8. `error.rs` — add error variants (can be done in parallel with step 1)
9. `lib.rs` — wire up all new modules and re-exports

### Serde considerations

All new types need `Serialize`/`Deserialize` for JSON lane output and report output:

- **Enums** (`Lane`, `GateScope`, `FindingEffect`, `Location`, `RepairHint`, `LaneFailure`, `ProcessTermination`, `LaneOutcome`, `SkipReason`, `Report`, `RejectKind`, `DiagnosticSeverity`): `#[derive(Serialize, Deserialize)]` + `#[serde(rename_all = "snake_case")]`
- **Structs** (`Finding`, `LaneEvidence`, `CommandEvidence`, `PolicyDiagnostic`, `InputDiagnostic`): `#[derive(Serialize, Deserialize)]` with `#[serde(rename_all = "snake_case")]` on field names if needed
- **Non-exhaustive**: `GateScope` gets `#[non_exhaustive]` which prevents deriving `Deserialize` for exhaustive matching. With `#[non_exhaustive]` on an enum, `Deserialize` still works — it produces an unknown variant that can't be exhaustively matched in user code.

### Test strategy (mirroring existing patterns)

Add to `tests/json_roundtrip.rs`:
- Round-trip tests for each new serializable type
- Reject tests for invariant-violating inputs

Add to `tests/unit_tests.rs`:
- Unit tests for each type's constructor and accessor methods
- Invariant enforcement tests (e.g., `Report::reject` with empty findings + empty failures)

Add to `tests/properties.rs`:
- Property tests for invariant relationships (e.g., gate scope composition always includes its constituent lanes)

### Cargo.toml

No new dependencies needed. `serde` and `thiserror` are already present.

### Spec compliance checklist

| Spec type | Implemented | Notes |
|-----------|------------|-------|
| `Lane` | ✓ | 9 unit variants, Copy, Hash, Serialize, Deserialize |
| `GateScope` | ✓ | 3 unit variants, non_exhaustive, Serialize, Deserialize |
| `Digest` | ✓ | Already exists |
| `RuleId` | ✓ | Already exists |
| `WorkspacePath` | ✓ | Already exists |
| `TextRange` | ✓ | Already exists (private fields, not pub as spec says — repo convention) |
| `Report` | ✓ | Pass/Reject/PolicyError/InputError variants |
| `RejectKind` | ✓ | CodeOnly/GateOnly/Mixed |
| `LaneOutcome` | ✓ | Clean/Findings/Failed/Skipped |
| `SkipReason` | ✓ | PriorCompilationFailure/NotSelectedByScope/NotApplicable/PolicyDisabled |
| `LaneEvidence` | ✓ | struct with command/tool_version/exit_status/parsed_result_digest |
| `CommandEvidence` | ✓ | struct with executable/argv |
| `Finding` | ✓ | struct with lane/rule_id/location/message/repair/effect |
| `FindingEffect` | ✓ | Reject/Informational |
| `Location` | ✓ | Span/Dependency/Manifest/Workspace/Tool |
| `RepairHint` | ✓ | Patch/UseIteratorPipeline/FlattenNesting/UseCheckedArithmetic/RemoveAllowAttribute/ReplaceDependency/RequiresHumanReview |
| `LaneFailure` | ✓ | InfraFailure/ToolFailure/ResourceFailure/SuspiciousFailure |
| `ProcessTermination` | ✓ | Exited/Signaled/TimedOut/MemoryLimitExceeded/SpawnFailed |
| `QualityReceipt` (v1) | ⚠ | Spec QualityReceipt diverges from existing — may need separate v1 type |
| `LaneReceipt` | ✓ | struct with lane/evidence_digest/clean |
| `PolicyDiagnostic` | ✓ | struct with message/file/severity |
| `InputDiagnostic` | ✓ | struct with message/tool/severity |
| `DiagnosticSeverity` | ✓ | Error/Warning |

---

## 6. Key Design Decisions

1. **Private fields over pub fields**: Following existing repo convention (TextRange, Digest, RuleId, WorkspacePath all use private fields with getters), the bead should use private fields even when spec shows `pub`.

2. **No validation for enum-only types**: `Lane`, `GateScope`, `FindingEffect`, `Location`, `RepairHint`, `LaneFailure`, `ProcessTermination`, `SkipReason`, `RejectKind`, `DiagnosticSeverity` are all unit-variant enums — no smart constructor needed, derives handle everything.

3. **GateScope #[non_exhaustive]**: Applied per spec §4. This means external code can't exhaustively match without a default arm.

4. **Scope composition function**: `GateScope::lanes() -> &'static [Lane]` or `fn scope_lanes(scope: GateScope) -> &'static [Lane]` — returns which lanes each scope covers.

5. **Box<[T]> for collections**: Spec consistently uses `Box<[LaneOutcome]>`, `Box<[Finding]>`, `Box<[LaneFailure]>`. Follow this convention.

6. **QualityReceipt v1 divergence**: The spec defines a different QualityReceipt than what exists. The bead should create a new `QualityReceiptV1` or similar in a new module to avoid breaking the existing receipt module, or note this as a known divergence to be resolved later.
