# Architecture Spec: Xtask — The Deterministic Rust Quality Gate

> Status: ARCHITECTURE SPEC v4.0 — proof machinery removed, honest evidence gate
> Next step: run `arch-spec-to-beads` to shred this into molecular tasks.

---

## 0. Product Sentence

```
Xtask is a deterministic Rust quality gate for AI-authored code. It runs a
pinned, hermetic toolchain over real Rust crates and emits a structured verdict
plus a signed certificate only when every policy-selected lane passes. It
enforces panic-surface discipline, unsafe restrictions, lint zero-tolerance,
functional-style structural rules, supply-chain hygiene, feature-matrix
compilation, tests, and mutation-resistance. It does not prove behavioral
correctness; it certifies conformance to a checked-in quality policy.
```

Xtask is not a theorem prover. It is a deterministic evidence gate. It makes low-discipline Rust expensive to submit, easy for an AI to repair, and impossible to deploy without a signed record of exactly which quality checks passed.

---

## 1. Threat Model

**The AI code author is potentially MALICIOUS, not merely buggy or careless.**

The design assumes the author will attempt to bypass the gate via: `#[allow]`/`#[expect]`, `.cargo/config.toml`, build scripts, proc macros, `include!`/`#[path]`, `cfg`-gated code, PATH poisoning, tool-native suppressions (semgrep ignores, cargo-audit `--ignore`, cargo-vet exemptions, cargo-mutants `--exclude`), generated code, and dependency tricks.

**The signing key is a crown-jewel secret.** Cargo executes repository-controlled code through build scripts, proc macros, tests, examples, and mutation harnesses. The signing key must NEVER exist in any environment that runs a Cargo command on untrusted code. See §9 (two-environment architecture). Dropping the proof layer does NOT let you drop this hardening.

**Xtask's own source is trusted** (gated by itself, reviewed). The GATED code is untrusted.

---

## 2. Non-Goals

- **NO Xtask-specific authoring macros or DSL.** Ordinary Rust macros (`thiserror`, `serde`/`clap` derives, `assert!` in tests) are allowed through policy.
- **NO LLM inside the gate.** The AI is external; it consumes the verdict JSON.
- **NO formal verification, proofs, bounded model checking, refinement types, UB interpreters, or concurrency model checking in v1.** Xtask does not claim to prove algorithmic correctness, memory-model correctness, concurrency correctness, or semantic equivalence.
- **NO bypass flag.** No `--force`. The only escape is a policy-PR checked against the PREVIOUS policy plus a meta-policy (§14).
- **NO code generation as source of truth.** Xtask emits typed repair hints, never the author's final code.
- **NO claim of omniscience.** Xtask certifies conformance to a pinned policy; it does not prove behavioral correctness.

---

## 3. Positioning — The AI OODA Loop + Scope Tiers

```
OBSERVE   AI reads context, existing code, Xtask's prior verdict
ORIENT    AI generates candidate Rust
DECIDE    Xtask gate — runs the toolchain in a sandbox, emits verdict + certificate
ACT       Accepted code (valid, fresh, signed certificate) ships
```

The AI is the primary invoker, running Xtask repeatedly in a repair loop.

**Scope tiers (the "edit" loop is actually fast — no supply chain or feature powerset):**

| Scope | Lanes | Use case |
|---|---|---|
| `edit` | fmt, check, clippy, semgrep, panic/assert scan | AI repair loop (dozens of iterations) |
| `prepush` | edit + tests + supply chain + feature matrix | before push |
| `full` | prepush + mutation testing | CI / deploy-gate |

```
edit     = fmt + check + clippy + semgrep + panic/assert scan
prepush  = edit + tests + supply chain + feature matrix
full     = prepush + mutation testing
```

Do NOT call layers 0–6 "fast" if supply-chain and feature powerset are included. They aren't fast. The `edit` scope is fast.

---

## 4. EARS Requirements

### Event-driven
- **When** the AI submits a crate or diff, **the system shall** run the scoped lanes, short-circuiting ONLY compilation-dependent lanes when Layer 1 (check) fails. Semgrep (Layer 3) runs on source REGARDLESS of compilation status — it does not need the crate to typecheck, and skipping it on compile failure reduces repair signal.
- **When** any scoped lane cannot produce a result (tool crash, timeout, missing binary), **the system shall** emit a `GateReject` with no certificate.
- **When** all scoped lanes emit clean, **the system shall** emit `Pass`, compute the evidence digest, and write canonical evidence artifacts. Signing happens in a SEPARATE environment (§9).

### State-driven
- **While** a certificate is valid, fresh (within `expires_at`), and its evidence digest matches the current artifact+policy+advisory-db, **the deploy-gate shall** permit deployment.
- **If** the advisory DB has changed since the certificate was issued, **the deploy-gate shall** REJECT regardless of signature validity.

### Unwanted
- **If** the signing key is present in any environment that executes a Cargo command on untrusted code, **the system design is BROKEN.** This is an invariant (§9).
- **If** a tool-native suppression (`#[allow]`, `#[expect]`, semgrep ignore, cargo-audit ignore, etc.) is found in gated source, **the system shall** flag it unless explicitly policy-approved.

---

## 5. Domain Model

### 5.1 The verdict type — single disjoint root enum

```rust
pub enum Report {
    Pass { evidence: EvidenceDigest, per_lane: Box<[LaneOutcome]> },
    CodeReject { findings: Box<[Finding]>, per_lane: Box<[LaneOutcome]> },
    GateReject { failures: Box<[LaneFailure]>, per_lane: Box<[LaneOutcome]> },
    PolicyError(PolicyDiagnostic),
    InputError(InputDiagnostic),
}
```

`CodeReject` = the Rust is wrong (AI edits code). `GateReject` = a tool crashed/missing/timeout (AI does NOT edit code). `PolicyError` = policy malformed (edit policy). These are type-disjoint with disjoint fix paths.

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

### 5.3 Finding

```rust
pub struct Finding {
    pub lane: Lane,
    pub rule_id: RuleId,
    pub location: Location,
    pub message: String,
    pub repair: RepairHint,
}

pub enum Location {
    Span { file: PathBuf, line_start: u32, col_start: u32, line_end: u32, col_end: u32 },
    Dependency { crate_name: String, version: String },
    Manifest { file: PathBuf },
    Workspace,
    Tool { name: String, version: String },
    Artifact { digest: Digest },
}
```

### 5.4 Repair hints (serializable, JSON-friendly)

```rust
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
```

### 5.5 Lane evidence

```rust
pub struct LaneEvidence {
    pub command: String,
    pub tool_path_hash: Digest,
    pub tool_version: String,
    pub env_digest: Digest,
    pub stdout_digest: Digest,
    pub stderr_digest: Digest,
    pub duration_ms: u64,
    pub exit_status: i32,
    pub parsed_result_digest: Digest,
}
```

---

## 6. The Doctrine (Holzman + functional-rust)

### 6.1 Panic-free standard

`unsafe_code` = forbid. `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`unreachable!`/`dbg!` denied. Production `assert!`/`assert_eq!`/`assert_ne!` (the rg scan). Build scripts get their own lane (they EXECUTE during builds).

Honest caveats: `panic_in_result_fn` cannot prove a function doesn't panic (called functions may). `unused_must_use` only catches `#[must_use]` types (add `unused_results` as warn). Indexing ban catches `items[i]` but not all panic paths.

### 6.2 Strict clippy (STUPIDLY strict — all groups maxed)

Start with ALL groups denied. Tests exempt from style (compile + behavior only).

```toml
all = { level = "deny", priority = -1 }
pedantic = { level = "deny", priority = -1 }
nursery = { level = "deny", priority = -1 }
cargo = { level = "deny", priority = -1 }
restriction = { level = "warn", priority = -1 }
```

Critical lints use `-F` (forbid) on the command line, NOT workspace `-D` — `#[allow]` cannot lower a forbid:
```
-F clippy::unwrap_used -F clippy::expect_used -F clippy::panic
-F clippy::indexing_slicing -F clippy::string_slice -F clippy::get_unwrap
-F clippy::arithmetic_side_effects
```

Functional-rust style denies (STYLE, not panic-safety — `unwrap_or` does NOT panic):
```toml
unwrap_or_default = "deny"
unwrap_or_else = "deny"
unwrap_or = "deny"
too_many_lines = "deny"         # threshold 40
too_many_arguments = "deny"     # threshold 5
exit = "deny"
default_numeric_fallback = "deny"
missing_errors_doc = "deny"
fn_params_excessive_bools = "warn"  # CORRECTED: not fn_args_justly (doesn't exist)
wildcard_enum_match_arm = "warn"    # house style; #[non_exhaustive] needs exceptions
```

NOTE: `non_exhaustive_omitted_patterns` is the correct lint name. `#[non_exhaustive]` external enums FORCE wildcard arms — the no-wildcard rule needs explicit exceptions.

`#[allow]`/`#[expect]` scan: Xtask scans gated source for suppression attributes. ANY un-approved suppression = `BYPASS_ALLOW_ATTRIBUTE` finding → CodeReject.

`--cap-lints allow` for dependencies: Cargo caps lints for deps. The regime applies to FIRST-PARTY source only.

Xtask generates CUSTOM repair hints, not raw clippy suggestions (clippy's own suggestions can conflict with policy).

### 6.3 Functional-rust doctrine (honest about decidability)

| Rule | Enforcement | Honest caveat |
|---|---|---|
| No imperative loops | semgrep | House style, not inherently more verifiable. |
| ≤2 nesting depth | semgrep | House style. |
| No `unwrap_or*` | clippy deny | STYLE — `unwrap_or` does NOT panic. |
| No `Result<T, String>` | semgrep | Pattern check, not proof of error quality. |
| No wildcard arms | semgrep (warn) | `#[non_exhaustive]` forces wildcards — needs exceptions. |
| No bool flags | `fn_params_excessive_bools` (warn) | Checks declarations, not every bool. |
| Parse don't validate | NOT a hard gate | Architectural guidance. |
| Zero-copy | NOT enforced | Clippy catches "some" cases. |
| No hidden I/O | NOT decidable by semgrep | Call behind trait/dep/callback can do I/O. |
| No recursion | semgrep (syntactic only) | Catches direct recursion only. |

### 6.4 Supply-chain (honest about tool limits)

| Tool | Does | Does NOT |
|---|---|---|
| `cargo audit` | Checks Cargo.lock vs KNOWN RustSec advisories | Unknown vulnerabilities. |
| `cargo deny` | Advisories + licenses + bans + sources + dupes | Overlaps audit on advisories (intentional). |
| `cargo vet` | Third-party deps have trusted-entity audits; reports gaps | NOT a correctness proof. Bootstrap cost real. |
| `cargo geiger` | Counts `unsafe` in dep tree | Does not prove soundness. |
| `cargo machete` | Detects unused deps | EXPLICITLY IMPRECISE — baseline/triage, not blanket reject. |
| `cargo hack --feature-powerset` | Every feature combo compiles | Does NOT prove target-specific code compiles. |

Supply-chain checks distinguish runtime, build, dev, and proc-macro dependencies.

### 6.5 Unsafe policy

v1 is **zero first-party `unsafe`** (`unsafe_code = forbid`). No SIMD waiver, no FFI exception, no raw-pointer accounting. If a genuine need arises, it requires a policy-PR.

### 6.6 Pinned toolchain

`rust-toolchain.toml` with a **version-pinned** channel (`1.x.y` explicit, or date-pinned `nightly-YYYY-MM-DD`). "Stable" floats — use explicit version. Components: `rustfmt`, `clippy`, `rust-src`, `llvm-tools-preview`. `RUSTC_BOOTSTRAP` = violation. `portable_simd`/`try_blocks` are nightly-only — unavailable on stable pin. `RUSTFLAGS`/`RUSTC_WRAPPER`/`RUSTC_WORKSPACE_WRAPPER` scanned and constrained (§13).

### 6.7 Allowed library & crate policy

Curated allowlist via `cargo deny` + clippy `disallowed_methods`/`disallowed_types` + semgrep. Adding a non-approved crate requires a policy-PR.

| Purpose | Approved | Banned |
|---|---|---|
| Async I/O | `tokio` | — |
| HTTP | `axum`, `tower`, `tower-http`, `hyper` | — |
| CPU parallelism | `rayon` (scaling evidence) | — |
| Concurrency | `crossbeam-channel`, `parking_lot`, `flume` (bounded) | `std::sync::Mutex` in async scope |
| Buffers | `bytes`, `arrayvec`, `smallvec`, `heapless` | — |
| Maps | `hashbrown`, `ahash`, `rustc-hash` (internal keys) | fast non-crypto hasher for adversarial keys |
| Formats | `postcard`, `serde_json` | new `bincode` |
| Errors | `thiserror` (core), `anyhow` (shell) | `Result<T, String>` in core |
| Parsing | `winnow`, `nom`, `lexical-core` | — |
| Hashing | `blake3`, `crc32fast` | `chrono::Local`, raw `rand::random()` |

### 6.8 Test & Mutation Evidence

Xtask does not perform formal verification in v1. It does not claim to prove algorithmic correctness, memory-model correctness, concurrency correctness, or semantic equivalence.

Instead, Xtask enforces deterministic test and mutation evidence:

- `cargo test --workspace --locked` must pass.
- Tests must run with fixed seeds where randomness is used.
- Property tests may be used by the codebase, but Xtask treats them as ordinary deterministic tests unless policy declares specific seed/corpus inputs.
- `cargo mutants` runs in the full scope and rejects surviving non-baselined mutants.
- Equivalent or intentionally surviving mutants must be recorded in a checked-in mutation baseline with owner, reason, and expiry.
- The mutation baseline digest is bound into the certificate.

A passing test/mutation lane means the code satisfied the recorded test evidence for the recorded policy, seeds, dependencies, and toolchain. It is not a proof that the implementation is correct.

---

## 7. The Enforcement Lanes

| Layer | Tool(s) | Scope | Rejects |
|---|---|---|---|
| 0 | `cargo fmt --check` | edit/prepush/full | formatting drift |
| 1 | `cargo check --workspace --locked` + rustc lints (`-D warnings`, `-F unsafe_code`, deny `unused_must_use`/`unused_results`/exhaustiveness) | edit/prepush/full | compile errors, denied lints |
| 2 | `cargo clippy --workspace --all-targets --locked` with policy profile (source-only style gating; critical lints via `-F`) | edit/prepush/full | denied lints |
| 3 | semgrep / structural source rules (no-loops, nesting, `unwrap_or*`, wildcard arms, `Result<_, String>`, hidden I/O patterns, `#[allow]` scan) | edit/prepush/full | functional-rust structural violations, bypass attributes |
| 4 | production panic/assert scan (`rg` for `assert!`/`assert_eq!`/`assert_ne!`/`unreachable!` outside tests/benches/examples) + build.rs own scan | edit/prepush/full | production panic/assert macros; build-script violations |
| 5 | `cargo test --workspace --locked` | prepush/full | failing tests |
| 6 | supply chain: `cargo audit` + `cargo deny` + `cargo vet` + `cargo geiger` + `cargo machete` (with triage baseline) | prepush/full | advisories, banned crates, license violations, vet gaps, unsafe-dep threshold, unused deps |
| 7 | `cargo hack check --workspace --feature-powerset --locked` | prepush/full | broken feature combination |
| 8 | `cargo mutants` (with baseline + equivalent-mutant triage) | full | surviving non-baselined mutants |

**Skip rules:** layers that depend on compilation (2, 5, 7, 8) skip if Layer 1 fails. Layers 0, 1, 3, 4 run regardless (3/4 don't need compilation). Feature-powerset (7) depends on compilation — skip if L1 fails.

**Build scripts get their own scan (within Layer 4):** build.rs files EXECUTE during builds and are attacker-controllable. They are NOT excluded from scanning.

---

## 8. Certificate Model (artifact-bound, fresh, split deterministic/signed)

### 8.1 Evidence (deterministic — identical input produces identical digest)

```rust
pub struct Evidence {
    pub schema_version: u16,
    pub source_digest: Digest,
    pub cargo_lock_digest: Digest,
    pub artifact_digest: Option<Digest>,      // Required for deploy certs
    pub policy_digest: Digest,
    pub toolchain_digest: Digest,             // hash of RESOLVED BINARIES or hermetic OCI digest
    pub dependency_source_digest: Digest,     // vendored dep tree digest
    pub advisory_db_digest: Digest,           // pinned RustSec/deny DB snapshot
    pub feature_profile_digest: Digest,
    pub mutation_baseline_digest: Option<Digest>,
    pub per_layer: Box<[LayerDigest]>,
    pub scope: GateScope,                     // Edit | Prepush | Full
}
```

### 8.2 Attestation (non-deterministic, SIGNED — detached over canonical pre-sign payload)

```rust
pub struct Attestation {
    pub evidence_digest: Digest,              // blake3 of canonical-serialized Evidence
    pub issued_at_utc: DateTime<Utc>,
    pub expires_at_utc: DateTime<Utc>,
    pub signing_key_id: String,
    pub signature: Ed25519Signature,          // detached, over evidence_digest + fields above
}
```

The signature is DETACHED over a canonical pre-sign payload. It does not sign itself.

### 8.3 Scope conflation fix

An attestation's Evidence encodes `scope`. Deploy-gate REQUIRES `scope = Full`. A `scope = Edit` attestation (from the fast loop) is NOT deploy-acceptable regardless of signature.

### 8.4 Canonical serialization

Canonical JSON (sorted keys, no whitespace), sorted findings (by lane, rule_id, location), normalized paths (workspace-relative, forward-slash), scrubbed volatile fields (durations stripped from evidence digests), fixed locale (C/POSIX), fixed timezone (UTC).

---

## 9. Two-Environment Architecture (signing-key isolation — CRITICAL)

**THE SIGNING KEY MUST NEVER EXIST IN ANY ENVIRONMENT THAT EXECUTES A CARGO COMMAND ON UNTRUSTED CODE.**

```
┌─────────────────────────────────────────────────────────────┐
│ UNTRUSTED EXECUTION JOB (verifier sandbox)                   │
│                                                              │
│  Sandbox: network-off, readonly source, frozen PATH,        │
│  no secrets, CPU/mem limits, fixed env/locale/timezone       │
│                                                              │
│  Runs: cargo check/clippy/test/mutants/hack, semgrep,        │
│        rg, build scripts, proc macros, tests                 │
│                                                              │
│  HAS NO SIGNING KEY                                          │
│                                                              │
│  OUTPUTS: evidence bundle + evidence digest                  │
└──────────────────────┬──────────────────────────────────────┘
                       │ canonical evidence only (no live checkout)
                       ▼
┌─────────────────────────────────────────────────────────────┐
│ SIGNING JOB (hardened, no code execution)                    │
│                                                              │
│  Receives ONLY: evidence digest, source digest, artifact     │
│  digest, policy digest, advisory-db digest                   │
│                                                              │
│  HAS THE SIGNING KEY (KMS/HSM or keyless CI identity)        │
│                                                              │
│  OUTPUTS: signed Attestation                                 │
└─────────────────────────────────────────────────────────────┘
```

The signer NEVER receives a live checkout. It receives digests and canonical evidence. It cannot execute code. An attacker who compromises the execution job cannot sign — the key isn't there.

---

## 10. Sandbox Profile

| Control | Value | Why |
|---|---|---|
| Network | OFF | Determinism; no dependency fetching, no live advisory DB |
| Source tree | READ-ONLY | Build scripts cannot mutate source; compare source digest before/after |
| Writable | `target/`, `OUT_DIR`, temp only | Build artifacts only |
| Secrets | ABSENT | No signing key, no tokens |
| PATH | FROZEN — resolved absolute tool paths from trusted toolchain dir | No PATH poisoning |
| Env vars | Fixed, frozen, digest-bound | `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`, `CARGO_NET_OFFLINE=true`, `--locked`, `--frozen` |
| Locale | Fixed (C/POSIX) | Deterministic output |
| Timezone | Fixed (UTC) | Deterministic timestamps |
| CPU/Memory | Cgroup-capped | mutants can run away |
| `.cargo/config.toml` | In policy digest, constrained | Build flags/wrappers are a policy surface |

Network-off means: dependencies vendored or `--frozen`/`--locked`. Advisory DBs are pinned snapshots (digests in certificate), not live-fetched.

---

## 11. Policy Files (what's in policy_digest)

```
clippy.toml
rustfmt.toml
.xtask/semgrep/           (all rules)
deny.toml
rust-toolchain.toml
cargo-vet supply-chain/   (vet policy + imports)
.xtask/policy.toml        (hot-module sets [future], scope config, mutation baseline ref)
.cargo/config.toml        (constrained)
Cargo.toml [lints]        (workspace lint config)
.xtask/mutants-baseline.json  (surviving-mutant triage)
```

---

## 12. Bypass-Surface Countermeasures

| Bypass vector | Countermeasure |
|---|---|
| `#[allow(...)]` / `#[expect(...)]` | rg/semgrep scan; un-approved = CodeReject. Critical lints use `-F` (can't be lowered). |
| `cfg_attr(..., allow(...))` | same scan |
| `.cargo/config.toml` | in policy_digest; constrained; scanned for wrapper overrides |
| `RUSTFLAGS` / `CARGO_ENCODED_RUSTFLAGS` | scanned; unexpected rejected; frozen in env_digest |
| `RUSTC_WRAPPER` / `RUSTC_WORKSPACE_WRAPPER` | scanned; rejected unless policy-approved |
| PATH poisoning | frozen absolute tool paths |
| build scripts | own scan (Layer 4); readonly source; digest compared before/after |
| proc macros | execute at compile time; mitigated by signing-key isolation (§9) + supply-chain scan |
| `include!` / `#[path]` | canonical discovery; symlink handling; source-escape ban |
| generated OUT_DIR code | scanned and ledgered |
| `cfg`-gated code | semgrep parses all branches conservatively; target ≠ feature powerset |
| `#[cfg(test)]` in src/*.rs | cfg-aware scanning (not falsely production) |
| semgrep ignore comments | scanned; un-approved = CodeReject |
| cargo-audit `--ignore` / deny exceptions / vet exemptions / mutants excludes | scanned in config; un-approved = CodeReject |
| dependency tricks | cargo-deny bans + dependency_source_digest |
| macro-expanded code | KNOWN LIMITATION: semgrep/rg blind to proc-macro output. Clippy partially covers (sees post-expansion). Stated honestly. |

---

## 13. Escape Hatch (anti-circular policy)

**NO per-site bypass.** The only escape is editing policy files (§11). Anti-circularity: a policy-PR is checked against the **PREVIOUS main-branch policy** (not the weakened one). A meta-policy requires:
- CODEOWNER approval on any policy file change.
- Explicit diff classification (tightening / loosening / neutral). Loosening flagged for human review.
- The policy-PR passes the gate under the PREVIOUS policy.

---

## 14. Deploy-Gate

The deploy-gate checks ALL of:
1. Attestation present, `scope = Full`.
2. Signature valid under `signing_key_id` (key not revoked).
3. Fresh: `issued_at ≤ now ≤ expires_at`.
4. Evidence matches: recompute `Evidence` digest from actual artifact+source+policy+toolchain+advisory-db+env. Compare. Mismatch → REJECT.
5. Advisory DB current: `advisory_db_digest` matches current pinned snapshot. Changed → REJECT.
6. Artifact-bound: if deploying a binary, `artifact_digest` matches actual binary.
7. If the deploy-gate itself cannot run → REJECT (fail-closed).

Deploy-gate is an EXPLICIT dependency of the deploy target — not skipped by Moon's affected-target logic.

---

## 15. Moon CI/CD Integration

```yaml
gate-edit:
  command: 'xtask gate --scope edit --emit json --out target/xtask/report.json'
  toolchains: [rust]
  options: { runInCI: true }
  inputs:
    - '@globs(sources)'
    - '.xtask/**'
    - 'Cargo.toml'
    - 'Cargo.lock'
    - '**/Cargo.toml'
    - '.cargo/**'
    - 'rustfmt.toml'
    - 'clippy.toml'
    - 'deny.toml'
    - 'supply-chain/**'
    - 'rust-toolchain.toml'
  outputs: ['target/xtask/report.json', 'target/xtask/evidence.json']

gate-full:
  command: 'xtask gate --scope full --emit json --out target/xtask/report.json'
  inputs:
    - '@globs(sources)'
    - '.xtask/**'
    - 'Cargo.toml'
    - 'Cargo.lock'
    - '**/Cargo.toml'
    - '.cargo/**'
    - 'rustfmt.toml'
    - 'clippy.toml'
    - 'deny.toml'
    - 'supply-chain/**'
    - 'rust-toolchain.toml'
    - '.xtask/mutants-baseline.json'
    - '.xtask/advisory-db-snapshot/'

deploy-gate:
  command: 'xtask verify-attestation target/xtask/attestation.json --require-signature --require-scope full'
  options: { runInCI: true }
```

---

## 16. Error Taxonomy

### Report root (disjoint)
`Pass | CodeReject | GateReject | PolicyError | InputError`

### Lane failures (GateReject)
`ToolMissing | ToolCrashed | ToolTimeout | ToolVersionMismatch | ToolPathPoisoned | PanicInGate`

### Rule families
- `HOLZMAN_PANIC_*` — unwrap/expect/panic/todo/indexing/assert-macro
- `HOLZMAN_UNSAFE_*` — unsafe/raw-pointer/transmute/as-conversion
- `HOLZMAN_CHECKED_*` — ignored Result, arithmetic side effects
- `FUNC_NESTING_*` — >2 nesting depth
- `FUNC_LOOPS_*` — imperative for/while/loop
- `FUNC_STYLE_*` — Result<_,String>, wildcard arm, bool flag, unwrap_or family
- `SUPPLY_ADVISORY` — known advisory
- `SUPPLY_LICENSE` — license violation
- `SUPPLY_BANNED_CRATE` — banned crate
- `SUPPLY_VET_GAP` — cargo-vet gap
- `SUPPLY_UNUSED_DEP` — unused dependency
- `SUPPLY_UNSAFE_DEP_THRESHOLD` — unsafe in dep tree above threshold
- `FEATURE_COMBO_FAILED` — broken feature combination
- `TEST_FAILURE` — failing test
- `TEST_NONDETERMINISTIC` — test produces non-deterministic results
- `MUTANT_SURVIVED` — surviving non-baselined mutant
- `MUTANT_BASELINE_EXPIRED` — baselined mutant past expiry
- `MUTANT_BASELINE_UNOWNED` — baselined mutant missing owner
- `BYPASS_*` — allow-attribute / cfg-attr-allow / semgrep-ignore / cargo-ignore / tool-wrapper / source-escape
- `POLICY_*` — malformed policy / circular-policy-change
- `CERT_*` — cert-absent / unsigned / digest-mismatch / expired / scope-mismatch
- `INPUT_*` — invalid input contract
- `GATE_*` — tool-missing / crashed / timeout

### Severity
No decorative severity. Findings either cause CodeReject or are informational. Removed from the model.

---

## 17. Toolchain Requirements

**Hard-required (all scopes):**
`cargo`, `rustc`, `rustfmt`, `clippy`, `rg`, `semgrep`, `cargo-audit`, `cargo-deny`, `cargo-vet`, `cargo-geiger`, `cargo-machete`, `cargo-hack`

**Hard-required for full scope only:**
`cargo-mutants`

**Optional but allowed inside tests (Xtask v1 does not know about them unless invoked through `cargo test`):**
`proptest`, `quickcheck`, `insta`, `rstest`, `loom`, `miri`, `cargo-fuzz`

All versions pinned and bound into `toolchain_digest` (hash of resolved binaries or hermetic OCI digest, not version strings).

---

## 18. Second-Order & Pre-Mortem

**Key-leak mitigation:** "digest recompute" does NOT mitigate a leaked key. Real mitigation: key revocation, KMS/HSM or keyless CI identity (Sigstore), transparency logging (Rekor), branch-protected signing policy, separation from untrusted execution (§9).

**3 AM disaster:** A logical correctness bug that passes all lints/tests/mutations but is semantically wrong. Xtask certifies conformance to a pinned quality policy — it does not prove the algorithm is correct. The certificate records which lanes ran.

**Macro-expanded code blind spot:** semgrep/rg cannot see proc-macro output. Clippy partially covers (post-expansion). Known residual.

**Build-script mutation:** readonly source (§10) + source-digest comparison before/after.

---

## 19. The Honest Trust Boundary

Xtask certifies that code **conforms to a checked-in quality policy**. It is mechanically checked, policy-conformant, evidence-backed, fail-closed, artifact-bound, and deterministic for a pinned input set.

**Xtask does NOT guarantee:**
- Behavioral correctness (the algorithm is right).
- All UB freedom.
- Macro-expanded code quality (semgrep blind; clippy partial).
- Unknown vulnerabilities (cargo-audit checks KNOWN advisories).
- Concurrency soundness.
- That `unwrap_or` is a panic risk (it isn't — it's house style).

The certificate is a **quality floor for a pinned policy**, not a correctness proof.

---

## 20. Component / Module Map

Single Cargo workspace (decomposer refines):
- `xtask-bin` — CLI (clap). Subcommands: `gate`, `verify-attestation`, `doctor`.
- `xtask-core` — domain types: `Report`, `Finding`, `Lane`, `RuleId`, `RepairHint`, `Location`, `LaneOutcome`, `SkipReason`, `LaneFailure`, `LaneEvidence`.
- `xtask-policy` — policy loading, validation, `policy_digest`, meta-policy (CODEOWNER, diff classification).
- `xtask-lanes` — lane runners: `fmt`, `rustc`, `clippy`, `semgrep`, `assert_scan`, `build_rs_scan`, `test`, `supply`, `feature`, `mutants`.
- `xtask-sandbox` — sandbox: network-off, readonly source, frozen PATH, fixed env, cgroup caps.
- `xtask-evidence` — canonical serialization, `Evidence` computation, `LaneEvidence`.
- `xtask-signer` — `Attestation` signing/verification, Ed25519, canonical pre-sign payload, key_id, revocation.
- `xtask-bypass` — bypass-surface scans: allow-attribute, cfg-attr, tool-wrapper, source-escape.
- `xtask-output` — report JSON schema (versioned), `doctor` diagnostics.

All first-party crates pass their own gate (dogfooded).

---

## 21. CLI Surface

```
xtask gate [--input <crate|diff>] [--scope edit|prepush|full] [--emit json] [--out <path>]
    Run scoped lanes. Emit report JSON + exit code. Emit evidence on Pass.

xtask verify-attestation <attestation.json> [--require-signature] [--require-scope full]
    Deploy-gate: recompute digests, verify signature, check freshness.

xtask doctor [--scope <scope>]
    Report required tools for the CURRENT scope/policy. Fail-closed.
```

Exit codes: `0` Pass, `1` CodeReject, `2` GateReject, `3` PolicyError, `4` InputError, `>=5` internal.

---

## 22. Definition of Done — Xtask v1

1. `xtask gate --scope edit` runs fmt, cargo check, clippy, semgrep, and the production panic/assert scan.
2. `xtask gate --scope prepush` adds deterministic tests, supply-chain checks, and feature-powerset compilation.
3. `xtask gate --scope full` adds mutation testing.
4. A passing full gate emits deterministic evidence and, in CI only, a signed attestation.
5. Deploy requires a full-scope CI-signed attestation binding source, artifact, policy, toolchain, dependency source, advisory database, feature profile, and layer evidence digests.
6. Any unrunnable scoped layer forces fail-closed rejection.
7. Code violations, gate failures, policy errors, and input errors are type-disjoint in the output schema.
8. No inline bypass is accepted for gated first-party source; exceptions require policy files and policy PR review.
9. Moon tasks `:gate` and `:deploy-gate` are wired.
10. Xtask's own source passes the full gate.
11. Killer demo: AI writes Rust with a `for` loop and `.unwrap()`; `xtask gate` rejects with typed `FUNC_LOOPS_*` and `HOLZMAN_PANIC_UNWRAP` findings; AI fixes the code; the full CI gate passes; the signed deploy attestation is accepted.

---

## 23. References

- `holzman-rust/SKILL.md` + all 6 references (nasa-jpl-standards, latency-throughput-playbook, runtime-performance-architecture, zero-cost-abstractions, simd-patterns, mechanical-empathy-toolchain)
- `functional-rust/SKILL.md` + all 3 references (scott-ddd-types, typing-refactor-checklist, complete-workflow)
- `moon-v2/SKILL.md` — canonical `moon ci` gate
