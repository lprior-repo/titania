# Functional-Rust + Scott-Wlaschin DDD Audit — v1.5 Kani + Mutants

```
Scope:        v1.5 Kani + Mutants migration
Bead:         tn-7bq2.1 (contract); tn-7bq2.3/.4/.5 (impl/test lanes)
Date:         2026-07-16
Files:        4 source files + their contract + tests
Reviewer:     functional-rust / Scott-Wlaschin DDD doctrine
Doctrine:     ../functional-rust/SKILL.md §1–14
              ../AGENTS.md §2 (operating order) + §5 panic-free + §7 control-flow
Status:       AUDIT, no source edits performed
```

## Verdict

**FAIL — partial migration with one critical DDD violation, four typed-error
violations, and three control-flow violations. The functional-core shell in
`titania-core/src/{proof_id,mutants_baseline}.rs` is clean and disciplined.
The imperative shell in `titania-lanes/src/run_lane_{kani,mutants}.rs`
**bypasses every newtype the core defines** and re-implements a parallel
string-typed universe, then smuggles `String` errors through `Result<T, String>`
shell signatures. The contract's bound types (`KaniHarnessId`, `MutantId`,
`MutantsBaseline`, `MutantBaselineEntry`) are vestigial — they exist in core and
are asserted on by tests, but the lanes that emit findings never construct,
parse into, or thread them. This is the worst class of DDD violation: a
divided type universe where the type only exists in the part of the program
that is never touched at runtime.**

**Disposition**:

| Area | Verdict |
|------|---------|
| `proof_id.rs` (core) | PASS — parse-don't-validate, total enum, typed errors, smart constructor |
| `mutants_baseline.rs` (core) | PARTIAL — file-level schema good, but `from_bypasses` bypasses validation, and `diff(&[String])` is stringly typed |
| `run_lane_kani.rs` (shell) | FAIL — `String`-typed harness name, `Result<_, String>` signatures, `loop { try_wait }` imperative control flow |
| `run_lane_mutants.rs` (shell) | FAIL — bypasses `MutantId` and `MutantsBaseline` entirely; parallel `Baseline`/`BaselineEntry` types; `format!`-built id strings stored as `Vec<String>` |

**Required fixes before this audit clears:**
1. The mutants lane must import and construct `MutantId` instead of `String`,
   parse `RawMutant` rows into `MutantId::new(...)?`, and thread `Vec<MutantId>`
   through `MutantsBaseline::diff`. (Findings F-01, F-05.)
2. The mutants lane must replace its parallel `Baseline`/`BaselineEntry`
   shadow types with `MutantsBaseline` / `MutantBaselineEntry`. (F-02.)
3. The Kani lane must validate discovered harness names through
   `KaniHarnessId::new` before constructing findings. (F-04.)
4. All `Result<T, String>` shell signatures must become typed errors. (F-08.)
5. `KaniLaneError` / `MutantsLaneError` must gain an internal-error variant so
   that "evidence build failed" and "rule id construction failed" do not
   re-mint as the wrong variant. (F-11.)
6. The `loop { try_wait }` in `poll_kani_child` and the two `for` loops in
   `run_lane_mutants.rs` must be replaced with bounded iterator/fold pipelines
   with explicit iteration caps. (F-09.)

---

## Findings

Severity legend: **CRITICAL** = blocker (DDD / type-system bound contract);
**MAJOR** = DDD violation requiring rework; **MINOR** = nit that affects
maintainability. All references are `file:line`.

### F-01 — **CRITICAL** — `MutantId` smart constructor is dead code on the v1.5 happy path

`crates/titania-lanes/src/run_lane_mutants.rs:459-462`

```rust
fn build_mutant_id(package: &str, m: &RawMutant) -> String {
    let line = m.span.as_ref().and_then(|s| s.start.as_ref()).map_or(0, |p| p.line);
    let col  = m.span.as_ref().and_then(|s| s.start.as_ref()).map_or(0, |p| p.column);
    format!("{package}::{}:{line}:{col}:{}::{}", m.file, m.genre, m.name)
}
```

`build_mutant_id` returns `String`, **never** `MutantId`. The id shape produced
here is `{package}::{file}:{line}:{col}:{genre}::{name}` with two `::` doublets,
while `proof_id.rs::MutantId::new` produces `{package}::{rel_path}:{line}:{col}:{operator}`.
**The shapes are incompatible.** The lane invents a parallel id universe that
cannot round-trip through `MutantId` because (a) `m.genre` is `String` not
`MutantOperator`, and (b) the lane format appends `::{name}`.

What this costs: an illegal `line: 0` or `col: 0` survives every layer of
validation because the lane never runs it through `check_mutant_shape`.
Cargo-mutants discovery that lacks a `span` field (test file in v27 occasionally
emits these) silently produces `0:0` ids that the newtype would reject as
`LineZero`/`ColZero`.

**Fix**: lane imports `MutantId`, `MutantOperator`, and constructs every mutation
id via `MutantId::new(package, rel_path, line, col, operator).map_err(...)`. Map
`m.genre` to a `MutantOperator` via a dedicated `MutantOperator::from_genre`
parser that exhaustively maps every `genre` value cargo-mutants is documented
to emit; unknown genres bubble up as `MutantIdError::UnknownOperator`.

### F-02 — **CRITICAL** — `Baseline`/`BaselineEntry` shadow types bypass `MutantsBaseline`

`crates/titania-lanes/src/run_lane_mutants.rs:42-55, 67-71`

```rust
struct BaselineEntry { mutation_id: String, owner: String, reason: String, expiry: String }
struct Baseline      { entries: Vec<BaselineEntry> }
```

These two types are an *independent re-implementation* of
`MutantsBaseline`/`MutantBaselineEntry` from `mutants_baseline.rs`. Neither is
imported into the lane. Neither uses the `mutation_id: MutantId` field the
contract specified in `type-contracts.md` §"MutantsBaseline" / "MutantBaselineEntry"
(specified shape: `mutation_id: MutantId`). The lane instead feeds the JSON
through serde with raw `String` fields and reads `expiry` as `String`,
parsing-as-`u64` at query time (line 62: `self.expiry.parse::<u64>()`).

Two consequences:
1. `m.bypasses[].mutation_id` is *never* run through `MutantId::new`. The
   `MutantIdError::EmptyPackage / EmptyPath / LineZero / ColZero / PathAbsolute /
   UnknownOperator` set is unreachable from the lane happy path. The validation
   in `MutantBaselineEntry::mutation_id` field in the *other* module
   (`mutants_baseline.rs`) reaches `validate_entry` post-hoc, but only when
   callers use `MutantsBaseline::load`. The lane doesn't.
2. The lane has no `owns_until` / `now_or_max` abstraction; expiry is parsed
   from `String` per call inside `BaselineEntry::matches`
   (`run_lane_mutants.rs:62`).

**Fix**: delete `Baseline`/`BaselineEntry`, import `titania_core::MutantsBaseline`
and `titania_core::MutantBaselineEntry`, and call `MutantsBaseline::load(&path)?`.
At that point the `load_baseline`/`parse_baseline`/`accumulate_inventory`/
`new_survivors` functions (`run_lane_mutants.rs:192-242, 172-183`) collapse to
a typed `MutantsBaseline::diff(&[MutantId]) -> Vec<MutantId>`.

### F-03 — **CRITICAL** — `Vec<String>` mutants report instead of `Vec<MutantId>`

`crates/titania-lanes/src/run_lane_mutants.rs:77, 408, 459-463`

```rust
struct MutantsListReport { mutation_ids: Vec<String> }
let ids: Vec<String> = list.iter().map(|m| build_mutant_id(package, m)).collect();
```

The report surface is stringly-typed; `parse_mutants_list` never produces a
`MutantId` even though one exists in the same crate. This means:
- F-01 extends: an entire pipeline of `String → format! → Vec<String> → NewSurvivor::mutation_id: String → Finding::reject` carries illegal-state values that would have been rejected at construction.
- `NewSurvivor.mutation_id: String` (`run_lane_mutants.rs:98`) is downstream of this — the survivor itself is a stringly-typed domain object.
- `mutant_survived_finding` (`run_lane_mutants.rs:251`) consumes the string and
  formats it back into both the location string *and* a rule-id suffix.

**Fix**: `MutantsListReport { mutation_ids: Vec<MutantId> }`,
`NewSurvivor { mutation_id: MutantId, ... }`, and surface `MutantIdError`
through `MutantsLaneError` via `#[from]`.

### F-04 — **MAJOR** — `KaniHarness.name: String` instead of `KaniHarnessId`

`crates/titania-lanes/src/run_lane_kani.rs:47-56`

```rust
struct KaniHarness {
    package: String,
    name: String,                                    // <-- should be KaniHarnessId
    #[expect(dead_code, reason = "informational; not yet surfaced in findings")]
    file: String,
}
```

The Kani harness id newtype (`proof_id.rs::KaniHarnessId`) is defined,
exports-validated, and tested by `v15_kani_harness_id.rs`, **but the lane never
constructs one**. Discovery produces raw `String` names from the cargo-kani JSON
output (lines 622-630), `normalize_harness_name` (`run_lane_kani.rs:444-451`) is
a lossy lowercase-collision-collapse that maps non-uppercase characters to `_`,
and the resulting `PROOF_KANI_<NAME>` rule-id literal goes straight to
`RuleId::new`. The whole `KaniHarnessId` pipeline is dead code from the lane's
side.

What this costs: the harness id upper-case / underscore / max-length invariants
the contract promises (`type-contracts.md` §"KaniHarnessId") are only checked
incidentally by the rule-id grammar. A harness whose name normalises to a valid
`RuleId` but is otherwise a different kind of string is represented as
`KaniHarness.name: String` with no type discipline.

**Fix**: `KaniHarness { name: KaniHarnessId, ... }`, surface
`KaniHarnessIdError` through `KaniLaneError`. The `normalize_harness_name`
helper becomes either dead code (no longer needed because the id is already
uppercase) or migrates to a `RuleId::normalized_suffix(&self)` helper inside
the lane.

### F-05 — **MAJOR** — `MutantsBaseline::diff` takes `&[String]` instead of `&[MutantId]`

`crates/titania-core/src/mutants_baseline.rs:103`

```rust
pub fn diff<'a>(&self, survivors: &'a [String], now_unix: u64) -> Vec<&'a String> { ... }
pub fn contains(&self, mutation_id: &str, now_unix: u64) -> bool { ... }
```

The core public API is stringly-typed. This is what enables F-03: a lane that
calls `MutantsBaseline::diff` does so over `Vec<String>`, defeating the newtype.
The contract (`type-contracts.md` §"MutantsBaseline") mandates:

> Methods: `MutantsBaseline::load(path: &Path)`,
> `MutantsBaseline::contains(mid: &MutantId) -> bool`,
> `MutantsBaseline::diff(&self, survivors: &[MutantId]) -> Vec<MutantId>`.

The implementation does not match. This is a **contract deviation**, not just a
style nit.

**Fix**: `(self, survivors: &[MutantId], now_unix: u64) -> Vec<MutantId>`. The
return type stays owned, not borrowed, because `MutantId` round-trips through
serde and through `Finding::Location` payloads without lifetime contamination.

### F-06 — **MAJOR** — `MutantBaselineEntry` has no smart constructor; `from_bypasses` bypasses validation

`crates/titania-core/src/mutants_baseline.rs:21-31, 46-68`

```rust
pub struct MutantBaselineEntry {
    pub mutation_id: String,         // <-- not newtype
    pub accepted_by_rule: String,
    pub reason: String,
    pub expires_on_unix: Option<u64>,
}
impl MutantsBaseline {
    pub const fn empty() -> Self { ... }
    pub const fn from_bypasses(entries: Vec<MutantBaselineEntry>) -> Self { ... }
}
```

Two problems:

1. **`from_bypasses` does not validate**. Compare with `MutantsBaseline::load`,
   which runs `validate_baseline` (rejects empty `mutation_id` and mismatched
   schema version). `from_bypasses` is **untyped and unguarded** — a
   hand-built `[MutantBaselineEntry { mutation_id: "".into(), ... }]` passes
   the constructor and produces a baseline that silently accepts every
   survivor (because `entry.mutation_id == ""` never equals any real id).
2. **`mutation_id` is `String`, not `MutantId`**. The contract said
   `mutation_id: MutantId`. The struct field had to remain `String` for serde
   (`#[serde(transparent)]` on `MutantId` means *any* string round-trips through
   JSON), but a smart constructor is the missing piece: `MutantBaselineEntry::new(id: MutantId, ...)` validates at construction; deserialization calls the same `new` so the typed invariant survives the JSON boundary.

**Fix**: add `MutantBaselineEntry::new` (returns `Result<MutantBaselineEntry,
MutantsBaselineError>`); tighten `mutation_id` to `MutantId`; remove
`from_bypasses` (or have it call `new`); implement a serde deserializer that
routes through the smart constructor so on-disk JSON cannot smuggle an empty id
or a malformed operator.

### F-07 — **MAJOR** — `KaniHarnessId::new`, `MutantId::new`, `MutantsBaseline::load` lack `#[must_use]`

`crates/titania-core/src/proof_id.rs:50, 193`
`crates/titania-core/src/mutants_baseline.rs:80`

Returning a `Result` from a `pub fn new` without `#[must_use]` means a caller
is *free* to drop the result on the floor:

```rust
KaniHarnessId::new(input);           // discarded; no diagnostic
MutantsBaseline::load(&path);        // discarded; baseline silently missing
```

Per AGENTS.md §2.4 and the functional-rust doctrine, fallible constructors must
carry `#[must_use]`, otherwise the compiler will not warn when a contributor
inserts `let _ = ` (per the project's strict unwrap/must-use lint set). The
strict lint set already denies `let_underscore_must_use`; an un-annotated
`pub fn new -> Result<Self, _>` allows `KaniHarnessId::new(s);` to compile
without warning.

**Fix**: annotate every `pub fn new` returning `Result<Self, _>` with
`#[must_use = "..."]`; same for `MutantsBaseline::load` and
`MutantsBaseline::from_bypasses`.

### F-08 — **MAJOR** — `Result<T, String>` in shell code (`run_lane_kani.rs` and `run_lane_mutants.rs`)

`crates/titania-lanes/src/run_lane_kani.rs:121, 473, 534, 568`
`crates/titania-lanes/src/run_lane_mutants.rs:374`

```rust
fn list_error_outcome(reason: &str) -> Result<LaneOutcome, KaniLaneError> {
    if reason.contains("no such subcommand") || reason.contains("not found") { ... }
    ...
}
fn list_kani_harnesses(...) -> Result<Vec<KaniHarness>, String>
fn list_kani_for_crate(...) -> Result<Vec<KaniHarness>, String>
fn list_mutants_for_package(...) -> Result<MutantsListReport, String>
```

The contract section 6 explicitly enumerates typed error variants for the
lanes. `KaniLaneError` should carry `ToolMissing { tool: ToolKind, reason: ... }`,
`SpawnFailed { tool: ToolKind, error: io::Error }`,
`ListParseFailed { tool: ToolKind, package: String, stderr: Box<str> }`,
`ArtifactsUnreadable { tool: ToolKind, path: Box<str>, reason: Box<str> }`, etc.

What we have instead:
- `list_error_outcome` sniffs substrings (`contains("no such subcommand")` /
  `contains("not found")`). Acceptable as an *adapter* layer, but it should
  classify *typed causes*, not match by substring. A future cargo version that
  emits "command not installed" would fall through the `if` and become a
  `PROOF_KANI_INFRA` finding instead of a `PROOF_KANI_NOT_RUN` skip.
- `Result<_, String>` propagates untyped errors upward; the lane's own error
  variants (`KaniLaneError`, `MutantsLaneError`) only have variants per top-level
  cause (NotACargoWorkspace, BaselineMissing, BaselineMalformed, RuleId), so a
  mid-pipeline "we couldn't read the kani-list.json artifact" failure gets
  formatted into the same string and lost.

**Fix**: replace `Result<T, String>` with `Result<T, KaniLaneError>` /
`Result<T, MutantsLaneError>`, add at minimum `{SpawnFailed, NonZeroExit,
ArtifactUnreadable, ListJsonMalformed}` variants to each.

### F-09 — **MAJOR** — Imperative loops

Per AGENTS.md §7 and functional-rust skill §16: "No imperative loops. Use Iterator/Stream pipelines."

**`run_lane_kani.rs:667-674`** — `poll_kani_child` is a `loop { match ... { Some(r) => return r, None => sleep(...) } }`:

```rust
fn poll_kani_child(child: &mut Child, start: Instant, timeout: Duration) -> HarnessRun {
    loop {
        match step_kani_child(child, start, timeout) {
            Some(result) => return result,
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}
```

This is the cleanest case of "bounded action loop that the skill itself
acknowledges can exist" — it polls until the child exits or the wallclock
elapses, sleeping 50ms between polls. However:
1. No max iteration count. If `Instant::elapsed()` ever stops advancing
   (monotonic regression; extremely rare but possible under FFI/cgroup pressure),
   the loop spins with no exit.
2. No tracing — a 60s timeout at 50ms is 1200 iterations; silent CPU spin for 60s
   on a hung harness is invisible.
3. Should be a `fun` action taking a `Child` and returning an
   `Outcome<Child, HarnessRun>` with explicit state-machine encode / decode so
   each loop iteration's progress is observable.

**Fix**: encode the poll as a state-machine function
`fn poll_step(child: &mut Child, start: Instant, timeout: Duration, attempt: u32)
-> Option<HarnessRun>` with a hard cap on `attempt`; emit a tracing span every
N attempts; `poll_kani_child` becomes
`std::iter::repeat(()).scan(...).find_map(...)`.

**`run_lane_mutants.rs:128-131`** — `for package in &packages { let step = ...; findings.extend(step); }`:

```rust
for package in &packages {
    let step = findings_for_package(workspace_root, package, &baseline, now_unix, &mut state)?;
    findings.extend(step);
}
```

Should be `packages.iter().try_fold(Vec::new(), |acc, pkg| { let step = ...?; ... })`. The early-return on `RuleId` is preserved; intermediate `findings.extend` becomes a fold accumulator.

**`run_lane_mutants.rs:165-167`** — `for survivor in &survivors { out.push(mutant_survived_finding(survivor)?) }`:

```rust
for survivor in &survivors {
    out.push(mutant_survived_finding(survivor)?);
}
```

Should be `survivors.iter().map(mutant_survived_finding).collect::<Result<Vec<_>, _>>()`.

### F-10 — **MAJOR** — `LaneRunState::exit_code: i32` allows wrong-side sentinel

`crates/titania-lanes/src/run_lane_kani.rs:60-67`
`crates/titania-lanes/src/run_lane_mutants.rs:81-89`

```rust
struct LaneRunState {
    exit_code: i32,    // <-- starts at 0; "I haven't observed one yet" indistinguishable from "0 was observed"
    harnesses_run: usize,
    tool_version: String,
}
```

Per DDD: `LaneRunState` is a typestate machine. Allowed states are
`Pending` (no cargo invocation yet), `Observed { max_exit: i32 }`,
`Failed(tool, reason)`. The current shape exposes an `exit_code: 0` field on
a never-started lane; combined with `harnesses_run: 0` is ambiguous.

`run_lane_mutants.rs:581-586` *exploits* this — `state.exit_code = state.exit_code.max(code)` where
`code` is computed as `-1` on `output.status.code() -> None`. So a Windows
process with no exit code is "more failed than zero".

The doctrinally-pure alternative is an enum:
```rust
enum ObservedExit { NotYetSeen, Exited { code: i32 }, Signaled { signal: i32, code: i32 } }
```
with the lane's `build_clean_outcome` switching on it.

**Fix**: convert `exit_code: i32` to `ObservedExit` typestate. Lifts the
secret-sentinel path (-1 for missing code) into a typed variant.

### F-11 — **MAJOR** — Evidence-build failure re-mints as wrong error variant

`crates/titania-lanes/src/run_lane_mutants.rs:136-141`

```rust
let build = build_clean_outcome(&state);
match build {
    Ok(lane_outcome) => Ok(lane_outcome),
    Err(error) => {
        Err(MutantsLaneError::BaselineMissing(format!("evidence build failed: {error}")))
    }
}
```

When `CommandEvidence::new` / `LaneEvidence::new` / `ProcessTermination`
construction fails, the lane *labels the failure `BaselineMissing`*. That's
wrong by type: the baseline file is irrelevant to the `CommandEvidence`
constructor. Operators reading the message will assume the bootstrap script
needs to run.

`crates/titania-lanes/src/run_lane_kani.rs:105-108`:

```rust
build_clean_outcome(&state, inventory.len()).map_err(|error| {
    KaniLaneError::NotACargoWorkspace(format!("evidence build failed: {error}"))
})
```

Same anti-pattern — wraps an evidence-build error as `NotACargoWorkspace`.

**Fix**: both error enums need an `Internal { tool: ToolKind, reason: Box<str> }`
or `Evidence(OutcomeError)` variant. `OutcomeError` already exists and is the
correct inner type here.

### F-12 — **MAJOR** — `HarnessRun` is a flag pile, not a typestate

`crates/titania-lanes/src/run_lane_kani.rs:166-201`

```rust
struct HarnessRun {
    exit_code: Option<i32>,
    timed_out: bool,
    stdout: String,
}
```

Three concurrent boolean/Option flags over the same struct mean an instance can
be `{exit_code: Some(0), timed_out: true, stdout: ""}` — illegal state.

**Fix**: typestate enum

```rust
enum HarnessRun {
    SpawnFailed { error: Box<io::Error> },
    WaitFailed  { error: Box<io::Error> },
    TimedOut    { stdout: String },
    Completed   { exit_code: i32, stdout: String },
}
```

Eliminates the flag pile; `parse_verdict`/`build_harness_findings` get a clean
state-machine dispatch.

### F-13 — **MAJOR** — `HarnessVerdict::from_line` substring-sniffs `UNSUPPORTED`

`crates/titania-lanes/src/run_lane_kani.rs:287-297`

```rust
fn verdict_from_line(line: &str) -> Option<HarnessVerdict> {
    let trimmed = line.trim_start();
    let verification = trimmed.strip_prefix("VERIFICATION:")?;
    let verdict = verification.trim().trim_start_matches('-').trim();
    Some(match verdict {
        "SUCCESSFUL" => HarnessVerdict::Successful,
        "FAILED" => HarnessVerdict::Failed,
        other if other.contains("UNSUPPORTED") => HarnessVerdict::Unsupported,
        _ => HarnessVerdict::Unknown,
    })
}
```

`other.contains("UNSUPPORTED")` is the same kind of substring sniffing as F-08.
If cargo-kani ever emits `"SUCCESSFUL_BUT_UNSUPPORTED"` it gets bucketed as
`Unsupported` (wrong); if it emits `"UNSUCCESSFUL"` it gets bucketed as
`Unknown` (right). A typed set of CBMC verdict strings should drive this
match: `["SUCCESSFUL", "PASSED", "VERIFIED"]` and `["FAILED", "FAILURE",
"REFUTED", "COUNTEREXAMPLE"]` and `["UNSUPPORTED", "UNRECOGNIZED FEATURE"]
etc., exhausted explicitly.

**Fix**: replace `contains` with an exhaustive typed `match` against a frozen
verdict vocabulary; new CBMC outputs (especially upcoming CBMC 6+) require a
contract amendment rather than a substring overflow.

### F-14 — **MAJOR** — Kani `PROOF_KANI_<NAME>` rule-id literal can exceed `RuleId::TooLong`

`crates/titania-lanes/src/run_lane_kani.rs:329, 349` + `pass_finding`/`fail_finding`

The lane constructs `format!("PROOF_KANI_{normalized}")` where `normalized`
maps every non-upper-alphanumeric to `_`. For a harness name that yields > ~85
uppers after normalization, the literal exceeds `RuleId`'s 96-byte cap,
`RuleId::new` returns `TooLong`, and `rule_id_for_harness_or_fallback` falls
through to the lane-wide static fallback (`PROOF_KANI_FAIL` or
`PROOF_KANI_PASS`). The fallback is *informational* for some verdicts and
*reject* for others — meaning a fleet of harnesses with long names collapses
into a single `PROOF_KANI_FAIL` finding that masks which harness actually
failed. Spec §D6 promised unique per-harness rule ids; this is a silent
collision.

**Fix**: either (a) truncate / hash long names to fit 96 bytes deterministically
with a documented suffix scheme, or (b) emit a `KaniLaneError::RuleId` failure
out of every harness whose id exceeds the cap, surfacing as
`PROOF_KANI_INFRA` per the existing matrix. The current "silent fallback to
generic reject" path is the worst of both worlds: it loses identity and
loses the failure.

### F-15 — **MAJOR** — `KaniHarness.file: String` declared but unused

`crates/titania-lanes/src/run_lane_kani.rs:54-56`

```rust
file: String,
#[expect(dead_code, reason = "informational; not yet surfaced in findings")]
```

`#[expect(dead_code, reason = "informational; not yet surfaced in findings")`
is a footnote saying "this field is never used, and we tolerate it". The
production-source rule "no annotations without reason" treats
`#[expect(dead_code)]` as an escape hatch with a *clear* reason. Here the reason
is "we'll wire it up later", which is **technical debt disguised as a lint
expectation**. Either thread `file` into `Finding::Location::source_text` (it's
in the spec §3 value-object list) or drop the field.

### F-16 — **MINOR** — Stringly typed `lane.tool_version` magic numbers

`crates/titania-lanes/src/run_lane_kani.rs:92`: `tool_version: "0.67.0".to_owned()`
`crates/titania-lanes/src/run_lane_mutants.rs:127`: `tool_version: "27.0.0".to_owned()`

Tool versions are invariants of the lane. A `const KANI_VERSION: &str` /
`const MUTANTS_VERSION: &str` declared next to the existing `KANI_TOOL`
constant at line 33 of `run_lane_kani.rs` is the obvious fix.

### F-17 — **MINOR** — `crate_entry` filters `"titania-dylint"` stringly

`crates/titania-lanes/src/run_lane_kani.rs:514`

`if pkg == "titania-dylint" { return None; }` is a magic-string branch on a
crate name. The Kani-vs-dylint exclusion is a domain rule (R6 in spec §12);
it should be a const `KANI_EXCLUDED_CRATE: CrateName` (newtype) or a typed
`is_kani_safe(crate: CrateName) -> bool`.

### F-18 — **MINOR** — `LaneRunState` accessed by raw fields across two files

`run_lane_kani.rs` and `run_lane_mutants.rs` each define their own
`LaneRunState`. Both carry `{exit_code: i32, harnesses/packages_run: usize,
tool_version: String}`. Should be a single `titania_core::LaneRunState` in
core, parameterised by tool, so its methods (`record_exit_code`,
`record_tool_version`, `record_run`) are testable in core.

### F-19 — **MINOR** — Reading stderr into `String` then forgetting lifetime

`crates/titania-lanes/src/run_lane_kani.rs:580-582` and `run_lane_mutants.rs:385-386`

```rust
let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
```

Per functional-rust's §performance.no_heap_collections and AGENTS.md §9
("hot paths avoid allocation, formatting, cloning"), the CBMC run produces
multi-MB stderr streams. These should use `Cow<'_, str>` for the lossless case
and avoid `.into_owned()` when `from_utf8_lossy` returns `Cow::Borrowed`. A
classic `match output.stderr { valid => Cow::Borrowed(valid), lossy => Cow::Owned(...) }`
makes the hot path zero-copy on UTF-8-clean CBMC output.

### F-20 — **MINOR** — `current_unix` silent clock-skew recovery

`crates/titania-lanes/src/run_lane_mutants.rs:471`

```rust
SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
```

On clock skew (pre-1970 system time), this returns `0`, which means
`now_unix <= exp` is `true` for every positive `exp` — *every* baseline entry
is treated as in-date. Use `tracing::warn!` to surface skew and either
(a) treat as a lane infra failure or (b) document the saturating-to-zero
behaviour explicitly in the contract.

### F-21 — **MINOR** — `baseline_path` chain-joins magic directory

`crates/titania-lanes/src/run_lane_mutants.rs:466-468`

`workspace_root.join(".titania").join("profiles").join("strict-ai").join(...)`
— the `.titania/profiles/strict-ai` path is an invariant of the repo. A
`BaselinePath` newtype or a const-declared `BaselineLocation` can replace the
four `.join`s with a single typed accessor.

### F-22 — **MINOR** — `aggregate.last_err: String` reimplements diagnostic accumulation

`crates/titania-lanes/src/run_lane_kani.rs:494-501`

```rust
struct InventoryAggregate {
    harnesses: Vec<KaniHarness>,
    any_listed: bool,
    last_err: String,
}
```

`last_err` is `format!("{pkg}: {reason}")` and lives only so the function can
return *one* failure string at the end. A `Result<Vec<KaniHarness>, KaniLaneError>`
that folds with `try_fold` does not need this — every failure is itself a typed
error value that the caller composes.

### F-23 — **MINOR** — `KaniHarness.file` populated but not surfaced

Same root cause as F-15; flagged separately because the underlying decision
gap matters: the contract's value-object list says each harness has a `file`,
so the contract intends the field to flow into findings. Either surface it as
`Location.source_text()` payload or commit to dropping it.

### F-24 — **MINOR** — `Cargo.toml` import path: lanes do not import any v1.5 newtype

A non-finding observation (see `grep` results above): `run_lane_kani.rs` and
`run_lane_mutants.rs` import the following from `titania_core`:
`CommandEvidence, Digest, Finding, Lane, LaneEvidence, LaneFailure, LaneOutcome,
Location, OutcomeError, ProcessTermination, RepairHint, RuleId, RuleIdError,
SkipReason, TargetProject`.

**None of**: `KaniHarnessId`, `MutantId`, `MutantOperator`, `MutantsBaseline`,
`MutantBaselineEntry`, `ToolKind`, `MutantIdError`, `KaniHarnessIdError`,
`MutantsBaselineError` is imported. This is the symptom of F-01, F-02, F-03,
F-04 — fixing those four simultaneously restores the `use` lines.

---

## Required Fixes

These are **blockers** (must be repaired before this contract lane is allowed
to advance from `tn-7bq2.3`/`tn-7bq2.4` to closure). All repair edits belong
under the corresponding bead; this audit only enumerates them.

| # | Required fix | Blocked bead |
|---|--------------|--------------|
| R-01 | Lane imports `MutantId`, `MutantOperator`; constructs every mutation id via `MutantId::new(...)?`; `MutantOperator::from_genre(&str) -> Result<MutantOperator, MutantIdError>` is exhaustive over cargo-mutants' documented genre vocabulary; `RawMutant.genre` is parsed through this mapper; the lane's idempotent `String` ids disappear. | tn-7bq2.4 |
| R-02 | Lane imports `MutantsBaseline`; deletes its private `Baseline`/`BaselineEntry` shadow types; calls `MutantsBaseline::load(&path)?`. `parse_baseline`, `load_baseline`, and the serde-`RawEntry` shim disappear. | tn-7bq2.4 |
| R-03 | `MutantsBaseline::diff` signature changes from `(&[String]) -> Vec<&String>` to `(&[MutantId]) -> Vec<MutantId>`. `MutantsBaseline::contains` becomes `(&MutantId, u64) -> bool`. | tn-7bq2.4 |
| R-04 | `KaniHarness.name: KaniHarnessId`; cargo-kani's JSON `name` field flows through `KaniHarnessId::new(...)?` at parse time; `KaniHarnessIdError` surfaces through `KaniLaneError::Parse`. | tn-7bq2.3 |
| R-05 | `MutantBaselineEntry::new(id: MutantId, accepted_by_rule: ValidatedRule, reason: String, expires: Option<Expiry>) -> Result<Self, MutantsBaselineError>` smart constructor; `from_bypasses` removed; serde `Deserialize` impl routes through `new`. | tn-7bq2.1 closing |
| R-06 | `Result<T, String>` → typed errors in both lanes. New variants per shell failure mode; `contains("no such subcommand")` becomes a typed `ToolMissing { tool: ToolKind }` decision. | tn-7bq2.3, tn-7bq2.4 |
| R-07 | `KaniLaneError`/`MutantsLaneError` gain `Internal { tool: ToolKind, reason: Box<str> }` (or wraps `OutcomeError`); the `format!("evidence build failed: {error}")` re-mint into `NotACargoWorkspace` / `BaselineMissing` is deleted. | tn-7bq2.3, tn-7bq2.4 |
| R-08 | `poll_kani_child` becomes a `try_fold`/`scan`/`take(N)` with explicit max-iteration cap and a `tracing::debug!` heartbeat. The two `for` loops in `run_lane_mutants.rs::outcome` and `findings_for_package` become `try_fold` / `collect::<Result<_, _>>()`. | tn-7bq2.3, tn-7bq2.4 |
| R-09 | `HarnessRun` becomes `enum HarnessRun { SpawnFailed {...}, WaitFailed {...}, TimedOut {...}, Completed {...} }`. `LaneRunState.exit_code: i32` becomes `enum ObservedExit { NotYetSeen, Exited { code: i32 }, Signaled { signal: i32 } }`. | tn-7bq2.3 |
| R-10 | `KaniHarnessId::new`, `MutantId::new`, `MutantsBaseline::load`, `MutantBaselineEntry::new` all carry `#[must_use = "..."]`. Closes the `let _ = KaniHarnessId::new(...)` silent-drop risk. | tn-7bq2.1 closing |
| R-11 | `PROOF_KANI_<NAME>` rule-id cap. Either (a) explicit hash-suffix truncation with documented scheme, or (b) any harness id longer than the 96-byte cap emits `PROOF_KANI_INFRA` per harness with the original name in the location, instead of silently falling back to `PROOF_KANI_FAIL`. | tn-7bq2.3 |

---

## Recommended Fixes

These are **improvements** under the same DDD doctrine but are not strictly
blocking. They can ride with the next lane revision or a `tn-7bq2.5`
follow-on bead.

| # | Recommended fix | Where |
|---|-----------------|-------|
| Q-01 | Promote `tool_version` strings to `const KANI_VERSION: &str = "0.67.0"` and `const MUTANTS_VERSION: &str = "27.0.0"`. | both lanes |
| Q-02 | Promote `"titania-dylint"` to a const or `CrateName::is_kani_compatible()` predicate. | run_lane_kani.rs:514 |
| Q-03 | Collapse the two `LaneRunState` definitions into a shared `titania_core::LaneRunState`. | lanes + core |
| Q-04 | Replace `String::from_utf8_lossy().into_owned()` with a `Cow<'_, str>`-carrying evidence chunk that avoids the heap allocation on the UTF-8-clean path. | both lanes |
| Q-05 | Add `tracing::warn!` on `current_unix` saturating-to-0 (or surface as infra finding via `MutantsLaneError`). | run_lane_mutants.rs:471 |
| Q-06 | Replace `verdict_from_line`'s `other.contains("UNSUPPORTED")` with an exhaustive enum over a frozen CBMC vocabulary; unknown values bucket to `Unknown` *only*, no substring contagion. | run_lane_kani.rs:287 |
| Q-07 | Surface `KaniHarness.file` as `Finding::Location.source_text()` or commit to dropping the field. The `#[expect(dead_code, reason = "informational; not yet surfaced in findings")` should not outlive v1.5. | run_lane_kani.rs:54 |
| Q-08 | Define `BaselinePath` / `BaselineLocation` newtypes; collapse the four `.join`s in `baseline_path`. | run_lane_mutants.rs:466 |
| Q-09 | Add `CrateName` newtype for crate basenames filtering; replaces `if pkg == "titania-dylint"` and `entry.file_name().to_string_lossy()`. | run_lane_kani.rs:513, run_lane_mutants.rs:341 |

---

## Compliant Patterns

These are the v1.5-region spots where the DDD/functional-rust doctrine is
honoured; preserve them across refactors and treat as ground truth for what a
"good" v1.6 lane addition would look like.

- **`proof_id.rs::KaniHarnessId::new` (`proof_id.rs:50`)** — total smart
  constructor that returns `Result<Self, KaniHarnessIdError>` with one variant
  per failure axis (Empty / TooLong / NoUnderscore / LeadingDigit /
  NotUpperAscii). Every predicate (`as_str`, `is_equal`) is named after the
  type, not the field. Display, FromStr, Serde (transparent + deserialize-routes-through-new)
  complete the parse-don't-validate circle at the boundary. The
  `check_khi`/`check_khi_shape`/`check_khi_chars`/`check_khi_char` chain is
  the canon: pure, total, typed. (Minor gap: missing `#[must_use]` — flagged
  as F-07.)

- **`proof_id.rs::MutantOperator` (`proof_id.rs:135-171`)** — totally
  enumerated enum (8 variants), `#[non_exhaustive]` absent, exhaustive
  `as_str` const fn. New operators require a contract amendment — the closed
  set is the contract surface.

- **`proof_id.rs::MutantId::new` (`proof_id.rs:193`)** — takes a typed
  `MutantOperator` enum argument, *not* a `&str`. This pushes string-vs-enum
  classification to the boundary; the smart constructor is total and
  pre-enumerated.

- **`proof_id.rs::ToolKind` (`proof_id.rs:267-285`)** — closed two-variant
  enum used downstream by `SkipReason::ToolUnavailable(ToolKind)`. Same
  shape as `MutantOperator`.

- **`mutants_baseline.rs::MutantsBaseline::load` (`mutants_baseline.rs:80`)** —
  boundary parser: read file → parse JSON → validate schema version →
  validate per-entry mutation id. Each step is its own helper
  (`read_contents`, `parse_baseline`, `validate_baseline`, `validate_entry`).
  Errors are typed (`MutantsBaselineError`) with `Box<str>` reasons to keep
  variant sizes small. (The `from_bypasses` companion is the gap — flagged
  as F-06.)

- **`mutants_baseline.rs::read_contents` + `parse_baseline` + `validate_baseline`** —
  the *Data / Calculations / Actions* split applied: `read_contents` is
  Actions (I/O), `parse_baseline` is the boundary, `validate_baseline` is
  pure Calculations over the parsed Data. No hidden state. Each helper is
  ≤ 15 lines.

- **`mutants_baseline.rs::validate_baseline` (`mutants_baseline.rs:150-162`)** —
  uses `iter().try_for_each(|entry| validate_entry(...))` rather than an
  imperative `for entry in ... { match ... }`. Iterator pipeline correctly
  applied.

- **`mutants_baseline.rs::validate_entry` (`mutants_baseline.rs:166-178`)** —
  early-returns on the predicate failure; no nested `if`s; flat. Pure. The
  Box<str> reason pattern keeps `Err` payload sized.

- **`run_lane_kani.rs::gather_findings`** (`run_lane_kani.rs:145-157`) —
  `inventory.iter().try_fold(Vec::new(), |mut acc, harness| {...})` —
  iterator pipeline with intermediate `Vec::new()` accumulator (acceptable
  here because the synthesis is then re-marshalled into
  `LaneOutcome::Findings`; not a hot path).

- **`run_lane_kani.rs::harnesses_from_file_map`** (`run_lane_kani.rs:618-632`) —
  `file_map.iter().flat_map(...).collect()`. Pure, zero-mut, predictable.

- **`run_lane_kani.rs::parse_verdict`** (`run_lane_kani.rs:276-281`) —
  `stdout.lines().find_map(verdict_from_line).map_or(...)`. The flat
  `find_map`-then-default pipeline is exactly what functional-rust
  prescribes for "first match wins over a stream".

- **`run_lane_kani.rs::FALLBACK_RULE_ID` static** (`run_lane_kani.rs:70-75`) —
  `LazyLock<Result<RuleId, _>>` initialised once; the lane matches/uses a
  typed error path if the literal is unrecoverable. (The match arm at line
  73 `(_, Ok(_)) =>` could be unified with the kani lane's variant at line
  107 — drift between the two lanes is itself the F-15 sibling; fixing R-08
  also cleans this.)

- **`proof_id.rs::check_khi_chars` (`proof_id.rs:116-118`)** —
  `s.as_bytes().iter().enumerate().try_for_each(...)`. Pure, no mutation,
  no panic, no `unwrap`. The doctrine exemplum.

- **File headers both files (`mutants_baseline.rs:1`, `proof_id.rs:1-12`,
  `run_lane_kani.rs:1-12`, `run_lane_mutants.rs:1-12`)** — every new file
  has a Rust doc preamble tying it back to the contract (`§3 value
  objects`, `§5 boundaries`, `§7 skip-state`) before any `pub fn`. The
  doc-paragraph-to-evidence trace is what makes the contract reviewable
  on this PR.

- **Test surface in `v15_kani_harness_id.rs`, `v15_mutant_id.rs`,
  `v15_mutants_baseline_load.rs`, `v15_kani_harness_id_serde.rs`** —
  exact assertions (no `is_ok()`-only), one rejection axis per test, full
  happy-path coverage with the same shape as production. The core tests
  *are* the contract; the lanes need equivalent exact-assertion tests for
  the shell side.

---

## Summary

The clean v1.5 work is the core newtypes (`KaniHarnessId`, `MutantId`,
`MutantOperator`, `ToolKind`) and the schema-validated baseline
(`MutantsBaseline::load`). The dirty v1.5 work is the lanes: they ship
alongside the contract but never consume the contract's types, instead
doubling the type count with stringly-typed parallel implementations.
Combined with `Result<T, String>` shell signatures, an unannotated
`MutantBaselineEntry.mutation_id: String` field, and a `loop { try_wait }`
whose fallback hides loss of identity for long harness names, the lanes
read as a half-migration: typed core, untyped shell.

Four fixes (R-01, R-02, R-03, R-04) invert the type universe back to the
contract. Two fixes (R-06, R-07) realign the error surface. One fix (R-08)
restores the iterator discipline. One fix (R-09) restores the typestate
discipline. One fix (R-10) restores the must-use discipline. One fix
(R-11) prevents a silent rule-id collision that masks per-harness identity.

After R-01 through R-11 land and the recommended fixes (Q-01 through Q-09)
ride with the next lane revision, v1.5 will be in a state where removing
every stringly-typed domain name (`name: String`, `mutation_id: String`,
`tool_version: String` only at the call site, `last_err: String`) leaves
a clean DDD-typed surface.

This audit is **non-edit**: no source files are modified by this report.
Sign-off requires the listed beads to drive R-01..R-11 (and a subset of
Q-01..Q-09 if reviewer availability allows).
