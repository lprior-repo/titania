# tn-pdn — Type Contracts

## RuleId — Smart Constructor

```rust
pub struct RuleId(String);
```

**Constructor**: `RuleId::new(s: &str) -> Result<Self, RuleIdError>`

**Invariants** (enforced by constructor, never violated after construction):
1. Non-empty: `s.chars().any(|c| !c.is_whitespace())`
2. Contains at least one underscore: `s.contains('_')`
3. All characters in `[A-Z0-9_]`

**Violations** (returned, never panicked):
- `RuleIdError::Empty` — zero-length string
- `RuleIdError::NoUnderscore` — underscore required
- `RuleIdError::NotUppercase` — lowercase or non-ASCII found

**Construction path**: `RuleId::new(&format!("CLIPPY_{}", lint.to_ascii_uppercase()))` — clippy normalizer always produces valid RuleIds.

**Serialization**: `Serialize` → `"CLIPPY_UNWRAP_USED"`, `Deserialize` → passes through smart constructor.

## Lane — Fixed Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Lane { Fmt, Compile, Clippy, AstGrep, Dylint, PanicScan, PolicyScan, Test, Deny, Build }
```

**Invariant**: Closed set — `Lane::from_str` returns `LaneError::UnknownLane` for unrecognized strings. No runtime extension.

**Serde**: `Lane::Fmt` ↔ `"Fmt"`. Always PascalCase in JSON wire format.

## GateScope — Composite Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum GateScope { Edit, Prepush, Release }
```

**Invariant**: `GateScope::lanes()` returns a fixed, ordered slice per variant. Edit = 7 lanes, Prepush = 9, Release = 10.

**Forward compatibility**: `#[non_exhaustive]` prevents downstream exhaustive match from breaking when new scopes are added.

## Report — Total Enum

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum Report {
    Pass { receipt: QualityReceipt, per_lane: Box<[LaneOutcome]> },
    Reject { code_findings: Box<[Finding]>, gate_failures: Box<[LaneFailure]>, per_lane: Box<[LaneOutcome]> },
    PolicyError { diagnostics: Box<[PolicyDiagnostic]> },
    InputError { diagnostics: Box<[InputDiagnostic]> },
}
```

**Invariant (Reject)**: `!code_findings.is_empty() || !gate_failures.is_empty()`. Checked in `Report::reject()` via `check_reject_not_empty`. Empty reject returns `ReportError::EmptyReject`.

**Invariant (Pass)**: `per_lane` must be non-empty — `check_per_lane_not_empty` returns `ReportError::EmptyPerLane` for empty pass.

**Wire format**: `#[serde(tag = "variant")]` — single `variant` field discriminates. `ReportWire` deserialization smart-constructs domain types.

## RejectKind — Derived Classifier

```rust
pub enum RejectKind { CodeOnly, GateOnly, Mixed }
```

**Derivation**: `reject_kind_for(code_findings, gate_failures)` — computed from collection emptiness. Never constructed directly by callers.

## Finding — Opaque Struct

```rust
pub struct Finding {
    lane: Lane,
    rule_id: RuleId,
    location: Location,
    message: String,
    repair: RepairHint,
    effect: FindingEffect,
}
```

**Construction**: `Finding::reject(lane, rule_id, location, message, repair)` and `Finding::informational(...)`. Smart constructors enforce all invariants.

**Immutability**: All fields private. Accessors provided by `impl`. No direct field mutation.

## FindingEffect — Binary Discriminator

```rust
pub enum FindingEffect { Reject, Informational }
```

**Semantic invariant**: `Reject` findings contribute to `Report::Reject.code_findings`. `Informational` findings do not cause rejection.

## RepairHint — Enum with Validation on Deserialize

```rust
pub enum RepairHint {
    Patch { range: TextRange, replacement: String },
    UseIteratorPipeline { suggestion: String },
    FlattenNesting { suggestion: String },
    UseCheckedArithmetic { operation: String },
    RemoveAllowAttribute { attr: String },
    ReplaceDependency { from: String, to: String },
    RequiresHumanReview { suggestion: String },
}
```

**Invariant**: `Patch.range.width() > 0` — validated on construction and on deserialization via `repair_hint_from_wire` / `TryFrom<RepairHintReadWire>`.

**Wire deserialization**: Uses `RepairHintReadWire` intermediate, then `TryFrom` smart constructor. Invalid patch ranges return `RepairHintError::EmptyRange`.

## LaneOutcome — Total Enum

```rust
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum LaneOutcome {
    Clean { evidence: LaneEvidence },
    Findings { findings: Box<[Finding]> },
    Failed { tool_failure: LaneFailure },
    Skipped { reason: SkipReason },
}
```

**Invariant**: Total — exactly one variant. No gaps, no overlaps.

## QualityReceiptV1 — Schema-Validated Receipt

```rust
pub struct QualityReceiptV1 {
    pub schema_version: u16,
    pub scope: GateScope,
    pub source_digest: Digest,
    pub cargo_lock_digest: Digest,
    pub policy_digest: Digest,
    pub toolchain_digest: Digest,
    pub lanes: Box<[LaneReceipt]>,
}
```

**Invariant**: `schema_version == 1` — validated on deserialization. `RECEIPT_SCHEMA_VERSION = 1`. Wire deserialization rejects `schema_version != 1` with a serde error.

**Constructor**: `QualityReceiptV1::new(scope, digests, lanes)` always sets `schema_version = 1`. Callers cannot override.

## LaneEvidence — Command Evidence

```rust
pub struct LaneEvidence {
    pub command: CommandEvidence,
    pub tool_version: String,
    pub exit_status: ProcessTermination,
    pub parsed_result_digest: Digest,
}
```

**Invariant**: `CommandEvidence` argv[0] must match the expected executable (checked by `argv0_mismatch`).

## SkipReason — Why a Lane Was Skipped

```rust
pub enum SkipReason {
    PriorCompilationFailure,
    NotSelectedByScope,
    NotApplicable,
    PolicyDisabled,
}
```

**Invariant**: Closed set — all values covered in `SkipReason` enum.
