# v1.5 Error Taxonomy

> Errors raised in the v1.5 milestone, classified by the Rust error
> domain. No `Result<T, String>` is permitted; no panic path is permitted.
> Every fallible path returns a typed `Result` with thiserror-derived
> enums. Application boundary (`titania-check`) is the only place
> `anyhow::Error` may escape.

## Layer 1 — domain core (`crates/titania-core/src/`)

| Type | Carrier | Variant | Source |
|------|---------|---------|--------|
| `RuleIdError` | rule ids | `Empty`, `NoUnderscore`, `NotUppercase`, `TooLong` | (v1.0, unchanged) |
| `KaniHarnessIdError` | harness ids | `Empty`, `NoUnderscore`, `NotUppercase`, `TooLong` | (NEW) |
| `MutantIdError` | mutant ids | `Malformed`, `UnknownOperator`, `Backslash`, `NonOneBased` | (NEW) |
| `MutantsBaselineError` | baseline parse | `NotFound`, `Invalid`, `SchemaVersion{…}`, `InvalidEntry` | (NEW) |
| `RuleIdError` | rule family | unchanged — covers all new id strings | (v1.0) |
| `OutcomeError` | lane outcome construction | unchanged | (v1.0) |
| `ReportError` | report construction | unchanged | (v1.0) |
| `TargetProjectError` | target discovery | unchanged | (v1.0) |

## Layer 2 — lane shell (`crates/titania-lanes/src/`)

| Type | Variant | Source |
|------|---------|--------|
| `RunLaneError` | `LaneCommand`, `AstGrep`, `CurrentTarget`, `Outcome`, `Policy`, `RuleId`, `SourceWalk`, `Internal`, `Kani(KaniRunError)`, `Mutants(MutantsRunError)` | (NEW variants) |
| `KaniRunError` | `ToolMissing`, `CgroupFailed`, `InventoryParse`, `HarnessRunFailed`, `Infrastructure`, `WriteArtifact` | (NEW) |
| `MutantsRunError` | `ToolMissing`, `LoadBaseline`, `RunMutants`, `ParseOutcomes`, `ParseMutants`, `DiffBaseline`, `WriteArtifact` | (NEW) |

## Layer 3 — application boundary (`crates/titania-check`)

| Type | Variant | Notes |
|------|---------|-------|
| `anyhow::Error` | (escape hatch) | only at the `titania-check main.rs` boundary; preserved `source()` for typed upstream causes |
| `CliDisposition` | `input_error`, `internal_error`, `report`, `silent`, `lane_execution` | unchanged from v1.0 |

## Skip-state taxonomy (extends v1 spec §4)

`SkipReason` is the existing enum; v1.5 adds two variants:

```rust
pub enum SkipReason {
    PriorCompilationFailure,
    NotSelectedByScope,
    NotApplicable,
    PolicyDisabled,

    // NEW (v1.5)
    ToolUnavailable(ToolKind),    // typed payload, no string
    ProfileBaselineMissing,       // mutants first-run probe
}
```

`NotApplicable` continues to map to `LaneExit::NotApplicable` in the
shell; this disposition does NOT fail the gate.

`ToolUnavailable(CargoKani)` and `ToolUnavailable(CargoMutants)` are
emitted as findings (not skips) when the tool is present-but-too-old.
Too-old detection: `cargo-kani < 0.50.0` and `cargo-mutants < 25.0.0`.

## Reject kind extension

`LaneOutcome::RejectKind` (the typed sum-type tag v1 derives from the
finding collection) gains:

```rust
RejectKind {
    KaniFail,               // (NEW) Any PROOF_KANI_FAIL/BLOCKED/NOT_RUN rolled up
    MutantSurvivor,         // (NEW) Any MUTANT_SURVIVED or MUTANT_BASELINE_MISSING rolled up
    // (existing v1 variants: CodeOnly, GateOnly, Mixed, …)
}
```

These exist so the reporter can pattern-match on the lane's tag
without enumerating findings.
