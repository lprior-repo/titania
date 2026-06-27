# Architecture Spec: Xtask — The Deterministic Rust Quality Gate

> Status: ARCHITECTURE SPEC v6.0 — security machinery cut, quality ratchet
> Next step: build.

---

## 0. Product Sentence

```
Xtask is a deterministic Rust quality gate for AI-authored code. It runs a
pinned local/CI toolchain over real Rust crates and emits a structured report
that tells the AI exactly what to fix. It enforces a strict checked-in Rust
quality policy: formatting, compilation, lint zero-tolerance, panic-surface
discipline, unsafe restrictions, functional-style structural rules, dependency
hygiene, feature-matrix compilation, tests, and mutation-resistance.

Xtask is not a security boundary. It should not be run on untrusted repositories.
It does not prove correctness. It makes low-discipline Rust noisy, visible, and
hard to accidentally accept.
```

Xtask is a quality ratchet. It makes AI-authored Rust obey a strict, mechanical house style and gives the AI typed repair instructions until the code becomes boring, explicit, checked, and harder to break accidentally.

---

## 1. Quality Model

Xtask assumes the code author is an AI or human contributor who may be careless, inconsistent, or inclined to take shortcuts. Xtask is designed to produce deterministic, machine-readable feedback that drives repair loops.

Xtask is NOT a sandbox, not a malware defense, not a secure build system, and not a deployment trust root. It runs Cargo and other developer tools, which may execute repository code. Therefore Xtask should only be run on repositories the user is willing to build locally or in CI.

The goal is code quality enforcement, not hostile-code containment.

---

## 2. Non-Goals

- **NO Xtask-specific authoring macros or DSL.** Ordinary Rust macros (`thiserror`, `serde`/`clap` derives, `assert!` in tests) are allowed through policy.
- **NO LLM inside the gate.** The AI is external; it consumes the report JSON.
- **NO security boundary.** No sandbox, no signing, no deploy-gate, no provenance, no artifact trust.
- **NO formal verification or proofs in v1.**
- **NO bypass flag.** The only escape is a policy-PR.
- **NO claim of omniscience.** Xtask certifies conformance to a pinned policy; it does not prove correctness.

---

## 3. Scope Tiers

| Scope | Lanes | Use case |
|---|---|---|
| `edit` | fmt, check, clippy (source-only), semgrep, panic/assert+build-script scan | AI repair loop (dozens of iterations) |
| `prepush` | edit + tests + supply chain + feature matrix | before push |
| `full` | prepush + mutation testing | CI gate |
| `release` | full + artifact build (`cargo build --release`) | optional release check |

```
edit     = fmt + check + clippy(source) + semgrep + panic/assert+build-script scan
prepush  = edit + cargo test + supply chain + feature matrix
full     = prepush + cargo mutants
release  = full + cargo build --release
```

The `edit` scope is genuinely fast — no supply chain or feature powerset.

---

## 4. Terminology

| Term | Meaning |
|---|---|
| **Report** | Per-invocation structured output (pass/reject + findings). Machine-readable JSON. |
| **QualityReceipt** | Deterministic record of what passed: digests of source, policy, toolchain, per-lane evidence. Unsigned. |
| **Policy** | Checked-in rule files + thresholds + profile config. Changing policy requires a PR. |

No "certificate," no "attestation," no "signature," no "deploy semantics."

---

## 5. Domain Model

### 5.1 Report — single disjoint root, supports mixed failures

```rust
pub enum Report {
    Pass {
        receipt: QualityReceipt,
        per_lane: Box<[LaneOutcome]>,
    },
    Reject {
        code_findings: Box<[Finding]>,
        gate_failures: Box<[LaneFailure]>,
        per_lane: Box<[LaneOutcome]>,
    },
    PolicyError {
        diagnostics: Box<[PolicyDiagnostic]>,
    },
    InputError {
        diagnostics: Box<[InputDiagnostic]>,
    },
}

impl Report {
    pub fn reject_kind(&self) -> Option<RejectKind> { ... }
}

pub enum RejectKind { CodeOnly, GateOnly, Mixed }
```

A real run CAN produce both code findings AND tool failures (e.g. semgrep finds `#[allow]` while cargo-mutants is missing). The Report carries both; `reject_kind()` tells the caller the mix.

### 5.2 Lane outcomes

```rust
pub enum LaneOutcome {
    Clean { evidence: LaneEvidence },
    Findings(Box<[Finding]>),
    Failed(LaneFailure),
    Skipped(SkipReason),
}

pub enum SkipReason {
    PriorCompilationFailure,
    NotSelectedByScope,
    NotApplicable,
    PolicyDisabled,
}
```

### 5.3 Lane evidence (reproducible, not signed)

```rust
pub struct LaneEvidence {
    pub command: CommandEvidence,
    pub tool_version: String,
    pub exit_status: ProcessTermination,
    pub parsed_result_digest: Digest,
}

pub struct CommandEvidence {
    pub executable: String,           // resolved tool name
    pub argv: Box<[String]>,
}

pub enum ProcessTermination {
    Exited { code: i32 },
    Signaled { signal: i32 },
    TimedOut,
    MemoryLimitExceeded,
    SpawnFailed,
}
```

### 5.4 Finding

```rust
pub struct Finding {
    pub lane: Lane,
    pub rule_id: RuleId,
    pub location: Location,
    pub message: String,
    pub repair: RepairHint,
    pub effect: FindingEffect,
}

pub enum FindingEffect {
    Reject,           // causes CodeReject
    Informational,    // advisory only, does not reject
}

pub enum Location {
    Span { file: WorkspacePath, line_start: u32, col_start: u32, line_end: u32, col_end: u32 },
    Dependency { crate_name: String, version: String },
    Manifest { file: WorkspacePath },
    Workspace,
    Tool { name: String, version: String },
}

/// Normalized UTF-8 workspace-relative path. No backslashes, no `..`.
pub struct WorkspacePath(String);

#[derive(Serialize, Deserialize)]
pub enum RepairHint {
    Patch { file: String, range: TextRange, replacement: String },
    UseIteratorPipeline { suggestion: String },
    FlattenNesting { suggestion: String },
    UseCheckedArithmetic { op: String },
    RemoveAllowAttribute { attr: String },
    ReplaceDependency { from: String, to: String },
    RequiresHumanReview { note: String },
}

pub struct TextRange { pub start_byte: u32, pub end_byte: u32 }
```

Line/column: 1-based lines, 0-based columns (Unicode scalar values). `TextRange` uses byte offsets for deterministic patching.

### 5.5 Lane failure categories

```rust
pub enum LaneFailure {
    InfraFailure { tool: String, reason: String },
    ToolFailure { tool: String, termination: ProcessTermination },
    ResourceFailure { tool: String, limit: String },
    SuspiciousFailure { tool: String, evidence: String },
}
```

### 5.6 QualityReceipt

```rust
pub struct QualityReceipt {
    pub schema_version: u16,
    pub scope: GateScope,
    pub source_digest: Digest,
    pub cargo_lock_digest: Digest,
    pub policy_digest: Digest,
    pub toolchain_digest: Digest,
    pub dependency_source_digest: Option<Digest>,
    pub advisory_db_digest: Option<Digest>,
    pub feature_profile_digest: Option<Digest>,
    pub mutation_baseline_digest: Option<Digest>,
    pub lanes: Box<[LaneReceipt]>,
}

pub struct LaneReceipt {
    pub lane: Lane,
    pub evidence_digest: Digest,
    pub clean: bool,
}

pub enum GateScope { Edit, Prepush, Full, Release }
```

No signature. No `expires_at`. No `signing_key_id`. No deploy semantics. If CI wants to enforce, CI runs `xtask gate --scope full` and checks exit code.

---

## 6. The Doctrine (Holzman + functional-rust)

### 6.1 Panic-free standard

`unsafe_code = forbid`. `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`unreachable!`/`dbg!` denied via `-F` (forbid). Production `assert!`/`assert_eq!`/`assert_ne!` scanned (parser-backed for Rust source, rg as coarse prefilter).

Honest residuals: division by zero, char/string boundary ops, third-party panics, Drop panics, dependency-internal panics, macro-expanded panics (semgrep blind). Clippy helps; does not prove panic freedom.

### 6.2 Strict clippy (source-only, correct config placement)

**Critical: clippy runs `--lib --bins` only, NOT `--all-targets`.** `--all-targets` includes tests/benches/examples → nukes normal test code using `unwrap`/`assert!`/loops. Tests compile via `cargo test` and are behavior-gated, NOT style-gated.

**Lint levels in `Cargo.toml [workspace.lints.*]`:**

```toml
# Cargo.toml
[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
nursery = { level = "deny", priority = -1 }
cargo = { level = "deny", priority = -1 }
# restriction group: warn by default (informational), specific critical ones forbidden via -F
unwrap_or_default = "deny"       # real lint
exit = "deny"
default_numeric_fallback = "deny"
missing_errors_doc = "deny"

[workspace.lints.rust]
unsafe_code = "forbid"
unused_must_use = "deny"
unused_results = "warn"
non_exhaustive_omitted_patterns = "deny"
rust_2018_idioms = { level = "deny", priority = -1 }
```

**Thresholds in `clippy.toml`:**

```toml
too-many-lines-threshold = 40
too-many-arguments-threshold = 5
max-fn-params-bools = 1
```

**Critical lints via `-F` on the command line (forbid — `#[allow]` cannot lower):**

```bash
cargo clippy --workspace --lib --bins --frozen -- \
  -F clippy::unwrap_used \
  -F clippy::expect_used \
  -F clippy::panic \
  -F clippy::panic_in_result_fn \
  -F clippy::todo \
  -F clippy::unimplemented \
  -F clippy::indexing_slicing \
  -F clippy::string_slice \
  -F clippy::get_unwrap \
  -F clippy::arithmetic_side_effects \
  -F clippy::dbg_macro \
  -D warnings
```

**`unwrap_or`/`unwrap_or_else` are NOT valid clippy lint IDs.** A blanket ban on `.unwrap_or*` requires a semgrep rule. `unwrap_or_default` IS a real lint.

**Restriction lints = warn by default (informational), specific critical ones forbidden.** `FindingEffect::Informational` carries advisory findings without rejecting. No decorative severity.

**`#[allow]`/`#[expect]` scan:** parser-backed scan for suppression attributes including `#![allow(...)]`, `#![expect(...)]`, `cfg_attr(..., allow(...))`, `#[allow_internal_unstable]`, `#[allow_internal_unsafe]`, and Cargo `[lints]` sections that lower required lints. Un-approved suppression = `BYPASS_*` finding with `FindingEffect::Reject`.

**`--cap-lints allow` for deps:** Cargo caps lints for dependencies. Regime is first-party source only.

### 6.3 Functional-rust doctrine (honest about decidability)

| Rule | Enforcement | Honest caveat |
|---|---|---|
| No imperative loops (`for`/`while`/`loop`) | semgrep | House style, not inherently more verifiable. |
| ≤2 nesting depth | semgrep | House style. |
| No `.unwrap_or*` family | semgrep | STYLE — `unwrap_or` does NOT panic. |
| No `Result<T, String>` | semgrep | Pattern check, not proof of error quality. |
| No wildcard arms in domain match | semgrep (warn) | `#[non_exhaustive]` external enums FORCE wildcards — needs exceptions. |
| No bool control flags | `fn_params_excessive_bools` (warn) | Checks declarations, not every bool. |
| Parse don't validate | NOT a hard gate | Architectural guidance. |
| Zero-copy | NOT enforced | Clippy catches "some" cases. |
| No hidden I/O | NOT decidable by semgrep | Call behind trait/dep/callback can do I/O. Reject only direct calls to policy-disallowed I/O APIs. |
| No recursion | semgrep (syntactic only) | Catches direct recursion only, not mutual/trait/fn-pointer. |

### 6.4 Supply-chain (honest, operationally separated)

| Tool | Does | Does NOT |
|---|---|---|
| `cargo audit` | Checks Cargo.lock vs KNOWN RustSec advisories | Unknown vulnerabilities. |
| `cargo deny` | Advisories + licenses + bans + sources + dupes | Overlaps audit (intentional defense-in-depth). |
| `cargo vet` | Third-party deps have trusted-entity audits; reports gaps | NOT a correctness proof. Bootstrap cost real. |
| `cargo geiger` | Counts `unsafe` in dep tree | Does not prove soundness. |
| `cargo machete` | Detects unused deps | EXPLICITLY IMPRECISE — baseline/triage, not blanket reject. |
| `cargo hack --feature-powerset` | Every feature combo compiles | Does NOT prove target-specific code compiles. |

**Conflict resolution:** any advisory from either cargo-audit OR cargo-deny rejects. Duplicates normalized by advisory ID.

**cargo-machete baseline:** ignore/baseline is a policy file with owner/reason/expiry per entry.

**cargo-geiger thresholds by dependency class:** runtime/build/proc-macro = strict; dev = lenient.

**cargo-vet modes:** `enforce` (deploy gate), `report`, `bootstrap` (onboarding).

Supply-chain checks distinguish runtime, build, dev, and proc-macro dependencies.

### 6.5 Unsafe policy

v1: **zero first-party `unsafe`** (`forbid`). Dependency unsafe measured (geiger), not forbidden by rustc. No SIMD/FFI waiver in v1.

### 6.6 Pinned toolchain + hermeticity

- `rust-toolchain.toml` with version-pinned channel (`1.x.y` explicit or `nightly-YYYY-MM-DD`).
- **Do NOT invoke rustup shims.** Resolve absolute `cargo`/`rustc` binaries from the trusted toolchain dir.
- **`CARGO_HOME`** set to a controlled, read-only, digest-bound directory. **`RUSTUP_HOME`** same.
- **Reject parent-directory cargo configs.** Cargo searches parent dirs and `$CARGO_HOME/config.toml`. Xtask runs from canonical workspace root.
- `.cargo/config` (extensionless, legacy) AND `.cargo/config.toml` both checked.
- `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, `RUSTC_WRAPPER`, `RUSTC_WORKSPACE_WRAPPER` scanned, frozen.
- `RUSTC_BOOTSTRAP` = violation.
- `--frozen` everywhere, not `--locked`. `--frozen` = `--locked` + `--offline`.

### 6.7 Allowed library policy (profile-scoped, not universal)

The approved/banned crate table is a **named policy profile** (`strict-ai`), in `.xtask/profiles/strict-ai/policy.toml`. Core Xtask supports crate allowlists; the default profile is opinionated. Not "arbitrary Rust" — "Rust written for a very opinionated profile."

### 6.8 Test & Mutation Evidence

- `cargo test --workspace --frozen -- --test-threads=1` for deterministic harness configuration.
- Xtask runs tests in a deterministic harness configuration where practical: single-threaded, fixed env, policy-declared seeds for known frameworks. Xtask does NOT prove tests are deterministic.
- `TEST_NONDETERMINISTIC` emitted only for explicit known cases: missing declared proptest seed, forbidden raw random source in tests, test thread count > 1.
- Property tests (proptest/quickcheck) treated as ordinary deterministic tests unless policy declares specific seed/corpus inputs.
- `cargo mutants` runs in full scope, rejects surviving non-baselined mutants.
- Equivalent/intentionally surviving mutants recorded in checked-in baseline:

```rust
pub struct MutantBaselineEntry {
    pub mutant_id: String,
    pub file: WorkspacePath,
    pub function: String,
    pub reason: String,
    pub owner: String,
    pub expires_on: NaiveDate,
}
```

- cargo-mutants config (`.cargo/mutants.toml`) included in policy digest.
- **Doctests** run by `cargo test` by default — v1: allowed, same treatment as tests.

### 6.9 Generated code

`include!(concat!(env!("OUT_DIR"), ...))` is **banned in v1**. If build-time generation is needed (tonic/prost/bindgen/lalrpop/sqlx), generated Rust must be checked into source and gated like normal code. Revisit generated-code manifest support post-v1.

---

## 7. The Enforcement Lanes

| Layer | Tool(s) | Scope | Depends on compile? | Rejects |
|---|---|---|---|---|
| 0 | `cargo fmt --check` | edit | No | formatting drift |
| 1 | `cargo check --workspace --frozen` + rustc lints | edit | — | compile errors, denied lints |
| 2 | `cargo clippy --workspace --lib --bins --frozen` with `-F` critical lints (source-only; NOT `--all-targets`) | edit | Yes | denied lints |
| 3 | semgrep / structural source rules + `#[allow]`/`#[expect]` scan | edit | **No** | structural violations, policy-consistency violations |
| 4 | production panic/assert scan (parser-backed) + manifest-declared build-script scan | edit | No | production panic macros; build-script violations |
| 5 | `cargo test --workspace --frozen -- --test-threads=1` | prepush | Yes | failing tests |
| 6 | supply chain: `cargo audit` + `cargo deny` + `cargo vet` + `cargo geiger` + `cargo machete` | prepush | No | advisories, bans, licenses, vet gaps, unsafe-dep, unused deps |
| 7 | `cargo hack check --workspace --feature-powerset --frozen` (bounded-depth per policy) | prepush | Yes | broken feature combination |
| 8 | `cargo mutants` (with `.cargo/mutants.toml` + baseline) | full | Yes | surviving non-baselined mutants |
| 9 | `cargo build --workspace --release --frozen` | release | Yes | build failure |

**Skip rules:** compilation-dependent lanes (2, 5, 7, 8, 9) skip if Layer 1 fails. Lanes 0, 1, 3, 4 always run. Lane 3 (semgrep) runs on source regardless of compilation.

**Feature matrix policy:** full powerset can explode. Policy declares `mode = "powerset" | "bounded-depth" | "declared"` with depth/grouping/exclusions. QualityReceipt records the mode.

**cargo-hack modes that modify manifests (`--no-dev-deps`, `--no-private`):** banned — conflict with deterministic source digest.

**Recommended CI hygiene (not required):** run in a clean CI job with fixed tool versions, offline dependencies when possible, and no unnecessary secrets.

---

## 8. Policy Consistency Checks (not "bypass countermeasures")

These are rejected because they **silently weaken the quality policy**, not because the author is an attacker.

| Consistency check | What it catches |
|---|---|
| `#[allow(...)]` / `#[expect(...)]` scan | un-approved lint suppressions in first-party source |
| `#![allow(...)]` / `#![expect(...)]` / `cfg_attr(..., allow(...))` | crate-level suppressions |
| `#[allow_internal_unstable]` / `#[allow_internal_unsafe]` | internal escape hatches |
| Cargo `[lints]` weakening | manifests that lower required lints |
| semgrep ignore comments | suppressed structural rules |
| cargo-audit `--ignore` / cargo-deny exceptions | suppressed advisories |
| cargo-vet exemptions | suppressed vet requirements |
| cargo-mutants `--exclude` | suppressed mutation tests |
| `.cargo/config.toml` wrapper/flag overrides | toolchain tampering |
| `RUSTFLAGS` / `RUSTC_WRAPPER` unexpected values | toolchain tampering |

All suppressions/exceptions must be in checked-in policy files with owner, reason, and expiry.

---

## 9. Escape Hatch (anti-circular policy)

**NO per-site bypass.** The only escape is editing policy files. Anti-circularity: a policy-PR is checked against the **PREVIOUS main-branch policy** (not the weakened one). A meta-policy requires:
- CODEOWNER approval on any policy file change.
- Explicit diff classification (tightening / loosening / neutral). Loosening flagged for review.
- The policy-PR passes the gate under the PREVIOUS policy.

**Gate-control surface** (all require CODEOWNER + previous-policy evaluation):
```
.xtask/**  .moon/**  .github/workflows/**  Cargo.toml [workspace.lints]
rust-toolchain*  .cargo/**  clippy.toml  rustfmt.toml  deny.toml
```

---

## 10. Error Taxonomy

### Report root
`Pass | Reject{code_findings, gate_failures} | PolicyError | InputError`

### Lane failures
`InfraFailure | ToolFailure | ResourceFailure | SuspiciousFailure`

### Rule families
`HOLZMAN_PANIC_*`, `HOLZMAN_UNSAFE_*`, `HOLZMAN_CHECKED_*`, `FUNC_LOOPS_*`, `FUNC_NESTING_*`, `FUNC_STYLE_*`, `SUPPLY_ADVISORY`, `SUPPLY_LICENSE`, `SUPPLY_BANNED_CRATE`, `SUPPLY_VET_GAP`, `SUPPLY_UNUSED_DEP`, `SUPPLY_UNSAFE_DEP_THRESHOLD`, `FEATURE_COMBO_FAILED`, `TEST_FAILURE`, `TEST_NONDETERMINISTIC`, `MUTANT_SURVIVED`, `MUTANT_BASELINE_EXPIRED`, `MUTANT_BASELINE_UNOWNED`, `BYPASS_*`, `POLICY_*`, `INPUT_*`, `GATE_*`, `BUILD_*`

### Finding effect
`FindingEffect::Reject` (causes CodeReject) | `FindingEffect::Informational` (advisory only). No decorative severity.

---

## 11. Toolchain Requirements (scope-tiered)

| Scope | Hard-required |
|---|---|
| `edit` | `cargo`, `rustc`, `rustfmt`, `clippy`, `rg`, `semgrep` |
| `prepush` | edit + `cargo-audit`, `cargo-deny`, `cargo-vet`, `cargo-geiger`, `cargo-machete`, `cargo-hack` |
| `full` | prepush + `cargo-mutants` |
| `release` | full + (artifact build uses `cargo build`) |

Optional in tests (Xtask v1 doesn't know about them unless via `cargo test`): `proptest`, `quickcheck`, `insta`, `rstest`.

All pinned. `doctor` reports available vs **trusted** (expected version, actual version, resolved path, source of installation, policy-required?).

---

## 12. Moon CI/CD Integration

```yaml
# .moon/tasks/all.yml
gate-edit:
  command: 'xtask gate --scope edit --emit json'
  toolchains: [rust]
  options: { runInCI: true }
  inputs: ['@globs(sources)', '.xtask/**', 'Cargo.toml', 'Cargo.lock', '**/Cargo.toml',
           '.cargo/**', 'rustfmt.toml', 'clippy.toml', 'rust-toolchain.toml']

gate-full:
  command: 'xtask gate --scope full --emit json'
  toolchains: [rust]
  options: { runInCI: true }
  inputs: ['@globs(sources)', '.xtask/**', 'Cargo.toml', 'Cargo.lock', '**/Cargo.toml',
           '.cargo/**', 'rustfmt.toml', 'clippy.toml', 'deny.toml', 'supply-chain/**',
           'rust-toolchain.toml', '.cargo/mutants.toml', '.xtask/mutants-baseline.json',
           '.xtask/advisory-db-snapshot/', '.xtask/feature-matrix.toml']
```

CI requires `gate-full` exit code 0 before merge. Moon's source-lint zero-tolerance (`-W clippy::all`) reinforces Layer 2.

---

## 13. Component / Module Map

Single Cargo workspace:
- `xtask-bin` — CLI (clap). Subcommands: `gate`, `doctor`, `explain`.
- `xtask-core` — domain types: `Report`, `Finding`, `RepairHint`, `Location`, `LaneOutcome`, `SkipReason`, `LaneFailure`, `LaneEvidence`, `QualityReceipt`, `GateScope`, `FindingEffect`.
- `xtask-policy` — policy loading, validation, `strict-ai` profile, `policy_digest`.
- `xtask-lanes` — lane runners: `fmt`, `rustc`, `clippy`, `semgrep`, `assert_build_scan`, `test`, `supply`, `feature`, `mutants`, `build`.
- `xtask-bypass` — policy consistency checks (parser-backed `#[allow]` scan, etc.).
- `xtask-output` — report JSON schema (versioned), `doctor` diagnostics, `explain` rule catalog.
- `xtask-ledger` — (optional, post-v1) SQLite audit ledger.

All first-party crates pass their own gate (dogfooded).

---

## 14. CLI Surface

```
xtask gate [--scope edit|prepush|full|release] [--emit json] [--out <path>]
    Run scoped quality lanes. Emit report JSON + quality receipt on pass.

xtask doctor [--scope <scope>]
    Report required tools, versions, resolved paths, and policy health for the scope.

xtask explain <rule-id>
    Explain a rule and show examples of accepted repairs.
```

Exit codes: `0` Pass, `1` Reject, `2` PolicyError, `3` InputError, `>=4` internal.

---

## 15. Definition of Done — Xtask v1

1. `xtask gate --scope edit` runs fmt, cargo check, production-source Clippy, Semgrep structural rules, suppression scans, and production panic/assert scans.
2. `xtask gate --scope prepush` adds cargo test, supply-chain checks, and feature-matrix compilation.
3. `xtask gate --scope full` adds cargo-mutants with a checked-in mutation baseline.
4. The report schema is stable, versioned, machine-readable, and separates code findings from gate/tool failures.
5. The default `strict-ai` policy forbids first-party unsafe, unwrap/expect, panic macros, unchecked indexing, unchecked arithmetic, unapproved lint suppressions, imperative loops, excessive nesting, and core `Result<T, String>`.
6. Tests compile and pass but are not production-style-gated unless policy opts in.
7. All policy exceptions live in checked-in policy files with owner, reason, and expiry.
8. `xtask doctor --scope <scope>` reports the exact tools required for that scope and verifies pinned versions/paths.
9. Xtask's own repository passes `xtask gate --scope full`.
10. Killer demo: AI writes Rust with a `for` loop and `.unwrap()`; `xtask gate --scope edit` rejects with typed `FUNC_LOOPS_*` and `HOLZMAN_PANIC_UNWRAP` findings; AI repairs; the gate passes and emits a quality receipt.

---

## 16. References

- `holzman-rust/SKILL.md` + all 6 references (nasa-jpl-standards, latency-throughput-playbook, runtime-performance-architecture, zero-cost-abstractions, simd-patterns, mechanical-empathy-toolchain)
- `functional-rust/SKILL.md` + all 3 references (scott-ddd-types, typing-refactor-checklist, complete-workflow)
- `moon-v2/SKILL.md`
