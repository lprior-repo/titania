# Holzman Rust Review — v1.5 Kani + Mutants + Full scope migration

> Scope: `crates/titania-core/src/proof_id.rs`, `crates/titania-core/src/mutants_baseline.rs`,
> `crates/titania-lanes/src/run_lane_kani.rs`, `crates/titania-lanes/src/run_lane_mutants.rs`
> against NASA/JPL Power-of-Ten, the project's `clippy.toml` and `[workspace.lints]`,
> and the v1.5 spec at `.evidence/v1.5/spec.md`.
>
> Authored: read-only review — no source modified, no commit, no push.

## Reference files read

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md` (OpenCode bridge)
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md` (canonical doctrine)
- `/home/lewis/src/titania/AGENTS.md`
- `/home/lewis/src/titania/clippy.toml`
- `/home/lewis/src/titania/Cargo.toml` (workspace `[workspace.lints]`)
- `/home/lewis/src/titania/crates/titania-core/Cargo.toml`
- `/home/lewis/src/titania/crates/titania-lanes/Cargo.toml`
- `/home/lewis/src/titania/crates/titania-core/src/error.rs`
- `/home/lewis/src/titania/crates/titania-core/src/outcome.rs`
- `/home/lewis/src/titania/crates/titania-core/src/rule_id.rs`
- `/home/lewis/src/titania/crates/titania-core/src/lane.rs`
- `/home/lewis/src/titania/crates/titania-core/src/failure.rs`
- `/home/lewis/src/titania/crates/titania-core/src/finding/mod.rs`
- `/home/lewis/src/titania/crates/titania-core/src/finding/location.rs`
- `/home/lewis/src/titania/crates/titania-lanes/src/run_lane.rs` (parent dispatcher)
- `/home/lewis/src/titania/.evidence/v1.5/spec.md` (full)

## Verdict

**PARTIAL FAIL** — the migration contains **9 BLOCK_LOCAL** findings plus a
**critical correctness defect** (mutants lane runs `--list` mode but treats the
resulting enumeration as test-survivors). All four files are free of panic
paths, `unsafe`, unchecked indexing, and `Result`-swallowing in the obvious
spots — that part of the work is sound. But the new lane code deviates from
the v1.5 spec in **three correctness-critical ways**, hides several parse
errors, and leaves the cgroup/resource governance entirely to a layer that
this lane never invokes.

## Findings

Severity legend: **BLOCK_LOCAL** must be repaired before this lane ships; **WARN** is a Holzman-doctrine improvement that does not block; **INFO** documents a non-issue or a clean intent.

### F-01 — CRITICAL — Mutants lane reports every discovered mutation as a survivor
**Severity:** BLOCK_LOCAL (correctness, not a Holzman rule per se — but it inverts the meaning of every downstream finding).

`run_lane_mutants.rs:378` invokes `cargo mutants --list --json --no-shuffle -p <package>`.
`cargo mutants --list` enumerates *every* mutation cargo-mutants can imagine —
it does not apply them and does not run tests. The implementation then feeds
that enumeration into `new_survivors` (`run_lane_mutants.rs:172`) and emits a
`MUTANT_SURVIVED` finding per id that is not already in the baseline.

The spec (`.evidence/v1.5/spec.md` §4.3 steps 3–4) explicitly requires the
**full test-mode** invocation:
`cargo mutants --no-shuffle -o .titania/out/full/mutants.out --json`, then
parse `outcomes.json` (totals) + `mutants.json` (per-mutant list) and extract
the test-survivors.

Net effect: in any real workspace, every cargo-mutants-discovered mutation
becomes a `MUTANT_SURVIVED` reject, the gate fails closed on first run, and
the baseline diff degenerates into a discovery-vs-discovery comparison. The
zero-survivor baseline (D3) cannot be reached without entering every
discovered mutation into the baseline.

### F-02 — Critical — Mutants lane per-package invocation does not match spec; per-harness invocation pattern in Kani lane does not match spec
**Severity:** BLOCK_LOCAL.

**Mutants:** spec §4.3 step 3 expects a single workspace-level run via
`cargo mutants -o .titania/out/full/mutants.out --json` with cgroup scope.
Implementation runs `cargo mutants --list --json --no-shuffle -p <pkg>` per
package from the workspace root (`run_lane_mutants.rs:375-383`). No
`-o <dir>`, no cgroup, no `--no-shuffle` honored by the spec wording (the
flag is present, but the spec's cgroup is not). Combined with F-01 this is
the full spec deviation.

**Kani:** spec §4.2 step 4 says *"Re-run per package with `cargo kani -p
<pkg> --output-format=regular -j 1` inside `systemd-run --user --scope …`."*
One cargo-kani invocation per package, then parse the per-harness lines from
its combined output. Implementation (`run_lane_kani.rs:635-664,
`spawn_kani_child`) runs **one cargo-kani invocation per harness** with
`-p <pkg> --harness <name>`. This:
1. Pays per-harness build cost N times instead of once.
2. Loses the cgroup scope that the spec mandates.
3. Makes the per-harness timeout the only thing protecting the lane from a
   run-away CBMC job.

### F-03 — Critical — Kani lane has no cgroup enforcement anywhere in Rust
**Severity:** BLOCK_LOCAL.

Spec §4.2 step 4 and Risk R1/R6 both call for `systemd-run --user --scope
-p MemoryMax=24G -p MemorySwapMax=0 …`. The Rust lane spawns cargo-kani
directly via `Command::new("cargo")` (`run_lane_kani.rs:652`) with no
`pre_exec`, no `systemd-run`, and no validation that the runtime is cgroup-
capped. The repair hint (`run_lane_kani.rs:377`) *mentions* cgroup OOM,
but the lane itself neither enforces nor detects one — the cgroup is
expected to SIGKILL the child, but there is no parsing for the resulting
non-zero exit or signal.

Same gap for the mutants lane: `run_lane_mutants.rs:375-383` invokes
`cargo mutants` with no cgroup. Spec R2/R3 explicitly require
`MemoryMax=24G, MemorySwapMax=0` for the test-mode run.

If cgroup enforcement is delegated to the Moon task layer, that needs to be
documented in the lane doc-comment; otherwise it is a hidden coupling
between `moon-v2` and these lanes that the spec does not mention.

### F-04 — Important — Per-harness timeout is a runtime cap, not a Power-of-Ten Rule-2 static bound
**Severity:** WARN (Holzman Rule 2 hardening), but rising to BLOCK_LOCAL if
the lane ever runs in a safety-critical or moon-as-boundary mode.

`run_lane_kani.rs:667-674` `poll_kani_child` uses an unconditional
`loop { match ...; if !done { thread::sleep(50ms) } }`. The loop is
bounded only by the runtime constant `PER_HARNESS_TIMEOUT_SECS = 60`
(`run_lane_kani.rs:30`) — i.e., maximum 1,200 iterations at the configured
50 ms poll interval.

Power-of-Ten Rule 2 (per the doctrine) requires a *static* upper bound or
mathematical termination proof; runtime timeouts are "service containment,
not Rule 2 satisfaction". The constant is `pub const`, so a static proof is
feasible — express it as
`for _ in 0..(PER_HARNESS_TIMEOUT_SECS * 1000 / POLL_INTERVAL_MS)` with a
named `POLL_INTERVAL_MS = 50` and a doc comment citing the proof.

Same concern applies to `gather_findings` (`run_lane_kani.rs:145-157`) —
its iteration count is bounded by `inventory.len()` which is bounded by
the workspace's `#[kani::proof]` count. No static cap on that count either.
Recommendation: assert a hard `MAX_HARNESSES_PER_WORKSPACE` at inventory
load time.

### F-05 — Important — Baseline diff is O(survivors × baseline), not O(1) per entry
**Severity:** WARN (perf + Rule 4 contract).

`mutants_baseline.rs:94-96` `MutantsBaseline::contains` does `entries.iter()
.any(...)` — a linear scan. `MutantsBaseline::diff` (lines 103-105) calls
`contains` once per survivor, making the diff `O(s × n)`.

The mutants lane (`run_lane_mutants.rs:178-183`) reaches into the baseline
struct directly via `baseline.entries.iter().any(|e| e.matches(id, now_unix))`
inside the `new_survivors` filter — same asymptotic shape, bypasses the
newtype API entirely.

For the v1.5 numbers in `.evidence/v1.5/mutants-titania-core-summary.json`
(236 build-survivors in titania-core alone, ~thousands of discovered
mutations per package), the wallclock impact is small but grows linearly
with the baseline. Convert `entries: Vec<BaselineEntry>` into a
`HashMap<String, BaselineEntry>` (or carry a `HashSet<String>` alongside)
at load time so `contains` is O(1). Cap baseline size at load.

### F-06 — Important — `drop(...)` swallows two fallible results (Rule 7)
**Severity:** BLOCK_LOCAL.

- `run_lane_kani.rs:209`: `drop(pipe.read_to_string(&mut buf));` — `read_to_string`
  returns `io::Result<usize>`. The `drop` discards both the byte count and
  the error. The doc comment on `drain_pipe` (line 203) says *"ignoring I/O
  errors"* — that's the intent, but the policy in `AGENTS.md` rule 4
  forbids silently swallowed errors. The right fix: cap reads at e.g.
  `MAX_PIPE_BYTES` (say 256 KiB) and `match` the truncated-read outcome;
  never silently drop.
- `run_lane_kani.rs:571`: `drop(std::fs::remove_file(&artifact));` — file
  may not exist (this is the cleanup path before re-running `cargo kani
  list`). The intent is clearly "ignore `NotFound`". The right fix:
  `if let Err(error) = std::fs::remove_file(&artifact) { if error.kind()
  != std::io::ErrorKind::NotFound { return Err(format!("remove_file:
  {error}")); } }` — propagate non-NotFound errors as typed failures.

### F-07 — Important — JSON parse errors are silently downgraded to empty results
**Severity:** BLOCK_LOCAL (Rule 7 + spec deviation).

- `run_lane_kani.rs:604-606`: `let Ok(file) = serde_json::from_str::<KaniListFile>(contents) else { return Vec::new(); };` —
  if the on-disk `kani-list.json` is malformed, the crate contributes zero
  harnesses to the inventory. The caller then sees "no harnesses here" and
  the crate passes silently. This contradicts the spec §4.2 step 5 and
  means a corrupted artifact is not detected at all.
- `run_lane_mutants.rs:404-407`: same pattern for `cargo mutants --list --json`
  output. A malformed JSON from cargo-mutants becomes a clean empty report
  per package.

Both sites should propagate the parse error as `Err(...)` with the cargo
tool's stderr attached, so the lane returns `LaneOutcome::Failed {
LaneFailure::Infra { ... } }` rather than a passing clean outcome.

### F-08 — Moderate — Per-finding rule-id fallback chain has a latent bug in the Kani lane
**Severity:** WARN (latent bug; not triggered today).

`run_lane_kani.rs:70-75`:
```rust
static FALLBACK_RULE_ID: LazyLock<Result<RuleId, RuleIdError>> =
    LazyLock::new(|| match (RuleId::new("PROOF_KANI_FAIL"), RuleId::new("MUTANT_SURVIVED")) {
        (Ok(id), _) => Ok(id),
        (Err(primary_err), Err(_secondary_err)) => Err(primary_err),
        (Err(primary_err), Ok(_secondary)) => Err(primary_err),   // BUG
    });
```

The third arm discards a successful secondary id and returns the primary
error. The intent is *"use the first valid id; only fail if both fail."*
Correct version:
```rust
(Ok(id), _) | (_, Ok(id)) => Ok(id),
(Err(primary_err), Err(secondary_err)) => Err(primary_err),
```

In practice the bug is dormant because `PROOF_KANI_FAIL` is a well-formed
RuleId. The mutants lane (`run_lane_mutants.rs:104-110`) writes the same
fallback correctly with the `(Ok, _) | (_, Ok)` pattern — but pairs it with
`"MUTANT_SURVIVED"` for both slots, which is also wrong: the Kani lane
should not produce `MUTANT_SURVIVED` as a fallback id and vice versa.

Recommendation: drop the `LazyLock<Result<…>>` in favour of a single
`static FALLBACK_RULE_ID: LazyLock<RuleId>` initialised via `expect("...")
` at startup (this is the one place a startup-time `expect` is acceptable
because the literal is a compile-time constant). Eliminates the chained
fallback entirely.

### F-09 — Moderate — Infrastructure errors are reported as reject findings, not as `LaneFailure::Infra`
**Severity:** BLOCK_LOCAL (correctness + spec §2 error taxonomy).

- `run_lane_kani.rs:124-126`: a non-tool-missing infrastructure error from
  `cargo kani list` is converted to `LaneOutcome::Findings { findings: vec!
  [infra_finding(...)] }` with `FindingEffect::Reject`. The rejection
  blocks the gate as if it were a code violation. The spec and the
  v1 contract require `LaneOutcome::Failed { failure: LaneFailure::Infra
  { tool, reason } }` for these cases so the aggregator can distinguish
  infrastructure from code and the exit code can become `≥ 4` rather
  than `1`.
- `run_lane_mutants.rs:159-162`: same pattern — per-package list failure
  becomes `infra_finding(package, &reason)` with reject effect, instead
  of a `LaneFailure::Infra`.
- `run_lane_kani.rs:128-136`: `rule_id_failure_outcome` does the right
  thing (`LaneFailure::Infra`) but is only reachable from the per-harness
  findings path. The list-error path bypasses it.

### F-10 — Moderate — Error variants are reused for semantically wrong reasons
**Severity:** WARN (correctness of error reporting; not blocking).

- `run_lane_kani.rs:105-107`: `build_clean_outcome` errors are mapped to
  `KaniLaneError::NotACargoWorkspace(format!("evidence build failed: ..."))`.
  The variant semantically means "the target is not a Cargo workspace" —
  using it to wrap an `OutcomeError` is misleading. Add a
  `KaniLaneError::EvidenceBuild { reason: String }` variant.
- `run_lane_mutants.rs:138-141`: same misuse — `OutcomeError` is mapped
  to `MutantsLaneError::BaselineMissing`. Add a dedicated
  `MutantsLaneError::EvidenceBuild { reason: String }`.

### F-11 — Moderate — `MutantId::new` and `MutantBaselineEntry` accept unbounded `String` payloads
**Severity:** WARN (Rule 3 / Rule 4 allocation discipline).

- `proof_id.rs:201`: `format!("{package}::{rel_path}:{line}:{col}:{}", ...)` —
  no length cap on `package` or `rel_path`. A 10 MiB path produces a 10 MiB
  MutantId string. The downstream lane then normalizes this into a
  `MUTANT_SURVIVED_<...>` rule-id literal that may exceed `RuleId::MAX_LEN
  = 96`. The lane's fallback catches this (`run_lane_mutants.rs:255` →
  `RuleIdError::TooLong`), so it fails closed at the rule-id boundary, but
  the heap allocation is unbounded at construction. Cap `package` and
  `rel_path` length at load time.
- `mutants_baseline.rs:21-31`: `MutantBaselineEntry` fields are all
  `String` with no max length. A hostile or hand-edited baseline can put
  1 GiB into `reason` and the lane will hold it in memory throughout.
- `proof_id.rs:25` (`KaniHarnessId`) — fine, already capped at 96 bytes.
- `proof_id.rs:179` (`MutantId`) — uncapped, see above.

### F-12 — Moderate — Mutant ids are built as raw `String`, bypassing the `MutantId` validating newtype
**Severity:** WARN (kills the invariant the newtype was added for).

`run_lane_mutants.rs:459-463` `build_mutant_id` constructs the mutant id
via `format!("{package}::{}:{line}:{col}:{}::{}", ...)`. It then short-
circuits the absent-span case to `0` for line/col (`run_lane_mutants.rs:
460-461`). `MutantId::new` in `proof_id.rs:251-256` rejects `line == 0`
and `col == 0`. By stringifying directly, the lane produces a `MutantId`
that violates the invariant the core constructor enforces. The id is later
normalized into a rule id via `normalize_mutation_id`
(`run_lane_mutants.rs:309-311`) which itself bypasses the newtype.

Recommendation: parse `RawMutant` fields into the typed `MutantId` via
`MutantId::new` and propagate the typed error. Then the baseline lookup
operates on canonical ids.

### F-13 — Moderate — `stdout` is captured but immediately discarded
**Severity:** WARN (allocation hygiene).

`run_lane_kani.rs:573-580` `list_kani_for_crate` uses
`Command::output()`, which captures the child's stdout into a `Vec<u8>`
even though the lane reads its data from the `kani-list.json` file the
child wrote. The captured `Vec<u8>` is dropped at the end of the
expression. For a cargo-kani run that emits verbose output, this is a
wasteful allocation. Switch to `Command::spawn()` + `Command::wait()` and
only read the artifact file.

### F-14 — Moderate — `HarnessRun::completed` concatenates stdout+stderr into one unbounded `String`
**Severity:** WARN (Rule 3 / Rule 4).

`run_lane_kani.rs:198`: `let combined = format!("{stdout}\n{stderr}");`
The `stdout` and `stderr` strings were read from the child pipes via
`read_to_string` (uncapped). A misbehaving cargo-kani can emit hundreds
of MB of CBMC diagnostics; the lane will hold them all. Cap `drain_pipe`
at `MAX_PIPE_BYTES` and replace the rest with an ellipsis.

### F-15 — Moderate — `poll_kani_child` is the only path; no cancellation primitive
**Severity:** WARN (async/concurrency governance).

`poll_kani_child` busy-polls a `Child` on a 50 ms tick with
`std::thread::sleep`. There is no `Child::kill_on_drop` wrapper, no
signal handler, no shutdown channel — if the lane orchestrator wants to
abort the run, it has no handle. `titania-lanes/Cargo.toml:22` already
declares `wait-timeout = "0.2.1"` but the v1.5 file does not use it. If
the intent is `wait-timeout` for child reaping, wire it in. If not, drop
the dep (`cargo machete` will flag it).

### F-16 — Moderate — `normalize_harness_name` and `normalize_mutation_id` collapse to `_` with no length cap
**Severity:** WARN (Rule 3 / spec hazard).

`run_lane_kani.rs:444-451` and `run_lane_mutants.rs:309-311`: both produce
a normalized `String` whose length is `≤ input.len()`. A 1 KiB harness
name from `cargo kani list` JSON produces a 1 KiB rule-id literal that
**always** exceeds `RuleId::MAX_LEN = 96`. The lane falls back to
`PROOF_KANI_FAIL` / `MUTANT_SURVIVED` — losing the per-harness identity
that the spec says is the whole point of the rule. Cap the input or
hash the long-form id and append the hash.

### F-17 — Minor — Kani lane re-validates workspace root
**Severity:** INFO (defensive, not wrong).

`run_lane_kani.rs:454-460` `has_workspace_root` reads `Cargo.toml` and
looks for a `[workspace]` table. The parent dispatcher already validates
the target via `current_target_project()` and constructs a typed
`TargetProject`. The Kani lane's re-check is redundant (and consumes an
extra `read_to_string` on every run). Either remove it (trust the
parent) or have it return a typed `Result<(), KaniLaneError>` and
fold it into `outcome` so the failure mode is observable.

### F-18 — Minor — Tool version is hard-coded in five places
**Severity:** WARN (maintenance hazard).

`run_lane_kani.rs:92, 333, 353, 371, 392`: `"0.67.0"` is duplicated five
times in the Kani lane; `run_lane_mutants.rs:127, 261` duplicates
`"27.0.0"`. The actual cargo-kani / cargo-mutants version is never
queried. If the runtime version differs from the literal, the
`Location::tool(name, version)` payload lies. Either query the version
(`cargo kani --version` → parse) once at startup, or accept a
`tool_version: &str` argument from the Moon task layer.

### F-19 — Minor — `verdict_from_line` uses substring matching for "UNSUPPORTED"
**Severity:** INFO.

`run_lane_kani.rs:294`: `other if other.contains("UNSUPPORTED") => HarnessVerdict::Unsupported`.
A verification line that contains "UNSUPPORTED" inside an unrelated token
(e.g. `VERIFICATION: PARTIALLY_UNSUPPORTED_FEATURE`) will be classified as
Unsupported. Use exact-match against a closed set.

### F-20 — Minor — JSON parses are lenient (no `deny_unknown_fields`)
**Severity:** WARN.

Neither `MutantsBaseline` / `MutantBaselineEntry` nor `RawMutant` /
`KaniListFile` use `#[serde(deny_unknown_fields)]`. Unknown fields are
silently dropped. For the mutants baseline this is forward-compat (good);
for `RawMutant` it is mostly the same. But the audit asked: **the answer
is "lenient"** — call it out and document the decision, or add
`#[serde(deny_unknown_fields)]` where the schema is closed.

### F-21 — Minor — Spec/implementation mismatch on `KaniHarnessId` charset
**Severity:** WARN.

`proof_id.rs:25-50`: `KaniHarnessId` enforces uppercase-only
(`[A-Z][A-Z0-9_]*`). Spec §3 says
`KaniHarnessId::new(name)` validates `^[a-zA-Z][a-zA-Z0-9_]*$`. The
implementation is stricter (no lowercase allowed) — this is intentional
because the rule-id grammar is uppercase-only, but the spec is stale.
Update the spec or call out the deviation.

### F-22 — Minor — `ToolKind` is defined but unused
**Severity:** WARN.

`proof_id.rs:269-284` defines `ToolKind { CargoKani, CargoMutants }` and
exports it from the crate root. Grep across the v1.5 lane files shows
no usages (`run_lane_kani.rs`, `run_lane_mutants.rs`). Either the lanes
should adopt it (replacing the hard-coded `KANI_TOOL` const and the
`Location::tool("cargo-mutants", ...)` literal) or it should be removed.
Dead public API is a maintenance hazard.

### F-23 — Minor — `LaneFailure::Resource` is never produced by v1.5 lanes
**Severity:** INFO.

The v1 contract enumerates `Resource { limit }` for OOM scenarios. The
Kani and mutants lanes never produce this variant; OOM is currently
collapsed into `PROOF_KANI_BLOCKED` (informational "blocked by cgroup
OOM or wallclock timeout") which doesn't carry the `limit`. Either add
detection (via the cgroup's exit code) and emit `Resource { limit:
"MemoryMax=24G" }`, or remove the variant from the contract.

### F-24 — Minor — `MutantsBaseline::from_bypasses` is `const fn` but holds a `String`-bearing struct
**Severity:** WARN.

`mutants_baseline.rs:66-68`: `pub const fn from_bypasses(entries: Vec<MutantBaselineEntry>) -> Self` —
const-fn with `Vec` and `String`-containing struct fields. Const-stable
construction of `Vec`/`String` in stable Rust is limited (Vec moved in is
fine, but you cannot construct a Vec with capacity in const). As written
this *may* compile because the body is just a struct literal with a
moved-in Vec — but it is at the edge of stable const guarantees and
could break across toolchain bumps. Either drop the `const` qualifier
(it's never called from a const context per the grep) or write a
test that exercises this constructor.

### F-25 — Minor — `baseline_path` hard-codes `.titania/profiles/strict-ai/`
**Severity:** WARN.

`run_lane_mutants.rs:466-468`: the baseline path is a literal `PathBuf`
concat. The mutants lane has no way to run against a non-default baseline
(e.g. for local experimentation or for a different scope). Take it from
the target project manifest or via an explicit override.

### F-26 — Minor — `CommandEvidence` argv lies about the actual invocation
**Severity:** WARN (evidence integrity).

`run_lane_mutants.rs:319-338` records argv as
`["cargo-mutants", "mutants", "--baseline", ".titania/... (N pkgs)"]`.
This is not what was run. The actual command was per-package
`cargo mutants --list --json --no-shuffle -p <pkg>`. Receipt auditors
reading the artifact will see a command that never executed. Either
record each per-package invocation as its own `CommandEvidence`, or
record the workspace-level wrapper if you add one.

### F-27 — Minor — `RuleId` literal has underscore check that fails for `PROOF_KANI_BLOCKED` etc.
**Severity:** INFO (confirmed working).

Verified `PROOF_KANI_PASS`, `PROOF_KANI_FAIL`, `PROOF_KANI_BLOCKED`,
`PROOF_KANI_UNSUPPORTED`, `PROOF_KANI_INFRA`, `MUTANT_SURVIVED`,
`MUTANT_SURVIVED_INFRA` against `RuleId::new`'s grammar at
`rule_id.rs:77-85`. All seven literals are non-empty, contain at least
one underscore, are uppercase ASCII, and are ≤ 96 chars. **Pass** — no
finding here, listed only to confirm the rule-id literals are well-formed.

### F-28 — Minor — `FALLBACK_RULE_ID` is a `LazyLock<Result<…>>`
**Severity:** WARN.

`run_lane_kani.rs:70-75` and `run_lane_mutants.rs:104-110`. The `Result`
wrapper means every consumer has to `.as_ref().expect(...)` or pattern-
match again. A typed invariant like `OnceLock<RuleId>` plus a startup
`expect` (acceptable because the literal is a compile-time constant) is
simpler and prevents the F-08 latent bug by construction.

### F-29 — Minor — `titania-lanes/Cargo.toml` declares unused `wait-timeout`
**Severity:** WARN.

`Cargo.toml:22`: `wait-timeout = "0.2.1"` is declared but not used by any
of the v1.5 files (grep confirmed). This will trip `cargo machete` once
that gate is enabled. Either wire it in (see F-15) or remove.

### F-30 — Minor — `drain_pipe` accepts `Option<impl Read>` but never returns the truncated-read outcome
**Severity:** WARN.

`run_lane_kani.rs:204-211`. The `None` branch returns `String::new()` —
indistinguishable from "empty pipe". For diagnostic purposes this
collapses "child had no stdout" with "child closed stdout early". The
finding the lane emits will say "VERIFICATION: …" not found; the
operator cannot tell which case happened.

---

## Required Fixes (blockers)

These MUST be repaired before the lane ships to CI:

1. **F-01** — switch the mutants lane to actual test-survivor detection.
   Use `cargo mutants -o .titania/out/full/mutants.out --json` (workspace
   level) and parse `outcomes.json` + `mutants.json` per spec §4.3 step 4.
   Or, if `--list` is the intended fast-discovery posture, rename the
   finding from `MUTANT_SURVIVED` to `MUTANT_DISCOVERED` and remove the
   "survivor" claim from the spec.

2. **F-02** — bring per-invocation pattern into alignment with the spec
   (one cargo-kani per package with `--output-format=regular`, parse
   per-harness lines from a single run; one cargo-mutants workspace run
   with the directory output).

3. **F-03** — invoke `systemd-run --user --scope -p MemoryMax=24G -p
   MemorySwapMax=0` from the lane, OR document in the doc-comment at the
   top of each lane that the cgroup is a Moon-task-layer invariant the
   Rust code does not enforce, and that the lane only *records* OOM
   outcomes.

4. **F-06** — replace `drop(fallible_call(...))` with explicit match
   arms that propagate typed errors (`read_to_string` truncation and
   `remove_file` `NotFound`-only ignore).

5. **F-07** — propagate JSON parse errors as `LaneFailure::Infra` rather
   than downgrading them to empty result vectors.

6. **F-09** — change infrastructure-failure outcomes from
   `LaneOutcome::Findings { FindingEffect::Reject }` to
   `LaneOutcome::Failed { failure: LaneFailure::Infra { ... } }`.

7. **F-10** — add a dedicated `EvidenceBuild` variant to
   `KaniLaneError` and `MutantsLaneError`; stop reusing
   `NotACargoWorkspace` / `BaselineMissing`.

---

## Recommended Fixes (improvements)

These are Holzman-doctrine tightening; not strictly blocking:

- **F-04** — express the polling loop bound as
  `for _ in 0..(PER_HARNESS_TIMEOUT_SECS * 1000 / POLL_INTERVAL_MS)` with
  a named `POLL_INTERVAL_MS = 50` and a doc comment citing the proof.
  Add a `MAX_HARNESSES_PER_WORKSPACE` constant and assert at inventory
  load.
- **F-05** — convert `MutantsBaseline::entries` to a `HashMap` at load
  time so `contains` and the lane-level diff are O(1) per query.
- **F-08** — fix the `FALLBACK_RULE_ID` match arm or replace the
  `LazyLock<Result<…>>` with `OnceLock<RuleId>`.
- **F-11** — cap `package` and `rel_path` lengths at
  `MutantId::new`, and `reason` length at `MutantBaselineEntry` parse.
- **F-12** — route every `build_mutant_id` site through the typed
  `MutantId` constructor.
- **F-13** — switch `list_kani_for_crate` from `.output()` to
  `.spawn() + .wait()`; skip the stdout capture entirely.
- **F-14** — cap `drain_pipe` at `MAX_PIPE_BYTES` (e.g. 256 KiB).
- **F-15** — wire `wait-timeout` for child reaping or remove the dep;
  add a `Child::kill_on_drop` wrapper.
- **F-16** — cap `normalize_harness_name` / `normalize_mutation_id`
  output length to a `MAX_NORMALIZED_LEN` constant; hash longer inputs.
- **F-18** — query the tool version via `cargo kani --version` once at
  startup, store in `LaneRunState`.
- **F-19** — use exact-match for the unsupported verdict.
- **F-20** — add `#[serde(deny_unknown_fields)]` on `MutantsBaseline`
  and `KaniListFile` (the mutants baseline is closed-schema).
- **F-21** — update the spec to match the implementation's uppercase-
  only charset (or loosen the implementation).
- **F-22** — adopt `ToolKind` in the lane files or remove the unused
  re-export.
- **F-23** — emit `LaneFailure::Resource { limit }` when the cgroup
  kills the child (SIGKILL with the documented MemoryMax).
- **F-24** — drop `const` from `from_bypasses` unless a const-context
  caller is added.
- **F-25** — parameterise the baseline path or read it from the target
  project manifest.
- **F-26** — record per-package invocations as separate
  `CommandEvidence`; do not synthesise an argv that was never run.
- **F-28** — replace `LazyLock<Result<RuleId>>` with
  `OnceLock<RuleId>` + startup `expect`.
- **F-29** — wire `wait-timeout` or remove the unused dep.
- **F-30** — distinguish "no stdout captured" from "stdout was empty".

---

## Compliant Patterns (good examples)

These deserve to be preserved and emulated in adjacent code:

- **C-01 — Smart constructors returning typed errors.** `proof_id.rs:50,
  193` `KaniHarnessId::new` and `MutantId::new` return `Result<Self,
  KaniHarnessIdError | MutantIdError>`, with every variant carrying a
  structured payload and a `Display` impl. Exactly the shape the
  AGENTS.md error-discipline section requires.

- **C-02 — Closed-set enum for operators.** `proof_id.rs:137-154`
  `MutantOperator` is a closed enum (no `Unknown(String)` escape hatch)
  with `#[serde(rename_all = "snake_case")]` and a `const fn as_str`.
  `ToolKind` (lines 269-284) is the same pattern. This is the right
  way to forbid new operators without a contract amendment.

- **C-03 — Bounded string validation.** `proof_id.rs:86-128` `check_khi`
  caps length via `KANI_HARNESS_ID_MAX_LEN = 96` and rejects non-ASCII
  at the byte boundary, before any allocation. This is the cleanest
  example in the v1.5 set.

- **C-04 — `check_khi_chars` iterator pipeline.** `proof_id.rs:117` uses
  `try_for_each` with byte-level iteration; the loop is bounded by the
  already-checked `KANI_HARNESS_ID_MAX_LEN` — a static upper bound.
  Textbook Power-of-Ten Rule 2.

- **C-05 — `is_none_or` for Option-or-default comparisons.**
  `mutants_baseline.rs:182` `entry.expires_on_unix.is_none_or(|exp|
  now_unix <= exp)` — uses the stable `Option::is_none_or` (1.82+) to
  express "Option-or-true-with-predicate" without falling back to
  `unwrap_or` (which is forbidden by `clippy.toml`).

- **C-06 — `const fn` for small predicates.** `proof_id.rs:126-128
  is_khi_byte` and `mutants_baseline.rs:55-68 from_bypasses` use
  `const fn` for boundary predicates; this lets the compiler fold
  validation in release builds. The size of the function bodies stays
  small.

- **C-07 — No `unsafe` anywhere in v1.5 code.** Confirmed by grep —
  zero `unsafe` blocks, zero raw-pointer dereferences, zero `transmute`.
  Matches the AGENTS.md unsafe-law.

- **C-08 — No production `unwrap`/`expect`/`panic!`/`todo!`.** Confirmed
  by grep across all four files. The `#[expect(dead_code, reason = "…")]`
  attributes (six occurrences) are clippy's *lint expectations*, not
  panic paths — they are correct, every reason is documented.

- **C-09 — Every `Result`-returning function has an `# Errors` rustdoc
  section.** Verified by inspection — the lanes' `cargo doc` build will
  emit documented error paths for every public function. Satisfies
  `missing_errors_doc = "deny"` in the workspace lints.

- **C-10 — Iterator pipelines instead of imperative loops.**
  `proof_id.rs:117`, `mutants_baseline.rs:104`, `run_lane_mutants.rs:178
  -183`, `run_lane_kani.rs:622-631`, `run_lane_kani.rs:150-157` all use
  `iter()` / `try_fold` / `flat_map` rather than `for` / `while`. This
  is the functional-rust lane's preferred style and it makes the
  boundedness argument easy to write down.

- **C-11 — Function size ceiling respected.** Every function in the
  four files is under `too-many-lines-threshold = 60`. The longest
  is `parse_baseline` at 33 lines (mutants lane). All others are
  under 30 lines.

- **C-12 — Variable declared at first use.** No `let mut` of state that
  is then mutated far from the binding. The two `LaneRunState`
  accumulators are passed by `&mut` and folded through helper
  functions — scope stays narrow.

- **C-13 — Arity within `too-many-arguments-threshold = 5`.** The largest
  function signature is `MutantId::new` at 5 args (one is the operator
  enum, by-value). No `clippy::too_many_arguments` violation.

- **C-14 — `LazyLock` for one-time-init constants.**
  `run_lane_kani.rs:70-75` and `run_lane_mutants.rs:104-110` use
  `std::sync::LazyLock` for the fallback rule id. Thread-safe, no
  `unsafe`, no `OnceCell` boilerplate. (See F-28 for the `Result`
  wrapping nit.)

- **C-15 — Explicit `RuleId` literal validation at startup.**
  `RuleId::new("PROOF_KANI_PASS")` etc. are validated by the rule-id
  grammar (`rule_id.rs:77-85`). The lanes call `RuleId::new` once per
  finding and use the typed error to surface a `LaneOutcome::Failed`
  rather than fabricating an id. This is the right way to handle
  rule-id collisions between the rule family and the lane fallback.

- **C-16 — Rule-id literacy surfaced as findings, not panic paths.**
  `run_lane_kani.rs:128-136` and the per-harness helpers all convert
  `RuleIdError` into either a typed lane error or a
  `LaneFailure::Infra { ... }` finding. No panic, no `unreachable!`,
  no swallowed `Result`. (The improvements called out in F-08 / F-09
  apply to other sites.)

- **C-17 — `findings_for_package` separates "infra failure" from "new
  survivor".** `run_lane_mutants.rs:157-162` uses a `match` to handle
  the per-package `Result` cleanly, keeping the error path out of the
  main `Ok` arm. Pattern is right; just needs `LaneFailure::Infra`
  propagation per F-09.

---

## Power-of-Ten Rule Map

| Rule | Status | Notes |
|------|--------|-------|
| 1. Simple control flow | PASS | No recursion, no macro-hidden branches, no closure-hidden state. |
| 2. Fixed loop bounds | PARTIAL | `poll_kani_child` is bounded by `PER_HARNESS_TIMEOUT_SECS` only — runtime cap, not static proof. See F-04. |
| 3. No post-init allocation in critical paths | PASS for control flow; WARN on hot-path Strings | F-11, F-13, F-14, F-16 call out unbounded `String` growth. |
| 4. Functions fit on one page | PASS | All under 60 lines; longest is 33. |
| 5. Assertion/invariant density | PASS | Invariants enforced via types (`RuleId`, `MutantId`, `MutantsBaseline::load`, `KaniHarnessId::new`). Zero production `assert!` macros. |
| 6. Smallest scope | PASS | Variables declared near first use; mutation scoped to `LaneRunState` accumulators. |
| 7. Checked returns and parameters | PARTIAL | Two `drop(fallible_call())` violations (F-06); JSON parse swallowing (F-07). |
| 8. Limited macro/preprocessor power | PASS | No procedural macros beyond `#[derive(...)]`, `#[serde(...)]`, `#[expect(...)]`. |
| 9. Restricted pointer/indirect-call use | PASS | No `unsafe`, no `*mut`, no `&dyn`. `LazyLock` and `Box<str>` are safe abstractions. |
| 10. Warnings and analysis mandatory | UNVERIFIED | Cannot run `cargo clippy` from this review; the source-level patterns look clean. Workspace lint config is set up correctly; `clippy.toml` and `[workspace.lints]` match. |

## Allocation / Concurrency / Numeric Tally

- **Post-init heap allocation in lanes**: bounded per-finding `format!`s and
  `read_to_string` of cargo-kani / cargo-mutants output. The cargo output
  buffers are uncapped (F-14).
- **Mutability**: `mut` appears only on `LaneRunState` accumulators and on
  `Vec` builders passed by `&mut` to fold helpers. Narrow.
- **Locks across await**: N/A — no async in these files.
- **`Send + Sync`**: all types are `Send + Sync` (no `Rc`, no `RefCell`, no
  raw pointers).
- **Loom**: dev-dep only; v1.5 lane code is sync, no concurrent state to
  model.
- **Integer arithmetic**: only `saturating_add` (line 391) and a comparison
  `code > state.exit_code` (line 388). No division, no modulo.
- **Casts**: no `as` conversions in the four files (grep confirmed).
- **Float**: none.

## Verification Discipline Notes

- No third-ring evidence (assembly, IR, SBOM) is required for this review.
- No benchmark claims are made; nothing in the new code is on a measurable
  hot path that would warrant one.
- No `cargo geiger` check was performed from this review; if any of the new
  code added `unsafe`, the change would be visible immediately.
- The blocking items in **Required Fixes** must be cleared with their own
  focused commits; do not bundle them with this review.
- The fixes for **F-01 / F-02 / F-03** are spec-vs-implementation choices —
  the team should make a conscious decision about whether the spec or the
  implementation is authoritative for each.

## Residual Risks

1. The v1.5 spec has not yet been kept in sync with the implementation
   (F-21 spec/impl charset drift; F-25 hard-coded baseline path). This
   drift will compound as the gate matures.
2. `ToolKind` (F-22) and `wait-timeout` (F-29) are unused public/dep
   surface. Both are silent maintenance hazards that compile fine but
   mislead future readers.
3. The mutants lane *cannot* distinguish survivors from non-survivors
   until F-01 is repaired. Until then, the lane is effectively a
   "discovered-not-baselined" alarm, not a survivor gate.
4. The Kani lane's per-harness invocation pattern (F-02) means the
   wallclock cost of the Kani lane is bounded by
   `(harness_count × per_harness_timeout) = (harness_count × 60s)` plus
   per-harness build overhead. A workspace with 50 harnesses will spend
   ≥ 50 minutes on Kani alone. The spec pattern (one package-level run)
   is bounded by `per_package_timeout × package_count` and amortises
   build cost.
5. `FALLBACK_RULE_ID` (F-08, F-28) is a latent bug. It is dormant only
   because all current rule-id literals are well-formed. The next
   `RuleId` grammar tightening will silently turn every Kani finding
   into a single `PROOF_KANI_FAIL` regardless of harness.
