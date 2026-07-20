# v1.5 Type Contracts

> Concrete newtype/typestate contracts for the v1.5 milestone. Each type
> has an explicit smart constructor, predicates that classify them, and
> `Result<T, _>` error families per the domain-rule ban on
> `Result<T, String>`. Implementations land in `crates/titania-core/src/`.

## Newtypes (Domain Core: `crates/titania-core/src/`)

### `KaniHarnessId`

- Wraps `String`.
- Validates: `^[A-Z][A-Z0-9_]*$`, ASCII upper-case, length ≤ 96, ≥ 1 char.
- Construction: `KaniHarnessId::new(s)` returns `Result<Self,
  KaniHarnessIdError>`; `.new()` rejects the same things `RuleId::new`
  rejects except predicates must include the underscore (rule ids and
  harness ids share that invariant).
- Predicates: `.as_str() -> &str`, `.is_equal(other: &Self) -> bool`.
- Serialized as the inner `String`. JSON: `"KANI_LANE_NAME_REJECTS_EMPTY_STRING"`
  (the example Kani harness becomes this id).

### `MutantId`

- Wraps `String`. Shape: `<pkg>::<rel-path>:<line>:<col>:<operator>`.
- Construction: `MutantId::new(pkg, rel_path, line, col, operator)`
  returns `Result<Self, MutantIdError>`. Validates the operator against
  a closed set (`==-replace`, `!=` inserted, `&&→||`, integer ±1,
  arithmetic flip, etc.) sourced from cargo-mutants output.
- Builder: `MutantId::from_cargo_mutants(serde_json::Value)` parses a
  mutants.json row; the `serde_json` dependency is already in `Lane`.
  No new dep here.
- Predicates: `.as_str()`, `.package()`, `.location()`,
  `.operator_matched(name: &str) -> bool`.
- Used in the `MUTANT_SURVIVED` finding location's source field.

### `MutantsBaseline`

- Typed JSON document at
  `.titania/profiles/strict-ai/mutants.baseline.json`.
- Schema (Rust-side):
  ```rust
  pub struct MutantsBaseline {
      schema_version: u32,
      computed_at: Option<Timestamp>,
      entries: Vec<MutantBaselineEntry>,
  }
  pub struct MutantBaselineEntry {
      mutation_id: MutantId,
      accepted_by_rule: String,
      reason: String,
      expires_on: Option<Date>,
  }
  ```
- Implements `Serialize`/`Deserialize` (matches v1.0's exception
  schema-style); round-tripped via serde_json.
- Methods: `MutantsBaseline::load(path: &Path)`,
  `MutantsBaseline::contains(mid: &MutantId) -> bool`,
  `MutantsBaseline::diff(&self, survivors: &[MutantId]) -> Vec<MutantId>`.

### `KaniRunOutcome` (per package)

- Typed outcome struct that the lane builds and writes as
  `.titania/out/full/kani.json`.
- Schema mirrors `LaneOutcome` so the aggregate layer treats Kani
  findings identically to other lanes (no schema drift, no
  bespoke ingest path).
- Helpers: `KaniRunOutcome::clean_summary(pkg, harnesses: &[HarnessOutcome]) -> Self`,
  `KaniRunOutcome::with_failures(...)`, `fn rejects_kind() -> Option<RejectKind>`.

## Augmentations to existing types

### `Lane` (total enum; +2 variants)

```rust
pub enum Lane {
    Fmt, Compile, Clippy, AstGrep, Dylint,
    PanicScan, PolicyScan, Test, Deny, Build,
    Kani, Mutants,                         // NEW (v1.5)
}
```

- `name()` adds `Self::Kani => "Kani"`, `Self::Mutants => "Mutants"`.
- `from_str` matches `"Kani"` and `"Mutants"`.
- Round-trip: `Lane::from_str(Lane::name()) == Ok(lane)` for every variant.
- JSON serde name: `PascalCase` — yields `"Kani"`, `"Mutants"`.
- Every `match lane` site updated across 9 production files + tests.

### `GateScope` (total enum; +1 variant)

```rust
pub enum GateScope {
    Edit, Prepush, Release,
    Full,                                  // NEW (v1.5)
}
```

- `from_str` matches `"full"`.
- `lanes()` adds `FULL_LANES: &[Lane]` = `RELEASE_LANES + [Lane::Kani,
  Lane::Mutants]` (Full inherits everything Release runs).
- `titania-check --scope full` made available (CLI path).

### `SkipReason` (existing enum; +variants)

```rust
pub enum SkipReason {
    PriorCompilationFailure,
    NotSelectedByScope,
    NotApplicable,
    PolicyDisabled,
    ToolUnavailable(#[serde(with = "ToolKind"))])   // NEW (v1.5)
    ProfileBaselineMissing,                         // NEW (v1.5)
}
```

- `ToolUnavailable(ToolKind)` carries the failing tool (`CargoKani`,
  `CargoMutants`) in the typed payload (no stringly-typed skip).
- `ProfileBaselineMissing` is the v1.5 first-run baseline probe.

### `LaneFailure` (existing; +flavour)

The lane-failure infrastructure does not need a new variant. The
findings are enough — `MUTANT_BASELINE_MISSING` and `PROOF_KANI_NOT_RUN`
are findings, not failures.

### Rule IDs (existing `RuleId` newtype; +strings)

| Rule ID | Meaning |
|---------|---------|
| `PROOF_KANI_PASS` | per-harness success |
| `PROOF_KANI_FAIL` | per-harness failure or counterexample |
| `PROOF_KANI_BLOCKED` | harness blocked (OOM/timeout) |
| `PROOF_KANI_NOT_RUN` | lane did not run (tool missing) |
| `PROOF_KANI_UNSUPPORTED` | unsupported-feature warning |
| `MUTANT_SURVIVED` | per-survivor mutation |
| `MUTANT_BASELINE_MISSING` | first-run operator error |

These are plain `RuleId` instances — no new enum variants in `RuleId`
itself. Each is added to the explain catalog with prose + repair hint.

## Errors (typed)

### `KaniHarnessIdError` (new)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum KaniHarnessIdError {
    #[error("harness id must not be empty")]
    Empty,
    #[error("harness id must contain at least one underscore")]
    NoUnderscore,
    #[error("harness id must be uppercase ASCII; bad character {0:?} at byte {1}")]
    NotUppercase(char, usize),
    #[error("harness id must not exceed 96 characters; got {0}")]
    TooLong(usize),
}
```

### `MutantIdError` (new)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MutantIdError {
    #[error("mutant id must include package, path, line, col, and operator")]
    Malformed,
    #[error("operator {0:?} is not a recognised cargo-mutants operator")]
    UnknownOperator(String),
    #[error("mutant id path component must not contain backslashes")]
    Backslash,
    #[error("mutant id line/col must be 1-based; got line={0} col={1}")]
    NonOneBased(u32, u32),
}
```

### `MutantsBaselineError` (new)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MutantsBaselineError {
    #[error("baseline file not found at {0}")]
    NotFound(PathBuf),
    #[error("baseline file at {0} contains invalid JSON: {1}")]
    Invalid(PathBuf, String),
    #[error("baseline schema_version={got}, expected={expected}")]
    SchemaVersion { got: u32, expected: u32 },
    #[error("baseline entry for mutation {0} has invalid accepted-by-rule")]
    InvalidEntry(String),
}
```

### ToolKind (new, used inside `SkipReason::ToolUnavailable`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolKind {
    CargoKani,
    CargoMutants,
}
```

## Validation guarantees (compile-time / construction-time)

- Forbidden characters in `KaniHarnessId` are rejected at construction.
- Forbidden operator tags in `MutantId::new` are rejected at construction.
- `MutantsBaseline::load` returns a typed error if the file is missing,
  has a wrong `schema_version`, fails parse, or has an entry with an
  invalid `accepted-by-rule` shape.
- Lane findings always carry one of the typed rule IDs from the table
  above; no stringly-typed rule ids.

## Functional-core / imperative-shell split

Pure core (`titania-core/src/`):
- `KaniHarnessId` construction, validation, serialization.
- `MutantId` construction, validation, serialization.
- `MutantsBaseline` `Serialize`/`Deserialize`, `contains`, `diff`.
- `KaniRunOutcome` construction; mapping `HarnessOutcome` to
  `LaneOutcome` variants.
- JSON harness inventory parsing (`kjson inventory parser).

Imperative shell (`titania-lanes/src/`):
- Spawning cargo-kani / cargo-mutants processes.
- cgroup wrapping.
- Reading `outcomes.json` / `mutants.json` from the cargo-mutants output
  tree.
- Writing `.titania/out/full/{kani,mutants}.json`.
- Writing the baseline bootstrap prompt to stderr when
  `MUTANT_BASELINE_MISSING` fires.
