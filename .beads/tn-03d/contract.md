# v1 Domain Model — Rust Contract

> **Bead:** tn-03d  
> **Spec:** v1-spec.md §4 (Lane DAG) + §10 (Domain Model)  
> **Target crate:** `crates/titania-core`  
> **Status:** BUILDABLE CONTRACT

---

## 1. Ubiquitous Language

Every type below carries a precise English definition. These terms appear verbatim in
the spec (§4, §10) and must not be paraphrased in code, comments, or tests.

### Lane — the single judgment dimension

A `Lane` is one of ten judgment dimensions applied to a Rust/Cargo workspace. Each lane
runs one tool against one subset of the workspace and emits findings or a pass/fail.

| Variant | Tool invocation | Scope gate | Depends on Compile? | Rejects |
|---|---|---|---|---|
| `Fmt` | `cargo fmt --check` | edit | No | formatting drift |
| `Compile` | `cargo check --workspace --frozen` | edit | — | compile errors, denied rustc lints |
| `Clippy` | `cargo clippy --workspace --lib --bins --frozen` with `-F` | edit | Yes | denied clippy lints |
| `AstGrep` | embedded ast-grep-core | edit | No | structural rule violations, bypass attributes, architecture import violations |
| `Dylint` | `cargo dylint titania` | edit | No | type-aware bypass violations |
| `PanicScan` | `rg` with parser prefilter | edit | No | production `assert!`/`assert_eq!`/`assert_ne!`/`unreachable!` |
| `PolicyScan` | native TOML + env scanner | edit | No | `[lints]` weakening, `.cargo/config.toml` overrides, env var violations |
| `Test` | `cargo test --workspace --frozen -- --test-threads=1` | prepush | Yes | failing tests |
| `Deny` | `cargo deny check` | prepush | No | advisories, licenses, bans, sources, dupes |
| `Build` | `cargo build --workspace --release --frozen` | release | Yes | release build failure |

**Invariant:** `Lane` is a unit-sized enum (all variants carry no data). Every variant
has exactly one string label matching the variant name in PascalCase for JSON
serialization.

**From-str determinism:** `Lane::from_str("Fmt")` yields `Ok(Lane::Fmt)`. Case-sensitive,
PascalCase labels only. No aliasing.

### GateScope — composite gate membership

A `GateScope` is one of three scope tiers, each a superset of the previous:

```
edit     = Fmt + Compile + Clippy + AstGrep + Dylint + PanicScan + PolicyScan
prepush  = edit + Test + Deny
release  = prepush + Build
```

`#[non_exhaustive]` on the enum means external crates cannot exhaustively match on it;
v1.5 may add `Full` and v2.5 may add `Deep` without breaking downstream match arms.

**Invariant:** v1 code only produces `GateScope::Edit`, `GateScope::Prepush`,
`GateScope::Release`. No other variant is constructible.

### Report — the top-level judgment result

A `Report` is one of four mutually exclusive outcomes of a gate run:

- **`Pass { receipt, per_lane }`** — every lane either passed cleanly or was legitimately
  skipped. Contains the `QualityReceipt` and a per-lane outcome for each lane that was
  considered.

- **`Reject { code_findings, gate_failures, per_lane }`** — at least one lane rejected
  (via `Finding`) or failed (via `LaneFailure`). The `code_findings` are findings from
  individual lanes; `gate_failures` are lane-level infra/tool failures.
  **Invariant:** `code_findings` and `gate_failures` are not both empty. If both are
  empty, the correct report is `Pass`.

- **`PolicyError { diagnostics }`** — the policy configuration itself is malformed or
  invalid. No lanes were run.

- **`InputError { diagnostics }`** — the caller provided invalid input (bad args,
  missing workspace, etc.). No lanes were run.

**Invariant:** `Reject::reject_kind()` returns `None` only when both
`code_findings` and `gate_failures` are empty — this is an invariant violation
that should be impossible if `Report` is constructed correctly.

### Finding — a single judgment violation

A `Finding` is one specific rule violation discovered by one lane, with its location,
message, suggested repair, and effect.

**Fields:**
- `lane: Lane` — which lane produced this finding
- `rule_id: RuleId` — the rule that was violated (e.g. `FUNC_LOOPS_FOR`, `CLIPPY_UNWRAP_USED`)
- `location: Location` — where in the workspace the violation was found
- `message: String` — human-readable description
- `repair: RepairHint` — suggested fix (may be `RequiresHumanReview` for non-automated fixes)
- `effect: FindingEffect` — whether this finding rejects the gate or is informational

**Invariant:** A finding's `lane` field must match the lane that produced it. A
`FindingEffect::Reject` finding in `code_findings` directly blocks the gate.

### FindingEffect — gate-blocking vs informational

A `FindingEffect` classifies a finding as either gate-blocking (`Reject`) or advisory
(`Informational`). `Informational` findings are collected but do not block the gate.

### Location — where a finding lives

A `Location` encodes one of five positions where a finding may be emitted:

- **`Span { file, line_start, col_start, line_end, col_end }`** — a byte-range span
  inside a source file. Lines are 1-based, columns are 0-based Unicode scalar values.
- **`Dependency { crate_name, version }`** — a supply-chain finding about a dependency
  (e.g. `cargo deny` advisory).
- **`Manifest { file }`** — a finding about a manifest file (e.g. `Cargo.toml`).
- **`Workspace`** — a workspace-level finding (no single file).
- **`Tool { name, version }`** — a finding about the tool itself (e.g. tool version mismatch).

**Invariant:** `Span` locations always reference a `WorkspacePath` that exists under
the judged workspace root.

### RepairHint — suggested remediation

A `RepairHint` carries the best automated or semi-automated fix a lane can suggest:

| Variant | When used |
|---|---|
| `Patch { file, range, replacement }` | The lane can compute an exact text replacement |
| `UseIteratorPipeline { suggestion }` | ast-grep found a for/while/loop that should use iterators |
| `FlattenNesting { suggestion }` | ast-grep found excessive nesting depth |
| `UseCheckedArithmetic { op }` | clippy found unchecked arithmetic (`unchecked_{add,sub,mul}`) |
| `RemoveAllowAttribute { attr }` | ast-grep or dylint found a bypass attribute |
| `ReplaceDependency { from, to }` | cargo deny found a banned/unsafe dependency |
| `RequiresHumanReview { note }` | The lane cannot automate a fix (e.g. `Result<String, E>`) |

**Invariant:** `Patch` must have `range.width() > 0` — zero-width patches are meaningless.

### LaneOutcome — result of running one lane

A `LaneOutcome` is one of four states after a lane runs:

- **`Clean { evidence }`** — the lane ran successfully and produced zero findings.
  Contains `LaneEvidence` for reproducibility.
- **`Findings(Box<[Finding]>)`** — the lane ran successfully and produced findings.
- **`Failed(LaneFailure)`** — the lane failed due to infrastructure, tool crash, or
  resource constraints. No findings were collected.
- **`Skipped(SkipReason)`** — the lane was not executed (compile failed before it,
  scope didn't include it, etc.).

**Invariant:** `Clean` and `Findings` require a successful lane execution. `Failed`
and `Skipped` mean no findings were collected.

### SkipReason — why a lane was not run

| Variant | When |
|---|---|
| `PriorCompilationFailure` | `Compile` lane failed; lanes depending on compile are skipped |
| `NotSelectedByScope` | The lane is not in the chosen `GateScope` |
| `NotApplicable` | The lane has no work to do (empty workspace, etc.) |
| `PolicyDisabled` | The lane was explicitly disabled in the policy config |

**Future:** `CacheHit { input_digest: Digest }` — lane skipped because output was cached.

### LaneEvidence — reproducible evidence of a clean lane

A `LaneEvidence` captures enough detail to reproduce and verify a lane execution:

- `command: CommandEvidence` — what was run (executable + argument vector)
- `tool_version: String` — the tool's version string (e.g. "rustfmt 1.84.0")
- `exit_status: ProcessTermination` — how the process ended
- `parsed_result_digest: Digest` — blake3 digest of the parsed lane output

**Invariant:** `parsed_result_digest` is computed over the normalized, typed lane
output (after tool output is parsed into `Finding`/`LaneOutcome` values). This allows
bit-for-bit reproducibility verification.

### CommandEvidence — what was executed

A `CommandEvidence` records the exact subprocess invocation:

- `executable: String` — the binary path or name (e.g. "cargo")
- `argv: Box<[String]>` — the full argument vector including the executable as argv[0]

### LaneFailure — why a lane could not complete

| Variant | Meaning |
|---|---|
| `InfraFailure { tool, reason }` | Infrastructure problem (missing tool, permission error, output file missing) |
| `ToolFailure { tool, termination }` | The tool crashed or returned non-zero (process termination recorded) |
| `ResourceFailure { tool, limit }` | Resource limit hit (memory, timeout, file descriptor) |
| `SuspiciousFailure { tool, evidence }` | Failure that looks like tampering or flakiness (evidence string) |

**Invariant:** `LaneFailure` always names the tool that failed. A `LaneFailure` in
`gate_failures` blocks the gate regardless of `code_findings` status.

### ProcessTermination — how a subprocess ended

| Variant | Meaning |
|---|---|
| `Exited { code }` | Normal exit with the given exit code |
| `Signaled { signal }` | Killed by signal (Unix only; 9 = SIGKILL/OOM kill) |
| `TimedOut` | Killed by timeout |
| `MemoryLimitExceeded` | Killed by memory limit |
| `SpawnFailed` | Failed to start the process |

**Windows note:** `TerminateProcess` appears as `Exited { code: 1 }`. There is no
signal concept on Windows.

### RejectKind — classification of a Reject report

| Variant | Condition |
|---|---|
| `CodeOnly` | `code_findings` non-empty, `gate_failures` empty |
| `GateOnly` | `code_findings` empty, `gate_failures` non-empty |
| `Mixed` | Both `code_findings` and `gate_failures` non-empty |

**Invariant:** `RejectKind` is only meaningful on `Report::Reject`. Calling
`reject_kind()` on `Pass`, `PolicyError`, or `InputError` returns `None`.

### QualityReceipt — stable evidence envelope for a gate run

A `QualityReceipt` is the authoritative record of what was judged and what digests
were observed:

- `schema_version: u16` — always `1` for v1. Increments on breaking JSON schema changes.
- `scope: GateScope` — which scope was run
- `source_digest: Digest` — blake3 digest of the source tree
- `cargo_lock_digest: Digest` — blake3 digest of `Cargo.lock`
- `policy_digest: Digest` — blake3 digest of the policy config
- `toolchain_digest: Digest` — blake3 digest of the toolchain (rustc + cargo versions)
- `lanes: Box<[LaneReceipt]>` — per-lane receipt summaries

**schema_version policy:** additive changes (new optional fields) do NOT increment the
version. Removing fields or changing types DOES increment it.

### LaneReceipt — per-lane summary inside a QualityReceipt

A `LaneReceipt` is a compact summary of one lane's participation in a receipt:

- `lane: Lane` — which lane
- `evidence_digest: Digest` — blake3 digest of the lane's `LaneEvidence`
- `clean: bool` — whether the lane produced zero findings

### PolicyDiagnostic — policy configuration problem

A `PolicyDiagnostic` describes a problem found while loading or validating the policy:

- `message: String` — human-readable description
- `file: Option<WorkspacePath>` — file where the problem was found (if applicable)
- `severity: DiagnosticSeverity` — `Error` or `Warning`

### InputDiagnostic — caller input problem

An `InputDiagnostic` describes a problem with the caller's input (CLI args, environment,
workspace state):

- `message: String` — human-readable description
- `tool: Option<String>` — tool or component affected (if applicable)
- `severity: DiagnosticSeverity` — `Error` or `Warning`

### DiagnosticSeverity — urgency of a diagnostic

- `Error` — must be fixed; will block the gate or prevent execution
- `Warning` — should be fixed but does not block

---

## 2. Type Invariants and Constraints

| Type | Invariant | Enforcement |
|---|---|---|
| `Lane` | Exactly 10 variants, no data payloads | Unit enum; `FromStr` is PascalCase string match |
| `GateScope` | Exactly 3 variants + `#[non_exhaustive]` | Unit enum; constructors only produce v1 variants |
| `Report::Pass` | `per_lane` length ≥ 1 (at least one lane ran or was skipped) | Constructor validates |
| `Report::Reject` | `!code_findings.is_empty() || !gate_failures.is_empty()` | Constructor validates; `reject_kind()` returns `None` on violation |
| `Finding::effect` | Must be `Reject` or `Informational` only | Unit enum |
| `Finding::location` | `Span` locations reference a valid `WorkspacePath` | Constructor validates |
| `RepairHint::Patch` | `range.width() > 0` | Constructor validates |
| `LaneOutcome::Clean` | `evidence` must have `exit_status == Exited { code: 0 }` | Constructor validates |
| `LaneEvidence::parsed_result_digest` | Must be a valid 64-char lowercase hex digest | `Digest::from_hex` validates |
| `CommandEvidence::argv` | Must be non-empty (argv[0] = executable) | Constructor validates |
| `ProcessTermination::Exited` | `code` is any `i32` | No range check — tools use their own convention |
| `ProcessTermination::Signaled` | `signal` is a Unix signal number (1–31) | Constructor validates on Unix; rejected on Windows |
| `QualityReceipt::schema_version` | Must equal `RECEIPT_SCHEMA_VERSION` (currently 1) | `QualityReceipt` constructor validates (see existing `receipt.rs`) |
| `QualityReceipt::lanes` | Every `LaneReceipt` lane must appear in `per_lane` of its parent `Report` | Cross-type invariant; enforced by aggregator |
| `DiagnosticSeverity` | `Error` and `Warning` only | Unit enum |

---

## 3. Smart Constructor Signatures

```rust
// Lane — FromStr only; no smart constructor needed (unit enum)
impl FromStr for Lane { type Err = LaneError; }

// GateScope — FromStr only; v1 constructors produce Edit/Prepush/Release
impl FromStr for GateScope { type Err = GateScopeError; }

// Report
impl Report {
    pub fn pass(receipt: QualityReceipt, per_lane: Box<[LaneOutcome]>) -> Self
        // Invariant: per_lane.len() >= 1
    pub fn reject(
        code_findings: Box<[Finding]>,
        gate_failures: Box<[LaneFailure]>,
        per_lane: Box<[LaneOutcome]>,
    ) -> Result<Self, ReportError>
        // Error if both collections are empty
    pub fn policy_error(diagnostics: Box<[PolicyDiagnostic]>) -> Self
    pub fn input_error(diagnostics: Box<[InputDiagnostic]>) -> Self
    pub fn reject_kind(&self) -> Option<RejectKind>
        // Returns None on invariant violation (both empty)
}

// Finding
impl Finding {
    pub fn new(
        lane: Lane,
        rule_id: RuleId,
        location: Location,
        message: String,
        repair: RepairHint,
        effect: FindingEffect,
    ) -> Self {
        // No validation needed — all field types enforce their own invariants
    }
}

// Location
impl Location {
    pub fn span(
        file: WorkspacePath,
        line_start: u32,
        col_start: u32,
        line_end: u32,
        col_end: u32,
    ) -> Self
        // Validates line_start >= 1, col values >= 0
    pub fn dependency(crate_name: String, version: String) -> Self
    pub fn manifest(file: WorkspacePath) -> Self
    pub fn workspace() -> Self
    pub fn tool(name: String, version: String) -> Self
}

// RepairHint
impl RepairHint {
    pub fn patch(file: String, range: TextRange, replacement: String) -> Result<Self, RepairHintError>
        // Error if range.width() == 0
    pub fn use_iterator_pipeline(suggestion: String) -> Self
    pub fn flatten_nesting(suggestion: String) -> Self
    pub fn use_checked_arithmetic(op: String) -> Self
    pub fn remove_allow_attribute(attr: String) -> Self
    pub fn replace_dependency(from: String, to: String) -> Self
    pub fn requires_human_review(note: String) -> Self
}

// LaneOutcome
impl LaneOutcome {
    pub fn clean(evidence: LaneEvidence) -> Result<Self, LaneOutcomeError>
        // Validates exit_status == Exited { code: 0 }
    pub fn findings(findings: Box<[Finding]>) -> Self
    pub fn failed(failure: LaneFailure) -> Self
    pub fn skipped(reason: SkipReason) -> Self
}

// LaneEvidence
impl LaneEvidence {
    pub fn new(
        command: CommandEvidence,
        tool_version: String,
        exit_status: ProcessTermination,
        parsed_result_digest: Digest,
    ) -> Self {
        // No cross-field validation; individual types validate themselves
    }
}

// CommandEvidence
impl CommandEvidence {
    pub fn new(executable: String, argv: Box<[String]>) -> Result<Self, CommandEvidenceError>
        // Error if argv is empty or argv[0] != executable
}

// LaneFailure
impl LaneFailure {
    pub fn infra_failure(tool: String, reason: String) -> Self
    pub fn tool_failure(tool: String, termination: ProcessTermination) -> Self
    pub fn resource_failure(tool: String, limit: String) -> Self
    pub fn suspicious_failure(tool: String, evidence: String) -> Self
}

// ProcessTermination
impl ProcessTermination {
    pub fn exited(code: i32) -> Self
    pub fn signaled(signal: i32) -> Result<Self, ProcessTerminationError>
        // On Windows: rejects any signal value
    pub fn timed_out() -> Self
    pub fn memory_limit_exceeded() -> Self
    pub fn spawn_failed() -> Self
}

// RejectKind — FromStr only; produced by Report::reject_kind()
```

---

## 4. Workflow: Type Composition

```
                    ┌─────────────────────────────────────────────────┐
                    │                    Report                        │
                    │  ┌───────────────────────────────────────────┐  │
                    │  │ Pass { receipt: QualityReceipt,          │  │
                    │  │             per_lane: Box<[LaneOutcome]> } │  │
                    │  └───────────────────────────────────────────┘  │
                    │  ┌───────────────────────────────────────────┐  │
                    │  │ Reject {                                   │  │
                    │  │   code_findings: Box<[Finding]>,          │  │
                    │  │   gate_failures: Box<[LaneFailure]>,      │  │
                    │  │   per_lane: Box<[LaneOutcome]>            │  │
                    │  │ }                                         │  │
                    │  └───────────────────────────────────────────┘  │
                    │  ┌───────────────────────────────────────────┐  │
                    │  │ PolicyError { diagnostics: [...] }        │  │
                    │  │ InputError { diagnostics: [...] }         │  │
                    │  └───────────────────────────────────────────┘  │
                    └─────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┼───────────────────────────────┐
                    │               │                               │
          ┌─────────▼──────────┐   │                    ┌──────────▼──────────┐
          │  QualityReceipt    │   │                    │  LaneOutcome        │
          │  schema_version    │   │          ┌─────────▼─────────┐          │
          │  scope: GateScope  │◄──┼──────────│  Clean { evidence }│         │
          │  digests[4]        │   │          │  Findings([...])  │         │
          │  lanes: [LaneReceipt]│  │          │  Failed(LaneF)    │         │
          └────────────────────┘   │          │  Skipped(SkipR)   │         │
                                    │          └───────────────────┘          │
                                    │                    │                    │
                          ┌─────────▼──────────┐        │           ┌────────▼─────────┐
                          │  LaneReceipt        │        │           │  LaneEvidence     │
                          │  lane: Lane         │        │           │  command: CmdEvd  │
                          │  evidence_digest    │        │           │  tool_version     │
                          │  clean: bool        │        │           │  exit_status      │
                          └────────────────────┘        │           │  parsed_result    │
                                                        │           └───────────────────┘
                                                        │                     │
                                                        │           ┌─────────▼─────────┐
                                                        │           │  ProcessTerm        │
                                                        │           │  Exited/Signaled/  │
                                                        │           │  TimedOut/Mem/     │
                                                        │           │  SpawnFailed       │
                                                        │           └───────────────────┘
                                                        │
                          ┌─────────────────────────────▼──────────────┐
                          │  Finding                                   │
                          │  lane: Lane                                │
                          │  rule_id: RuleId                           │
                          │  location: Location ──► Span/Dep/Manifest  │
                          │  message: String                           │
                          │  repair: RepairHint ──► Patch/.../Human    │
                          │  effect: FindingEffect ──► Reject/Info     │
                          └───────────────────────────────────────────┘
```

**Execution flow:**
1. CLI dispatcher selects `GateScope` → determines which `Lane`s to run.
2. Each lane runner executes its tool, collects raw output.
3. Tool output → `Finding` values (normalized via finding normalization table in §5).
4. Each lane produces one `LaneOutcome`.
5. Per-lane JSON written to `.titania/out/<scope>/<lane>.json`.
6. Aggregator reads all lane output files → builds `per_lane: Box<[LaneOutcome]>`.
7. If all lanes are `Clean` or `Skipped` → `Report::Pass { receipt, per_lane }`.
8. If any lane has `Findings` or `Failed` → `Report::Reject { code_findings, gate_failures, per_lane }`.
9. If policy or input validation fails → `Report::PolicyError` or `Report::InputError`.
10. `QualityReceipt` is built from digests + `LaneReceipt` summaries + `GateScope`.

**Lane dependency rules (§4):**
- `Compile` has no dependencies → always runs first (or in parallel).
- `Clippy`, `Test`, `Build` depend on `Compile` → if `Compile` fails with `LaneFailure`,
  these lanes are `Skipped(PriorCompilationFailure)`.
- `AstGrep`, `Dylint`, `PanicScan`, `PolicyScan` run on source/config files regardless
  of compilation status → they are NOT skipped when `Compile` fails.
- `Deny` has no compile dependency → always runs.

---

## 5. Hazard Analysis

### H1: Malformed JSON during deserialization
**Risk:** Lane output JSON is written atomically (temp file + rename, §11.2). However,
concurrent reads before rename are prevented by atomic writes. Deserialization failures
are caught by `serde` and produce `PolicyError` or `InputError`.

**Mitigation:** Lane output is serialized from typed domain values, not raw strings.
Deserialization uses the same smart constructors as construction.

### H2: Invalid Lane names in JSON
**Risk:** If JSON contains a lane name not in the 10 defined variants, deserialization
must fail rather than silently ignoring or defaulting.

**Mitigation:** `Lane` uses `FromStr` with exact PascalCase match. Unknown names return
`LaneError`. JSON deserialization uses `FromStr`.

### H3: Empty Reject collections
**Risk:** `Report::Reject` with both `code_findings` and `gate_failures` empty would be
indistinguishable from `Pass`. This is an invariant violation.

**Mitigation:** `Report::reject()` constructor checks `!code_findings.is_empty() ||
!gate_failures.is_empty()`. `Report::reject_kind()` returns `None` on this violation.

### H4: Missing lane output file
**Risk:** If a lane's output file is missing, the aggregator must NOT skip the lane
silently.

**Mitigation (§11.2):** Missing file → `LaneOutcome::Failed(LaneFailure::InfraFailure {
tool, reason: "output file missing" })`. This is a gate failure.

### H5: Signal values on Windows
**Risk:** Unix `Signaled` values don't translate to Windows. `TerminateProcess` appears
as `Exited { code: 1 }`.

**Mitigation:** `ProcessTermination::Signaled` constructor rejects on Windows. Windows
tool failures are recorded as `Exited { code }` or `SpawnFailed`.

### H6: Schema version drift
**Risk:** A consumer reads a receipt with `schema_version != 1` and interprets fields
incorrectly.

**Mitigation:** `QualityReceipt` constructor validates `schema_version == RECEIPT_SCHEMA_VERSION`.
Version is a `u16`; additive changes don't increment, structural changes do.

### H7: RepairHint::Patch with zero-width range
**Risk:** A `Patch` repair with `range.width() == 0` produces a no-op or insertion at
the wrong position.

**Mitigation:** `RepairHint::patch()` constructor validates `range.width() > 0`.

### H8: Line/column convention confusion
**Risk:** Tools may report 0-based lines or 1-based columns. Titania standardizes to
1-based lines, 0-based columns.

**Mitigation:** Finding normalizers (§5 of spec) convert tool output to Titania's
convention before constructing `Location::Span`.

### H9: LaneEvidence::parsed_result_digest mismatch
**Risk:** The digest of parsed lane output may not match what the consumer verifies.

**Mitigation:** `parsed_result_digest` is computed from the canonical `LaneEvidence`
serialization (same serde derive, same field order). Consumers recompute and compare.

### H10: CommandEvidence::argv inconsistency
**Risk:** `argv[0]` may not match `executable` if the caller constructs the command
incorrectly.

**Mitigation:** `CommandEvidence::new()` validates `argv[0] == executable`.

---

## 6. Proof Seeds

These are invariants that could be proven using formal methods (Kani, Flux, Verus).
They are NOT obligations — just seeds for future verification work.

### P1: Report::Reject invariant
**Invariant:** `Report::Reject` always has at least one non-empty collection.
**Proof approach (Kani):** Property test that `Report::reject([], [])` panics or returns
`Err`. Property: `∀ code_findings, gate_failings: Report::reject(code, gate).is_ok()
→ (!code.is_empty() ∨ !gate.is_empty())`.

### P2: Lane::from_str determinism
**Invariant:** `Lane::from_str(Lane::to_string(l)) == Ok(l)` for all `l: Lane`.
**Proof approach (Kani):** Property test round-trip serialization/deserialization.

### P3: FindingEffect correctness
**Invariant:** A `Finding` with `effect == Informational` in `code_findings` does not
change `reject_kind()` classification.
**Proof approach (Flux):** Prove that `reject_kind()` depends only on collection lengths,
not on the `FindingEffect` values within them.

### P4: LaneOutcome::Clean exit status
**Invariant:** `LaneOutcome::Clean(evidence)` always has `evidence.exit_status ==
Exited { code: 0 }`.
**Proof approach (Kani):** Property test that constructing `Clean` with non-zero exit
returns `Err`.

### P5: QualityReceipt scope consistency
**Invariant:** `QualityReceipt::scope` matches the scope that determined which lanes ran.
**Proof approach (Flux):** Prove that every `LaneReceipt` lane is a member of the
receipt's `GateScope`.

### P6: WorkspacePath invariants
**Invariant:** All `WorkspacePath` values in `Finding::location` are under the workspace root.
**Proof approach (Flux):** Prove that `WorkspacePath::new` rejects absolute paths and
`..` components.

### P7: RepairHint::Patch non-empty
**Invariant:** `RepairHint::Patch` always has a non-empty replacement range.
**Proof approach (Kani):** Property test that `RepairHint::patch(range, ...)` where
`range.width() == 0` returns `Err`.

---

## 7. Traceability Matrix

| Type | v1-spec.md Section | Notes |
|---|---|---|
| `Lane` | §4 (Lane enum) | 10 variants; unit enum |
| `GateScope` | §4 (GateScope enum) | 3 variants + `#[non_exhaustive]` |
| `SkipReason` | §4 (Skip rules) | 4 variants + future `CacheHit` |
| `Report` | §10 (Report) | Pass/Reject/PolicyError/InputError |
| `RejectKind` | §10 (RejectKind) | CodeOnly/GateOnly/Mixed |
| `Finding` | §10 (Finding) | Struct with 6 fields |
| `FindingEffect` | §10 (FindingEffect) | Reject/Informational |
| `Location` | §10 (Location) | Span/Dependency/Manifest/Workspace/Tool |
| `RepairHint` | §10 (RepairHint) | 7 variants |
| `LaneOutcome` | §10 (LaneOutcome) | Clean/Findings/Failed/Skipped |
| `LaneEvidence` | §10 (LaneEvidence) | Struct with 4 fields |
| `CommandEvidence` | §10 (CommandEvidence) | Struct with 2 fields |
| `LaneFailure` | §10 (LaneFailure) | 4 variants |
| `ProcessTermination` | §10 (ProcessTermination) | 5 variants + Windows note |
| `QualityReceipt` | §10 (QualityReceipt) | Struct with 7 fields + schema_version policy |
| `LaneReceipt` | §10 (LaneReceipt) | Struct with 3 fields |
| `PolicyDiagnostic` | §10 (Diagnostics) | Struct with 3 fields |
| `InputDiagnostic` | §10 (Diagnostics) | Struct with 3 fields |
| `DiagnosticSeverity` | §10 (Diagnostics) | Error/Warning |

---

## 8. Cross-Reference with Existing Code

### Types already in `titania-core`
| Existing Type | New Type Interaction |
|---|---|
| `Digest` | Used in `QualityReceipt` (4 fields), `LaneReceipt` (1 field), `LaneEvidence` (1 field) |
| `RuleId` | Used in `Finding::rule_id` |
| `WorkspacePath` | Used in `Location::Span::file`, `Location::Manifest::file` |
| `TextRange` | Used in `RepairHint::Patch::range` |
| `QualityReceipt` (existing receipt.rs) | **This is a different type** — the spec §10 `QualityReceipt` is the v1 domain model receipt with `schema_version: u16`, `scope: GateScope`, 4 digests, and `lanes: Box<[LaneReceipt]>`. The existing `receipt.rs` `QualityReceipt` is for target-project runs with `target_root`, timestamps, and `Vec<LaneDigest>`. These are two distinct receipt concepts in the codebase. |
| `LaneDigest` (existing receipt.rs) | Precursor to `LaneReceipt`; different structure (has `exit`, `scanned`, `passed`, `finding_count` vs. `evidence_digest`, `clean`) |
| `LaneName` (existing receipt.rs) | Separate from `Lane`; `LaneName` is a string-based lane identifier for the receipt system |

### Module placement (proposed)
New types go in `crates/titania-core/src/lane_model.rs` (or split across multiple modules):
- `lane.rs` — `Lane`, `GateScope`, `SkipReason`
- `report.rs` — `Report`, `RejectKind`
- `finding.rs` — `Finding`, `FindingEffect`, `Location`, `RepairHint`
- `outcome.rs` — `LaneOutcome`, `LaneEvidence`, `CommandEvidence`, `LaneFailure`, `ProcessTermination`
- `lane_receipt.rs` — `QualityReceipt` (v1 spec version), `LaneReceipt`
- `diagnostic.rs` — `PolicyDiagnostic`, `InputDiagnostic`, `DiagnosticSeverity`

Each module re-exports its types into `lib.rs` alongside existing types.
