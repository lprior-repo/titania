# v1.5 — Kani + Mutants + Full scope — spec

> Companion to `v1-spec.md §16 v1.5`. Authoritative for the v1.5 milestone;
> supersedes the §16 one-paragraph stub with concrete design choices, contracts,
> and acceptance. Status: design lock.

## 0. Scope

- **In**: `GateScope::Full`; `Lane::Kani`; `Lane::Mutants`; `PROOF_KANI_*`
  rule family; `MUTANT_SURVIVED` rule family; Moon task `titania-kani`,
  `titania-mutants`; composite gate `gate-full`; v1.5 doc/run updates.
- **Out**: Kani in Prepush/Release; mutants in Prepush; Kani+Mutants on
  every crate (workspace inventory decides); `Full` in pre-Moon v1.0
  scope strings; non-rust non-cargo proof harnesses; persistent receipt
  ledger; Loom/Shuttle/Ferrocene.

## 1. Decisions Locked

| # | Decision | Choice |
|---|----------|--------|
| D1 | Lane / GateScope enum exhaustiveness | Stay **total**. `Lane::Kani`, `Lane::Mutants`, `GateScope::Full` added. Update every `match` site that exhausts on `Lane` or `GateScope` and every `fn lane_stem` / `fn scope_dir` / `fn stem_to_lane`. Today's count: 9 production files (5 in `titania-lanes`, 1 in `titania-aggregate`, 1 in `titania-check`, `titania-core/src/lane.rs`, `titania-core/src/gate_scope.rs`) plus tests that variant-cover. No `#[non_exhaustive]` loosening. |
| D2 | Scope placement | `Lane::Kani` and `Lane::Mutants` run **only** in `GateScope::Full`. Prepush/Release/Edit unchanged. |
| D3 | Mutants baseline posture | **Zero-survivor baseline under full `cargo mutants` (test-running) mode.** `cargo mutants --check` (build-only) surfaces 236 build-survivors in titania-core today (per `.evidence/v1.5/mutants-titania-core-summary.json`); those are candidate test-survivors. v1.5 bootstrap re-runs them under full test mode (`cargo mutants` with no `--check`) and accepts the surviving test-survivors into `.titania/profiles/strict-ai/mutants.baseline.json` with a `titania-bypass-mutant-<id>` exception. Empty baseline is the goal; non-empty entries require owner/reason/expiry. Lane fails on any new mutation that survives tests. |
| D4 | Kani scaling | Per-package `cargo kani -p <pkg>` enumeration from a workspace harness inventory. `-j 1`, `MemoryMax=24G`, `MemorySwapMax=0` cgroup cap. Failed harness ⇒ `BLOCK_LOCAL` only, not whole-lane reject, so orphan harnesses don't poison the lane. |
| D5 | Resource governance | Mandatory. No full-workspace `cargo kani --workspace` until explicit human waiver. |
| D6 | Proof artifacts | Each Kani/Mutants run emits one `lane outcome` plus a per-finding `PROOF_KANI_<NAME>` or `MUTANT_SURVIVED` typed `Finding`. No `final receipt only` mode. |
| D7 | Baseline location | `.titania/profiles/strict-ai/mutants.baseline.json` — alongside the existing policy profile. |
| D8 | Moon task names | `titania-kani`, `titania-mutants`. Composite gate: `:titania:gate-full` mirrors the existing `:titania:gate-edit` / `:titania:gate-prepush` / `:titania:gate-release` surface. |

## 2. Ubiquitous Language

| Term | Definition |
|------|------------|
| **Kani harness** | A `#[kani::proof]` or `#[kani::proof_for_contract]` function in the titania workspace. Files keep `cfg(kani)` isolation; metadata lives next to the function. |
| **Harness id** | Identical to the function name, normalized to `PROOF_KANI_<NAME>`. The `<NAME>` is the Kani harness name with non-alphanumeric collapsed to `_`. |
| **Mutant** | One mutation cargo-mutants can apply: `== → !=`, `&& → \|\|`, arithmetic flip, default-replace, etc. Each mutant has a stable mutation-id derived from source span + operator + summary. |
| **Surviving mutant** | A mutant that built and passed all target tests without being killed. Lane emits one `MUTANT_SURVIVED` finding per survivor with mutation-id, file:line, and source-text excerpt. |
| **Mutants baseline** | JSON list of `{mutation-id → accepted-by-rule}` entries. Empty file (no survivors) is the goal; non-empty baseline means each entry has an explicit `titania-bypass-mutant-<id>=<owner>/<reason>/<expiry>` policy exception — same shape as v1.0 policy exceptions. |
| **Full gate** | A Moon composite running `titania-release` plus `titania-kani` plus `titania-mutants`. |
| **Kani lane outcome** | `LaneOutcome::clean` iff every harness under the target workspace returned `VERIFICATION:- SUCCESSFUL`. Any `VERIFICATION:- FAILED`, undetermined, unsupported-feature, or OOM/timeout ⇒ `LaneOutcome::findings` with one `PROOF_KANI_FAIL` finding per failed harness. |
| **Mutation lane outcome** | `LaneOutcome::clean` iff zero survivors beyond the baseline. Any new survivor ⇒ `LaneOutcome::findings` with one `MUTANT_SURVIVED` finding per new survivor. |

## 3. Value Objects + Typestates

Adding/strengthening (kept total):
- `Lane`: `+Kani, +Mutants`. `lane::name()`, `Display`, `Lane::from_str`
  extended to recognize `"Kani"`, `"Mutants"`. Artifact stem mapping already
  lives in `crates/titania-check/src/main.rs::lane_stem`; v1.5 adds
  `Lane::Kani => "kani"`, `Lane::Mutants => "mutants"` there so output
  filenames are `.titania/out/full/kani.json` and `.titania/out/full/mutants.json`.
- `KaniHarnessId` — newtype around `String`, `KaniHarnessId::new(name)` validates
  `^[a-zA-Z][a-zA-Z0-9_]*$`; rejects empty, leading digit, non-ASCII.
- `MutantId` — newtype around `String` shaped `<pkg>::<rel-path>:<line>:<col>:<operator>`.
  Stable across runs for the same source mutation; lane uses this in `MUTANT_SURVIVED`
  finding `Location`'s source string.
- `RuleId` is a `String` newtype
  - `PROOF_KANI_PASS`, `PROOF_KANI_FAIL`, `PROOF_KANI_BLOCKED`, `PROOF_KANI_NOT_RUN`,
    `PROOF_KANI_UNSUPPORTED` — one per Kani outcome category. Same prefix
    family shape as `CLIPPY_*`.
  - `MUTANT_SURVIVED` — single rule id covering both individual mutations
    and aggregated lane outcomes (location + repair hint carry the
    per-mutation identity).
  - `MUTANT_BASELINE_MISSING` — emitted when the baseline file does not
    exist on a fresh checkout and the operator has not run the bootstrap
    recipe.
  - `explain` catalog entries for each.
- `GateScope::Full` — added as the fourth variant. `lanes()` returns the Full set.
- `RuleExplanation` — `explain catalog` extended with every new rule id.
  `titania-check explain PROOF_KANI_FAIL` returns the prose entry.

## 4. Workflows

### 4.1 Full gate happy-path

```
moon :titania:gate-full
  → titania:lint-src (unchanged from v1.0)
  → titania-clippy-all (unchanged)
  → :titania:gate-release (unchanged from v1.0)
  → titania-kani  (NEW)
  → titania-mutants (NEW)
```

### 4.2 Kani lane

1. Load `TargetProject` from cwd.
2. Enumerate every `crates/*/Cargo.toml`. For each: `cd crates/<pkg> && cargo kani list --format json > <tmp>.json`.
   (cargo-kani 0.67.0 rejects `--package` and `--output-format` on `cargo kani list`.)
3. Parse the JSON; collect every harness under `standard-harnesses`.
4. Re-run per package with `cargo kani -p <pkg> --output-format=regular -j 1`
   inside `systemd-run --user --scope -p MemoryMax=24G -p MemorySwapMax=0 …`.
5. For each harness line, classify: `VERIFICATION:- SUCCESSFUL` → `PROOF_KANI_PASS`;
   `PROOF_KANI_FAIL`; an unsupported-feature warning → `PROOF_KANI_UNSUPPORTED`;
   timeout/OOM → `PROOF_KANI_BLOCKED` (with cgroup log path); missing cargo-kani →
   `PROOF_KANI_NOT_RUN` and lane disposition `NotApplicable`.
6. Write `.titania/out/full/kani.json` typed artifact (same `LaneOutcome`
   schema as v1.0 lanes).
7. Map `LaneOutcome::Clean` to `LaneExit::Clean`; otherwise
   `LaneExit::Violations`.

### 4.3 Mutants lane

1. Confirm `cargo-mutants --version` present; else `NotApplicable`.
2. Read `.titania/profiles/strict-ai/mutants.baseline.json`. If absent,
   the lane FAILs with `MUTANT_BASELINE_MISSING` and prompts the operator
   to run the bootstrap recipe (§4.4).
3. Run **full test-mode**: `cargo mutants --no-shuffle -o .titania/out/full/mutants.out --json`
   with a cgroup cap (`MemoryMax=24G`, `MemorySwapMax=0`).
   `--check` is forbidden; we want true test-survivors.
4. Read `.titania/out/full/mutants.out/outcomes.json` and `mutants.json`.
   Compare against the baseline entries: any survivor NOT in the baseline
   is a new test-survivor. Emit one `MUTANT_SURVIVED` finding per new survivor.
5. Write `.titania/out/full/mutants.json` typed artifact.
6. **Zero-survivor baseline bootstrap** (§4.4): pre-fix all current survivors
   before first baseline commit. The bootstrap is a documented recipe run by
   the operator once per package; tracked under a contract work item
   `tn-7bq2.4-bootstrap`.

### 4.4 Baseline bootstrap (D3)

Before `gate-full` is wired into CI:

1. Per package, run `cargo mutants --no-shuffle --json --output <run-dir>`
   (no `--check`; full test mode). Each run produces `outcomes.json` (totals)
   and `mutants.json` (per-mutant list) under `<run-dir>`.
2. For every survivor in `mutants.json`:
   - **Killed by adding a test**: write a unit/property test that fails after
     the mutation is applied. Document the test id in the finding's `RepairHint`.
   - **Acceptable by domain rule**: add an exception entry to
     `.titania/profiles/strict-ai/mutants.baseline.json` of the form
     `{ "mutation-id": "<id>", "accepted-by-rule": "mutant-accept/<owner>/<reason>/<expiry>" }`.
     Same shape as v1.0 policy exceptions.
3. After zero survivors remain, commit the baseline.
4. Save the per-package raw outputs and total-mutant counts as
   `.evidence/v1.5/raw/mutants-baseline-<pkg>.{json,txt}` for v1.5 evidence-packaging.

## 5. Functional Core / Imperative Shell Boundaries

- **Pure (core)**: `KaniHarnessId`, `MutantId`, the v1.5 rule-id string literals
  (`PROOF_KANI_*`, `MUTANT_SURVIVED`, `MUTANT_BASELINE_MISSING`), every
  `LaneOutcome` constructor, the JSON harness inventory parser, and the
  survivor-vs-baseline diff. All `titania-core/src/`.
- **Shell (lanes, check)**: spawning cargo-kani / cargo-mutants processes,
  cgroup setup, artifact writing, Moon task invocation. All
  `titania-lanes/src/run_cargo_lane.rs` and `titania-check`.

The Lane dispatch table (`run_lane.rs`) is the boundary: it must adopt the
total-enum + per-variant arm pattern (no `#[non_exhaustive]`); v1.5 adds
`Lane::Kani` and `Lane::Mutants` arms.

## 6. Error Taxonomy

| Variant | Source |
|---------|--------|
| `LaneReport::Kani(cargo_kani_lane::LaneError)` (new) | `crates/titania-lanes/src/run_lane_kani.rs` (new) |
| `LaneReport::Mutants(cargo_mutants_lane::LaneError)` (new) | `crates/titania-lanes/src/run_lane_mutants.rs` (new) |
| `LaneOutcome::RejectKind::KaniFail` (new variant of the existing sum-type tag) | `crates/titania-core/src/outcome.rs` |
| `LaneOutcome::RejectKind::MutantSurvivor` (new variant of the existing sum-type tag) | same |
| `RuleId` validation errors: existing `RuleIdError` covers all v1.5 ids — no new variants. Each id is just a `String` the same as every v1 rule id. | n/a |

Domain rules forbid `String` errors, panic, and `unwrap`/`expect`/etc. across the
new core code. `Result<T, anyhow::Error>` is permitted only at the
`titania-check` boundary where errors leave the process.

## 7. Skip-State Contract (v1 spec §4 extension)

| SkipReason | Lane | Trigger |
|------------|------|---------|
| `ToolUnavailable` | Kani | `cargo-kani` missing or version older than `0.50.0` |
| `ToolUnavailable` | Mutants | `cargo-mutants` missing or version older than `25.0.0` |
| `WorkspaceNotRootedAtCargo` | Kani, Mutants | `TargetProject::rooted_at()` reports a non-crate path |
| `ProfileBaselineMissing` | Mutants | baseline file absent on first invocation (D3) |
| `OutOfMemoryCgroup` | Kani | CBMC exceeds `MemoryMax=24G` |

`NotApplicable` lane disposition stays a valid aggregate outcome; it does not
fail the gate unless the underlying skip is one of `BLOCKED` (OOM, harness
syntax error, etc.).

## 8. Moon Surface

```yaml
# .moon/tasks/all.yml additions
titania-kani:
  command: 'titania-check run-lane Kani --emit json'
  # Cache key: SOURCE + KANI_VERSION; stamp on harness inventory drift.
  inputs: ['crates/**/*.rs', '.titania/out/full/kani.json']

titania-mutants:
  command: 'titania-check run-lane Mutants --emit json'
  inputs: ['.titania/profiles/strict-ai/mutants.baseline.json', 'crates/**/*.rs']
```

`:titania:gate-full` lists `:titania:gate-release` followed by `titania-kani`,
`titania-mutants`. `cargo check` continues to skip these (heavyweight).

## 9. Acceptance

| # | Required claim | How it is verified |
|---|----------------|--------------------|
| A1 | `cargo kani list --format json` returns ≥ v1.5 required harness set | `.evidence/v1.5/kani-harnesses.json` |
| A2 | Each Kani harness exit `VERIFICATION:- SUCCESSFUL` | `.evidence/v1.5/kani-verification-run.log` (cgroup-capped) |
| A3 | Each Kani harness has recorded `#[kani::unwind(N)]` (or per-harness `kani::unwind`) and any cover! assertions are reached | harness-inventory + run-log review |
| A4 | `cargo mutants --check` baseline accepted and gate rejects new survivors | `.evidence/v1.5/mutants-baseline.json`, `.evidence/v1.5/mutants-survived-fixture.json` (a synthetic survivor produced by an experimental mutation, captured as fixture) |
| A5 | `.titania/out/full/kani.json` schema-valid typed `LaneOutcome` | typed-artifact contract test |
| A6 | `.titania/out/full/mutants.json` schema-valid typed `LaneOutcome` | same |
| A7 | `moon :titania:gate-full` exits 0 from a clean workspace | `.evidence/v1.5/gate-full.run.log` |
| A8 | `titania-check --scope full --emit json` exits 0 from a clean workspace | `.evidence/v1.5/check-full.run.log` |
| A9 | `titania-check explain PROOF_KANI_FAIL` and `titania-check explain MUTANT_SURVIVED` return prose | explain-catalog snapshot |
| A10 | No production `unwrap`/`expect`/`panic`/`todo` from cargo clippy strict lane | clippy run log |

## 10. Out of scope (deferred)

- Verus specs on hotpath (deferred to v2.5, where Verus is a first-class lane).
- Flux refinements (v2.0).
- Miri / sanitizers / cargo-fuzz (v2.5).
- Per-feature Per-crate `--unwind` schemas (only `#[kani::unwind]` per harness
  for v1.5; per-package schemas land when v1.5 proves stable).
- Cargo `--target` matrix splits (future Kani lane option; not v1.5).
- Storing Kani run outputs to a durable ledger (post-v3.0).

## 11. Files Touched (preliminary)

- `crates/titania-core/src/lane.rs` — add `Kani`, `Mutants`; total enum.
- `crates/titania-core/src/gate_scope.rs` — add `Full`; total enum.
- `crates/titania-core/src/lane.rs` — add `Kani`, `Mutants`; total enum.
- `crates/titania-core/src/gate_scope.rs` — add `Full`, extend `FULL_LANES` const; total enum.
- `crates/titania-core/src/rule_id.rs` — no code change (rule ids are strings);
  the explain catalog gains entries (see §11).
- `crates/titania-core/src/error.rs` — review `RuleIdError`; new ids use the
  same error path.
- `crates/titania-core/src/kani.rs` — expand harness inventory (D4).
- `crates/titania-core/src/proof_id.rs` (new) — `KaniHarnessId`, `MutantId`.
- `crates/titania-core/src/report/*` and `crates/titania-core/tests/*` —
  match-lane arms updated for `Kani`/`Mutants`.
- `crates/titania-lanes/src/run_lane.rs`, `run_cargo_lane.rs`, `run_cargo/args.rs`,
  `run_lane_outcome.rs`, `artifact_writer.rs`, `ast_grep_lane/{engine,rules}.rs`
  — match-lane arms updated.
- `crates/titania-lanes/src/run_lane_kani.rs` (new) — Kani lane implementation.
- `crates/titania-lanes/src/run_lane_mutants.rs` (new) — Mutants lane implementation.
- `crates/titania-lanes/src/run_cargo_lane.rs` — wire Kani/Mutants into the dispatch table.
- `crates/titania-lanes/Cargo.toml` — no new deps (cargo-kani/cargo-mutants are
  invoked via `CommandIn` from the shell; no library binding).
- `crates/titania-check/src/main.rs` — `lane_stem()` arms + `scope_dir()` arm
  for `GateScope::Full`; `run-lane` accepts `Kani`/`Mutants`.
- `crates/titania-check/src/args.rs` — full scope flag (already exists for the
  three v1 scopes; extend the parser).
- `crates/titania-aggregate/src/{artifact_reader.rs,report_assembly.rs}` and tests —
  match-lane arms updated; LaneReceipt slots for Kani/Mutants.
- `crates/titania-policy/profiles/strict-ai/policy.toml` — exception families.
- `crates/titania-policy/src/explain.rs` — explain-catalog additions.
- `.moon/tasks/all.yml` — `titania-kani`, `titania-mutants`, `gate-full`.
- `docs/` (new) — v1.5 user-facing docs.

## 12. Risks + Residual

- **R1**: CBMC OOM on a single harness crashes the lane. Mitigation: per-package
  enumeration + cgroup; a single harness OOM does not poison the lane.
- **R2**: cargo-mutants `baseline` flag semantics shift between minor versions.
  Mitigation: pin to `cargo-mutants = "27.x"` in the dev-side check.
- **R3**: First-run baseline bootstrap might surface dozens of survivors in
  the v1 codebase, delaying the contract. Mitigation: D3 declares the
  bootstrap a separate contract work item before `gate-full` lands.
- **R4**: Kani `[-Z function-contracts, -Z stubbing]` warnings emit "unsupported
  feature" lines that look like failures. Mitigation: lane classifies
  unsupported-feature into `PROOF_KANI_UNSUPPORTED`, not `PROOF_KANI_FAIL`.
- **R5**: Existing v1 clippy strict lanes reference `Lane::name()` / `Lane::from_str`
  pairs and assume `#[non_exhaustive]` is **off**. Mitigation: D1 keeps the
  enum total; the 9 production match sites get new arms; serializer round-trips pass
  unchanged (no variants affect existing JSON).
- **R6**: cargo-kani 0.67.0 may collide with the workspace `titania-dylint`
  rustc_driver — flagged by the KaniRefresh pre-impl smoke (workspace fallback
  hit the collision). Mitigation: Kani lane runs per-crate (`cd crates/<pkg>`)
  rather than `--workspace`; if a single harness is blocked, lane emits
  `PROOF_KANI_BLOCKED` but does not fail the gate unless every harness blocks.
  The collision also dictates the lane's ordering: Kani before dylint in the
  `gate-full` Moon composite so failures are caught early.

## 13. Pointers

- `v1-spec.md §16` — original (sparse) v1.5 stub.
- `kani` skill (`skill://kani`) — harness discipline.
- `holzman-rust` skill (`skill://holzman-rust`) — strict clippy/lints.
- `moon-v2` skill (`skill://moon-v2`) — Moon CI posture.
- `.evidence/kani-list/titania-core.json` — current 8-harness inventory.
