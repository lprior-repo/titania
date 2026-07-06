# tn-pdn — Domain Model

## Ubiquitous Language

| Term | Meaning |
|------|---------|
| **Gate** | A composite set of analysis lanes (Edit, Prepush, Release). |
| **Lane** | A single tool or analysis pass (Fmt, Compile, Clippy, AstGrep, Dylint, PanicScan, PolicyScan, Test, Deny, Build). |
| **Scope** | A `GateScope` value — Edit, Prepush, or Release — determines which lanes run. |
| **Report** | The aggregated output of a `titania-check` run: Pass, Reject, PolicyError, or InputError. |
| **Finding** | A single violation from a lane, carrying a `RuleId`, `Location`, `RepairHint`, and `FindingEffect`. |
| **GateFailure** | An infrastructure or tool failure (not a code issue): infra failure, tool failure, suspicious failure, resource failure. |
| **CodeFinding** | A finding with `FindingEffect::Reject` that belongs in `Report::Reject.code_findings`. |
| **Receipt** | `QualityReceiptV1` — a v1-stable evidence envelope with schema_version, scope, 4 digests, and per-lane receipts. |
| **Fixture** | A minimal Cargo project used as test input (bad = rejects, repaired = passes). |

## Value Objects

### RuleId
A validated rule identifier: uppercase ASCII (`A-Z`, `0-9`), at least one underscore. Never constructed directly — always via `RuleId::new` which returns `Result<RuleId, RuleIdError>`. **Illegal state**: lowercase, no underscore, or empty string — unrepresentable after construction.

### Lane
Fixed enum of 10 analysis passes. Serializes to `PascalCase`. Construction via `Lane::from_str` or literal. No runtime extension possible without source change.

### GateScope
Composite set of lanes. `Edit` = 7 lanes (fmt, compile, clippy, ast-grep, dylint, panic-scan, policy-scan). `Prepush` = edit + test + deny. `Release` = prepush + build. `#[non_exhaustive]` for forward compatibility.

### Location
Where a finding occurred: span (file + line/col range), workspace root, dependency (crate + version), or manifest path. Span is the dominant variant for code findings.

### FindingEffect
Binary: `Reject` (must be resolved) or `Informational` (notes only, lane passes).

### RepairHint
Machine-actionable suggestion: `Patch { range, replacement }`, `UseIteratorPipeline { suggestion }`, `FlattenNesting { suggestion }`, `UseCheckedArithmetic { operation }`, `RemoveAllowAttribute { attr }`, `ReplaceDependency { from, to }`, `RequiresHumanReview { suggestion }`. Smart constructor validates invariants on deserialization.

### LaneOutcome
Exactly one of: `Clean { evidence }`, `Findings { findings }`, `Failed { LaneFailure }`, `Skipped { reason }`. Total — no gaps.

### LaneFailure
Infrastructure classification: `Infra { tool, reason }`, `ToolFailure { tool, ProcessTermination }`, `SuspiciousFailure { tool, evidence }`, `ResourceFailure { tool, limit }`.

### ProcessTermination
How a process ended: `Exited { code }`, `Signaled { signal }`, `TimedOut`, `MemoryLimitExceeded`, `SpawnFailed`.

### Report
Four variants, total and mutually exclusive:
- `Pass { receipt, per_lane }` — all lanes clean or skipped, receipt present.
- `Reject { code_findings, gate_failures, per_lane }` — at least one finding or failure.
- `PolicyError { diagnostics }` — policy configuration could not be loaded.
- `InputError { diagnostics }` — invocation validation failed.

**Invariant**: `Reject` with empty `code_findings` AND empty `gate_failures` is impossible — `check_reject_not_empty` returns `EmptyReject`.

### RejectKind
Classifier for `Reject`: `CodeOnly`, `GateOnly`, `Mixed`. Derived from which collections are non-empty.

### QualityReceiptV1
Stable evidence envelope: `schema_version: u16` (always 1), `scope: GateScope`, `source_digest: Digest`, `cargo_lock_digest: Digest`, `policy_digest: Digest`, `toolchain_digest: Digest`, `lanes: Box<[LaneReceipt]>`.

### LaneReceipt
Per-lane summary: `lane: Lane`, `evidence_digest: Digest`, `clean: bool`.

### Digest
Blake3 hash of inputs (source tree, Cargo.lock, policy config, toolchain). Immutable once computed.

## Aggregates

### titania-check CLI
Command: `titania-check --scope <Edit|Prepush|Release> --emit json [--out <path>]`. Orchestrates: parse scope → dispatch lanes → collect per-lane artifacts → aggregate → emit Report.

### Lane Dispatch
For each lane in `GateScope.lanes()`, the dispatch shell runs the lane runner, writes typed artifacts to `.titania/out/edit/<lane>.json`, and records the `LaneOutcome`.

### Report Assembly
`assemble_report` combines per-lane outcomes into a single `Report`:
- Any `Findings` with `Reject` effect → `code_findings`
- Any `Failed` → `gate_failures`
- All `Findings` with `Informational` → ignored for rejection
- All clean/skipped lanes → `per_lane` includes them
- If any code_findings or gate_failures → `Reject`; else → `Pass { receipt, per_lane }`

## Entities

- **Fixture project**: Minimal Cargo workspace (Cargo.toml + src/lib.rs). Two variants: `bad` (for-loop + unwrap), `repaired` (iterator pipeline, no unwrap). Owned by the killer demo test.
- **Policy configuration**: TOML file defining scope, lane enable/disable, exceptions. Not in delivery scope for tn-pdn.

## Policies

1. **Finding ownership**: A finding produced by AstGrep or Clippy lanes with `Reject` effect MUST appear in `code_findings`, never in `gate_failures`.
2. **Failure ownership**: A lane infrastructure failure MUST appear in `gate_failures` as `LaneFailure`, never in `code_findings`.
3. **Receipt ownership**: `Report::Pass` MUST carry a `QualityReceiptV1` with `schema_version = 1`.
4. **Scope enforcement**: `GateScope::Edit` runs exactly 7 lanes in the fixed order defined in `gate_scope.rs`.
5. **Digest immutability**: Once computed, digests are final — they are Blake3 hashes of the actual input files.
6. **Reject non-empty**: `Report::Reject` must have at least one finding or failure — an empty reject is a bug.
