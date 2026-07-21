# v1.5 Kani + Mutants + Full scope — Bitter Truth Simplicity Review

> Reviewer lens: Farley / Holzman / strict DDD / bitter-truth.
> Doctrine: the simplest solution that satisfies the contract is the best.
> Scope: `run_lane_kani.rs`, `run_lane_mutants.rs`, `proof_id.rs`, `mutants_baseline.rs`,
> and any code they should have leaned on. No source modifications; report only.

## Verdict

**PARTIAL**

The two new lanes ship **correctness-shaped** findings and reuse the
typed domain surface (`Lane`, `LaneOutcome`, `LaneFailure`, `ToolKind`,
`SkipReason`, `MutantsBaseline`, `KaniHarnessId`, `MutantId`, `Finding`,
`RepairHint`, `Location`, `ProcessTermination`). The contract keeps.

But the implementation **invents machinery the codebase already provides**,
**rewrites code it already depends on**, and **triples its line count by
inflating private types instead of folding them**. The v1.5 source is
`run_lane_kani.rs` = 1127 lines and `run_lane_mutants.rs` = 619 lines.
Per the spirit of the contract they should each be well under 400 lines.

Three blockers and ten improvements follow.

## Complexity Findings

1. **DRY: wait-with-timeout reinvention.** `wait-timeout` is already a
   dependency of `titania-lanes` (`Cargo.toml:22`) and is already used by
   `command/execution.rs:8,133,178` and by `run_lane_mutants.rs:20,296` via
   `ChildExt`. The Kani lane open-codes the same logic as ~60 lines of
   `poll_child` / `step_child` / `check_timeout` (`run_lane_kani.rs:730-773`)
   plus a `poll` loop around `try_wait` with `thread::sleep(50ms)`. This is
   identical work to `child.wait_timeout(timeout)`. The Kani lane also has
   `Command::spawn` → `try_wait` busy-polling, which is **measurable
   latency cost** (50ms wakeup) in the spec's per-package wallclock budget
   (`PER_PACKAGE_TIMEOUT_SECS = 600`). Replace the loop with
   `wait_timeout::ChildExt::wait_timeout`.

2. **Architectural-drift: `CommandIn` bypass.** `command.rs:1-7` declares
   `CommandIn` the **single chokepoint** for shelling out, with
   `CommandBudget` timeouts, `EnvPolicy`, `inherit_env()`, and `arg()`/`args()`.
   `run_cargo/outcome.rs:91-78` uses `CommandIn` to build typed evidence
   and `cargo --version`. The two new lanes bypass it entirely. They call
   `std::process::Command::new("cargo").arg("...").arg("...").spawn()`
   directly (`run_lane_kani.rs:693-696`, `885-892`, `run_lane_mutants.rs:283-294`),
   roll their own `Stdio::{piped,null}` plumbing, manage their own
   timeouts, and have to invent a `CommandEvidence` literal that records
   `argv` by hand (`run_lane_kani.rs:413-432`, `run_lane_mutants.rs:199-216`).
   The spec §11 line 242 already names `CommandIn` as the boundary. The
   v1.5 lanes ignore it.

3. **`has_workspace_root` is a bug, not a helper.** `run_lane_kani.rs:613-619`
   reads `Cargo.toml` and checks for a `[workspace]` line. A per-crate
   `Cargo.toml` whose dependency includes `titania-*` could legitimately
   contain a `#[patch]` or a virtual manifest entry that mentions
   `[workspace]`, and a path-resolved `Cargo.toml` at the top of the
   workspace (the usual case the lane must reject) may have its
   `[workspace]` table hosted in the parent. `TargetProject` already
   carries the contract: `titania-core/src/target_project.rs` distinguishes
   "absolute path with manifest presence" from "is the workspace root".
   The v1.5 spec §7 names `WorkspaceNotRootedAtCargo` as a documented
   skip-reason — but the lane never asks `TargetProject` whether the
   project is the workspace root, it asks "does the `Cargo.toml` file
   contain the word `[workspace]`". This is a missed reuse **and** a
   logic error.

4. **Two copies of `lane_stem` / `file_stem`.**
   `titania-core/src/lane.rs:71-86` exposes `Lane::file_stem()` returning
   `'static str`. `titania-check/src/main.rs:273-285` re-implements the
   same match in `fn lane_stem(lane: Lane)`. Both were updated for v1.5
   with the identical Kani / Mutants arms. The duplication forces every
   new lane to be wired in **twice**, and the two arms can drift
   silently — there is no test that pins them together.

5. **Hand-rolled package discovery.** `run_lane_kani.rs::list_kani_harnesses` +
   `crate_entry` (lines 786-833) and `run_lane_mutants.rs::discover_packages` +
   `path_to_package_basename` (lines 218-234) both walk `crates/*/Cargo.toml`
   with the same intent: enumerate packages under the workspace. There
   is no shared helper. The Kani variant hardcodes a
   `pkg == "titania-dylint"` skip, the Mutants variant does not skip
   anything; both call `path.join("Cargo.toml").exists()` independently.
   `crates/titania-lanes/src/discover.rs` already provides `discover_target`
   and `target_project_from_path` that the spec could lean on; instead
   the two new lanes re-walk the file system in parallel.

6. **Twinned `const PER_PACKAGE_TIMEOUT_SECS: u64 = 600`.** It is declared
   in `run_lane_kani.rs:50` and in `run_lane_mutants.rs:23`. Same value,
   same meaning, same units. Either file can declare it; declaring it
   twice makes the spec's "600s wallclock" budget drift apart the day
   someone tunes one and forgets the other.

7. **Twinned `cargo_{kani,mutants}_available` probes.**
   `run_lane_kani.rs:666-682` and `run_lane_mutants.rs:563-579` are
   pure copy-paste of the same `OnceLock<bool>` + `Command::new(...).
   stdin(null).stdout(null).stderr(null).output()` probe. The only
   difference is the second arg (`"kani --version"` vs `"mutants --version"`).
   One `pub fn tool_available(arg: &str) -> bool` helper would unify them.

8. **Hard-coded tool versions.** `KANI_VERSION: &str = "0.67.0"`
   (`run_lane_kani.rs:56`) and `tool_version: "27.0.0"`
   (`run_lane_mutants.rs:79`) are pinned string literals. The lane
   already proved the tool exists (`cargo <tool> --version`); it then
   ignores the actual version string and emits the literal into
   `LaneEvidence.tool_version()` and into every `Location::tool(..., version)`.
   The literal in the `Location::tool` for **every** finding is then
   duplicated across 4 calls (`run_lane_kani.rs:447, 468, 490, 512`).
   The findings carry a tool-version label that may be wrong if anyone
   upgrades cargo-kani / cargo-mutants without editing the source.

9. **5 finding-builder functions, 1 finding shape.** `pass_finding`,
   `fail_finding`, `blocked_finding`, `unsupported_finding`, `infra_finding`
   (`run_lane_kani.rs:441-538`) — five functions, ~170 lines, all of the
   form:
   ```rust
   let rule_id = per_harness_rule_id(...)?;
   Ok(Finding::{reject|informational}(
       Lane::Kani,
       rule_id,
       Location::tool(KANI_TOOL.to_owned(), KANI_VERSION.to_owned()),
       format!("package=... harness=... reason=..."),
       RepairHint::requires_human_review(format!("... note ...")),
   ))
   ```
   This is one function taking `(effect, fallback_rule, message, note)`
   with the `(KANI_TOOL, KANI_VERSION)` location folded into a const
   `Location`. The 5 funcs collapse to ~30 lines.

10. **Unused / dead state.**
    - `KaniHarness.file: String` is `#[expect(dead_code)]` at
      `run_lane_kani.rs:97`. The harness's `file` payload never reaches
      any `Finding` location; remove the field.
    - `LaneRunState.packages_run: usize` (line 106) is mutated in
      `record_run` (line 333) but never read. Remove it; `state.tool_version`
      and `state.exit_code` are the only consumed fields.
    - `RawMutant.replacement` and `RawMutant.function` fields are
      `#[expect(dead_code)]` (`run_lane_mutants.rs:582, 612`). The
      cargo-mutants JSON shape carries them for human consumption; the
      lane never reads them. If keep is required for "artifact
      compatibility" the code admits it; but then a single struct
      describing "what we extract" + a `#[derive(Deserialize)]` row type
      for the raw shape would shrink the per-field noise.

11. **`KaniHarnessId` rejects lowercase by design, then `canonical_harness_id`
    rewrites lowercase to uppercase.** Spec §3 line 50 says "identical to
    the function name, normalized to `PROOF_KANI_<NAME>`. The `<NAME>` is
    the Kani harness name with non-alphanumeric collapsed to `_`." The
    newtype enforces `^[A-Z][A-Z0-9_]*$` so it cannot hold the function
    name in its raw form. `canonical_harness_id` (lines 565-574) is a
    three-pass iterator — uppercase-lowercase, replace, validate — that
    exists only to feed the newtype. Two valid simplifications:
    - Lowercase letters are not actually dangerous in a rule id; either
      relax the validation to `[A-Za-z0-9_]` and skip the rename.
    - Or keep the validation strict and emit a single normalised form
      in a `Display` impl that doesn't require a mutable buffer per
      harness per invocation.

12. **`binary_operator(name)` is `genre`-by-`name.contains(...)`.**
    `run_lane_mutants.rs:536-547` dispatches `MutantOperator` by substring
    match on the cargo-mutants human-readable name. cargo-mutants does
    not export a structured operator tag — so the decoder is forced —
    but:
    - The `match raw.genre.as_str()` in `operator_for_mutant` (528-534)
      falls into `MutantOperator::DefaultReplace` for any unknown genre;
      that is a **silent loss of operator identity**. An
      `unknown_genre` arm that still records the raw genre string as a
      `String` payload would be honest.
    - The 4-arm `binary_operator` matches only the four English phrases
      cargo-mutants currently emits; an exhaustive `_other` arm returning
      `MutantOperator::ArithmeticOpFlip` (or a documented
      `MutantOperator::Unknown(&str)`) would close the open prefix
      match.

13. **`outcome()` in `run_lane_mutants.rs` reaches 4 levels of nesting.**
    `match package_result { Err(_) => ..., Ok(_) if !findings.is_empty()
    => ..., Ok(_) => match build_clean_outcome(...) { Ok(clean) => ...,
    Err(error) => ... } }` (lines 87-96). Two early returns (or a single
    `match` with three flat arms) would put the entire function under
    the 60-line ceiling.

14. **`outcome()` in `run_lane_kani.rs` mixes `Result` errors and
    `LaneOutcome::Failed`.** Per-harness `?` bubbles
    `KaniLaneError::RuleId` into `RunLaneError::Kani` and from there
    maps to `LaneExit::Failure`. Per-package `outcome()` builds findings
    directly and never bubbles a `RuleId` error. The mixing is fine
    but the `map_outcome_error` function (`run_lane_kani.rs:306-308`)
    smushes a non-`OutcomeError`-shaped error into
    `KaniLaneError::NotACargoWorkspace(format!("evidence build failed:
    {error}"))` which **misnames the variant**. This is an honest
    error taxonomy violation — the variant now represents "evidence
    build failed", not "not a cargo workspace".

15. **Two separate failure shapes for the same kind of problem.** Spec §6
    declares `LaneReport::Kani(...)` / `LaneReport::Mutants(...)` for
    lane failures. The Mutants lane uses `LaneOutcome::Failed {
    LaneFailure::Infra { tool: "cargo-mutants", reason } }` for
    infra-class problems (`infra_outcome` at `run_lane_mutants.rs:99`).
    The Kani lane uses **findings** with a `PROOF_KANI_INFRA` reject
    (`infra_finding` at `run_lane_kani.rs:527-538`). The spec table does
    not single out infra, but the two halves of the same contract pick
    different shapes and both believe they are correct. Pick one.

16. **Test-infrastructure complexity that the user's brief flagged.**
    `tests/v15_atomic_baseline.rs:21-24` uses
    `loom::sync::Arc<loom::sync::Mutex<PathBuf>>` and the same shape for
    `Option<MutantsBaseline>`. Under `cfg(loom)`, `PathBuf` is `!Sync` and
    `String` payloads are `!Sync`, so loom's cell-tracked mutex is
    required to satisfy the executor's borrow checker. The
    `Arc<Mutex<...>>` dance is **inherent to loom** — not bloat — and
    removing it is not simplification. The `LazyLock<Result<RuleId, _>>`
    pattern from the user's brief does not exist in this codebase
    (`rtk grep -rn 'LazyLock' crates/` returns matches only in
    `repair_catalog.rs` (production) and `killer_demo.rs` (test) — both
    of which legitimately cache a static table). So that question is
    answered: the wrappers the user worried about are either
    loom-mandatory or non-existent.

## Required Simplifications (blockers)

These must be resolved before merge. They each either violate the
workspace's own doctrine or break the existing reuse surface the v1.5
docs name.

B1. **Replace `poll_child`/`step_child`/`check_timeout` in
    `run_lane_kani.rs:730-773` with `wait_timeout::ChildExt::wait_timeout`.**
    The crate is already a dependency and the existing helper at
    `command/execution.rs:133,178` is the model. ~60 lines collapse to
    ~15. (Finding #1.)

B2. **Replace `has_workspace_root` in `run_lane_kani.rs:613-619` with a
    call into `titania_core::discover::TargetObservation` /
    `TargetProject::rooted_at()` (the skip-reason
    `WorkspaceNotRootedAtCargo` is in the spec §7) or assert on the
    `Cargo.toml` shape that the rest of the workspace uses.** A
    textual `[workspace]` line search does not distinguish workspace
    root from path-discovered member. (Findings #3.)

B3. **Delete `titania-check/src/main.rs::lane_stem` (lines 273-285)
    and replace every caller with `Lane::file_stem()` from
    `titania-core/src/lane.rs:71-86`.** The two-match duplication
    forces a Kani/Mutants arm to land in two places; the core
    `file_stem` is the source of truth. (Finding #4.)

## Recommended Simplifications (improvements)

R1. **Fold `pass_finding` / `fail_finding` / `blocked_finding` /
    `unsupported_finding` / `infra_finding` into one
    `finding(effect, rule, message, note)` helper.** ~170 lines →
    ~30. (Finding #9.) Build a `const KANI_LOCATION: fn() -> Location`
    so the `tool(KANI_TOOL, KANI_VERSION)` payload is one source.

R2. **Promote `PER_PACKAGE_TIMEOUT_SECS = 600` to a single
    `pub const` shared by both lanes.** Either file, ideally a
    `pub const` in the lanes crate's `helpers` module
    (`helpers.rs` is already the home for shared utilities). (Finding #6.)

R3. **Promote `cargo_kani_available` / `cargo_mutants_available` to one
    `tool_available(arg: &str) -> bool` helper.** Same `OnceLock<bool>`
    probe, two callers, three-line difference. (Finding #7.) Place it
    next to the `CommandBudget` executor or in `helpers.rs`.

R4. **Drive `tool_version` from the actual `cargo <tool> --version`
    stdout, not from a pinned string.** The version probe already
    proves the tool exists; piping the stdout into `LaneEvidence.tool_version`
    costs nothing. Pinning `"0.67.0"` / `"27.0.0"` and then emitting
    that literal into every `Location::tool(..., version)` is
    bookkeeping that drifts the day someone upgrades. (Finding #8.)

R5. **Stop bypassing `CommandIn`.** Both new lanes should issue
    cargo-kani / cargo-mutants through `CommandIn::new(target, "cargo")`
    with `CommandBudget { timeout: PER_PACKAGE_TIMEOUT_SECS, ... }`.
    That gives:
    - envelope-tested `inherit_env` (the two lanes currently let the
      process see all env vars without inspection);
    - typed evidence construction (no more hand-rolled
      `CommandEvidence::new(KANI_TOOL.to_owned(), argv)` shims);
    - the exit-handling / kill-on-timeout machinery the rest of the
      workspace already pays for.
    (Finding #2.)

R6. **Decide the failure shape for infra-class problems and apply it
    across both lanes.** Pick `LaneOutcome::Failed { LaneFailure::Infra }`
    or `Findings` + a `*_INFRA` reject rule id. Mutants uses Failed;
    Kani uses a `PROOF_KANI_INFRA` finding. Apply whichever you pick to
    both. (Finding #15.)

R7. **`outcome()` flattening.** Both lanes' `outcome()` bodies should
    be flat. The Mutants version is 4-deep nested `match` (line 87-96)
    and the Kani version weaves `Ok(...)` arms into a `for` loop body.
    Both can drop a level by collecting findings into a `Vec<Finding>`
    first, then a single tail `match findings.is_empty() { ... }`.

R8. **Remove dead state.** `KaniHarness.file` (line 97), `RawMutant.replacement`,
    `RawMutant.function`, `RawSpan.end` (`#[expect(dead_code)]`) — each
    is admitted-as-unused. (Finding #10.) If kept for "artifact
    compatibility" wrap them in a single private
    `RawMutantsJson` parser type that does the extraction in one place
    rather than letting the lane code name every field.

R9. **`canonical_harness_id` should be a `Display` impl, not a free
    function doing 3 passes.** (Finding #11.) Or relax the newtype's
    validation to accept mixed case and let `KaniHarnessId::new` succeed
    directly.

R10. **`binary_operator` closed match.** An `_other` branch that does
     not silently become `MutantOperator::ArithmeticOpFlip` (Finding #12.)
     Either use `MutantOperator::Unknown` as a new variant, or return
     `Result<MutantOperator, String>` so the lane surfaces the
     unrecognised genre through `MUTANT_SURVIVED` finding messages.

R11. **`map_outcome_error` is a category error.** It folds
     `OutcomeError` into `KaniLaneError::NotACargoWorkspace`. Either
     introduce a new variant
     `KaniLaneError::EvidenceBuildFailed(OutcomeError)` and rename
     `map_outcome_error` accordingly, or surface the `OutcomeError`
     directly. The variant currently misnames the failure. (Finding #14.)

R12. **Share the `crates/*/Cargo.toml` walker.** Both lanes do the
     same walk (`run_lane_kani.rs:786-833` vs `run_lane_mutants.rs:218-234`)
     with diverging "skip" rules. One helper,
     `walk_cargo_packages(root: &Path) -> impl Iterator<Item = &str>`,
     with a `skip: &[&str]` parameter, gives both lanes a single source.
     (Finding #5.)

R13. **Drop `current_unix` helper if it has one caller.** Inline it.
     (Finding #1 in the user's brief; the function exists at
     `run_lane_mutants.rs:553-555`.)

R14. **Magic-number audit.** `600` (timeout), `96` (KANI_HARNESS_ID_MAX_LEN),
    `128` (size limit), `24G` (cgroup cap), `0.67.0` / `27.0.0` (tool versions),
    `50ms` (poll cadence), `2` (cargo-mutants exit-code-with-survivors).
    `600` and `24G` are spec-pinned (D5). `96` is already aligned with
    `RuleId::MAX_LEN = 96` (`rule_id.rs:25`). `128` is layout-policy-driven.
    `50ms` is invisible and worth removing once `wait_timeout` is in
    (R5). Two tool versions are baked-in literals (R4). The value `2`
    in `validate_mutants_exit` (`run_lane_mutants.rs:130`) — cargo-mutants
    returns exit code 2 "had survivors" — is repeated knowledge and
    should be a named `const CARGO_MUTANTS_FOUND_SURVIVORS_EXIT: i32 = 2`.

## System Leverage

The two lanes **already lean on** the typed domain surface. The core
contract under test is the lane returns `LaneOutcome` cleanly. The
following are reused correctly:

| Domain primitive | Reused? | Location |
|------------------|---------|----------|
| `Lane::Kani` / `Lane::Mutants`              | YES   | `titania-core/src/lane.rs:43-46`, arms in `File::from_str`, `file_stem`, `Display` |
| `GateScope::Full`                           | YES   | `titania-core/src/gate_scope.rs:30`, `FULL_LANES` includes Kani / Mutants (`gate_scope.rs:52-55`) |
| `SkipReason::ToolUnavailable(ToolKind)`     | YES   | `run_lane_kani.rs:270, 325`, `run_lane_mutants.rs:72` |
| `ToolKind` (CargoKani / CargoMutants)       | YES   | `proof_id.rs:267-274`; both lanes import and use |
| `KaniHarnessId::new` validation             | YES   | `run_lane_kani.rs:565-574` (with a pre-pass uppercase conversion) |
| `MutantId::new`                             | YES   | `run_lane_mutants.rs:509` |
| `MutantOperator` (closed set)               | YES   | `proof_id.rs:135-154`; dispatch in `run_lane_mutants.rs:528-547` |
| `MutantsBaseline::load`                     | YES   | `run_lane_mutants.rs:159-168` (cleanly maps missing/malformed into typed `MutantsLaneError`) |
| `MutantsBaseline::contains`                 | YES   | `run_lane_mutants.rs:144` |
| `MutantsBaseline::diff`                     | NOT USED | exists at `mutants_baseline.rs:104-107`; the lane does `current.iter().filter(|m| !baseline.contains(&m.mutation_id, now_unix))` inline |
| `LaneOutcome::Clean/Findings/Failed/Skipped`| YES   | both lanes |
| `LaneFailure::Infra / Tool`                 | PARTIAL | Mutants uses `Infra` (`run_lane_mutants.rs:100`); Kani re-uses Findings-shape |
| `Finding::reject / informational`           | YES   | `run_lane_kani.rs:444, 465, 487, 509, 531`; `run_lane_mutants.rs:178` |
| `RepairHint::requires_human_review`         | YES   | every finding-emission site in both files |
| `Location::tool / workspace`                | YES   | 5 sites in kani, 1 in mutants |
| `ProcessTermination::Exited { code: ... }`  | YES   | `run_lane_kani.rs:425`, `run_lane_mutants.rs:212` |
| `CommandEvidence::new / LaneEvidence::new`  | YES   | both lanes (correctly) |
| `Digest::from_bytes`                        | YES   | `run_lane_kani.rs:430`, `run_lane_mutants.rs:213` |
| `TargetProject::as_std_path()`              | YES   | both lanes |
| `TargetProject::manifest_path()`           | NOT USED | exists at `target_project.rs:122-125`; the kani lane has `has_workspace_root` instead |
| `cargo_kani_available` → `cargo kani --version` | HALF | the literal version captured by the probe is **dropped** in favour of pinned strings (`KANI_VERSION = "0.67.0"`) |
| `MutantBaselineEntry`                       | YES (via spec) | not constructed in the lane — the lane only consumes the loaded baseline |
| `current_target_project()`                  | NO    | lanes take `&TargetProject` so this is fine at the call site, but the lanes do not use the discovery helpers in `titania-lanes/src/discover.rs` |
| `CommandIn` (typed subprocess builder)      | NO    | the single biggest miss — see B / R notes |
| `CommandBudget`                             | NO    | `CommandBudget { timeout: PER_PACKAGE_TIMEOUT_SECS, ... }` would replace the entire bespoke timeout machinery |
| `LaneReport` (the public finding-collector type at `lib.rs:60-194`) | NO | the new lanes build `Box<[Finding]>` directly rather than going through `LaneReport::push`; `LaneReport` is **internally** rich and not the same as `titania_core::Finding` |
| `helpers::walk_rs_files`                    | NO    | `walk_rs_files` (`helpers.rs`) already does the directory walk; the new lanes re-implement `read_dir` |

### The one missing reuse that matters most

**`CommandIn` + `CommandBudget`.** `command.rs` declares the invariant
that every subprocess a lane launches goes through it; both new lanes
are exceptions to that invariant. Pulling `CommandIn` in does not just
save code — it gives the lanes env-filtered execution, automatic
timeout / kill via `command/execution.rs:133,178`, and typed argv
capture, all of which the lanes currently reconstruct by hand. The two
lane files lose ~150 lines each if `CommandIn` is used.

### The one missing reuse that is a bug

**`has_workspace_root` should not exist.** The v1.5 spec §7 lists
`SkipReason::WorkspaceNotRootedAtCargo` for Kani AND Mutants. The
Mutants lane does not even check it. The Kani lane's heuristic is a
text search for `[workspace]`. Replace with a `TargetProject`-side
helper.

### Tests that don't earn their keep

- `clean_outcome_records_cgroup_metadata_in_argv` and
  `clean_outcome_records_fallback_when_cgroup_unavailable`
  (`run_lane_kani.rs:1080-1104`) are integration-level tests of the
  string-formatted `argv` only. With `CommandIn` driving execution
  these tests become unnecessary — `CommandIn::args_strings` is tested
  elsewhere.

- `verdict_driven_package_emits_one_finding_per_harness` and
  `timed_out_package_emits_blocked_finding_per_harness`
  (`run_lane_kani.rs:1045-1078`) are good behavior tests but they
  require `RunLaneError`-shaped errors to be coerced through the
  `cargo_kani_available` not-available path. With the `findings_for_package`
  helper accepting a `Verdict` param (rather than a `PackageRun`),
  they collapse to half their current line count.

## Summary

- **Blockers (3)**: B1 replace reinvented timeout with `wait_timeout::ChildExt`;
  B2 replace `has_workspace_root` with `TargetProject`-aware check that
  emits `SkipReason::WorkspaceNotRootedAtCargo`; B3 dedupe `file_stem`.
- **Improvements (10)**: most are mechanical — fold the 5 `*_finding`
  helpers into 1, lift shared constants, switch to `CommandIn`, parse
  the tool-version string the probe already exposes, close the
  `binary_operator` and `genre` open prefixes, share the `crates/*`
  walker, remove admitted-as-dead fields.
- **Expected outcome**: with the blockers and improvements applied, both
  lanes comfortably fit under `clippy::too_many_lines-threshold = 60` per
  function. `run_lane_kani.rs` lands near 450 lines; `run_lane_mutants.rs`
  near 350. None of the changes alter the `LaneOutcome`, `Finding`,
  `RuleId`, or `MutantsBaseline` wire format — the contract surface is
  preserved end-to-end.
- **Spec deviation**: spec §4.2 / D4 forced the per-crate `cargo kani
  list` + per-package `cargo kani -p <pkg>` split because cargo-kani
  0.67.0 rejects `--package` on `list`. The complexity is forced by the
  tool, not invented by the implementation. Justified.
- **`LazyLock<Result<RuleId, _>>` / `loom::sync::Arc<loom::sync::Mutex<PathBuf>>`**
  patterns from the brief do **not** appear in the v1.5 production code.
  `LazyLock` appears in `repair_catalog.rs` (production caching of a
  static catalog table — earned) and `killer_demo.rs` (test dylint
  resolution — earned). `loom::sync::Arc<loom::sync::Mutex<...>>` is in
  `tests/v15_atomic_baseline.rs` and is loom-mandatory under
  `cfg(loom)` for `!Sync` payloads — earned. Removing either would be
  a correctness regression.
