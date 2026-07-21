# Black Hat Review — v1.5 Kani + Mutants + Full scope migration

```
Bead: tn-7bq2.2
State: 13 (gate before landing)
Reviewer: black-hat-reviewer
Source checkout: /home/lewis/src/titania
Attempt: 1
Review date: 2026-07-16
Prior reviews consulted: .evidence/v1.5/holzman-rust-review.md, .evidence/v1.5/truth-serum-audit.md, .evidence/v1.5/red-queen-review.md, .beads/tn-7bq2.2/black-hat-review.md (prior APPROVED), .beads/tn-7bq2/paper-laundered/black-hat-review.md
```

## Gate Result

**STATUS: REJECTED**

The v1.5 migration contains **one critical correctness defect** (mutants lane runs
`--list` discovery but emits `MUTANT_SURVIVED` findings, producing 2827 false-positive
rejects), **multiple spec-deviations** at the lane/CLI/Moon layer, **two dead rule-id
families** in the explain catalog (`PROOF_KANI_NOT_RUN` / `MUTANT_BASELINE_MISSING`
are documented but never emitted), **paper-only evidence** for 7 of 18 verification-ledger
obligations, and **a hoisted `#[non_exhaustive]` violation** on `GateScope` that D1
explicitly forbids. The aggregate `--scope full` path now produces a real
`Reject` with 12 per-lane entries (verified live), and the parser gap that previously
blocked `titania-check run-lane kani|mutants` is repaired. The core domain types
(`KaniHarnessId`, `MutantId`, `MutantsBaseline`, `MutantOperator`, `ToolKind`) and their
associated tests are clean. But the lane implementations deviate from spec §4 in ways
that invert the meaning of every downstream finding, and the previous `.beads/tn-7bq2.2/
black-hat-review.md` (STATUS: APPROVED) is itself a paper artefact that cites
non-existent files (`crates/titania-core/src/kani_inventory.rs`,
`crates/titania-core/src/mutants_outcomes.rs`, `src/kani.rs::kani_kani_harness_id_bounded`)
and must be retracted.

---

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|-------------|--------|----------|
| `Lane::Kani` / `Lane::Mutants` total enum (D1) | ✅ | `crates/titania-core/src/lane.rs:42-46` declares both; `name()`, `file_stem()`, `FromStr` all extended. Test `v15_lane_roundtrip.rs` covers round-trip. |
| `GateScope::Full` total enum (D1) | ⚠️ struct-violates | `crates/titania-core/src/gate_scope.rs:21` retains `#[non_exhaustive]` pre-v1.5 attribute. D1 says "No `#[non_exhaustive]` loosening". Variant added without removing the loosened posture. `GateScope::lanes()` returns `FULL_LANES` for `Self::Full`. |
| `PROOF_KANI_*` rule family | ❌ partial | Catalog + explain cover 6 ids (`PASS`/`FAIL`/`BLOCKED`/`NOT_RUN`/`UNSUPPORTED`/`INFRA`). Only `PROOF_KANI_BLOCKED` is emitted in live runs; `PROOF_KANI_NOT_RUN` and `PROOF_KANI_PASS` are dead (see F-08, F-09). |
| `MUTANT_SURVIVED` rule family | ❌ partial | `MUTANT_SURVIVED_INFRA` and `MUTANT_BASELINE_MISSING` are cataloged but `MUTANT_BASELINE_MISSING` is never emitted (lane returns `Err` instead — see F-10). |
| `titania-check --scope full` (A8) | ✅ verified live | `cargo run --frozen --quiet -p titania-check -- aggregate --scope full --emit json` exits 1 with `Reject`, 12 per_lane (Fmt, Compile, Clippy, AstGrep, Dylint, PanicScan, PolicyScan, Test, Deny, Build, Kani, Mutants), 2835 code_findings, 10 gate_failures. Live run reproduced this audit. |
| `titania-check run-lane kani` (parser fix) | ✅ verified | `crates/titania-check/src/args/parse.rs:470-471` adds `"kani" => Ok(Lane::Kani)` and `"mutants" => Ok(Lane::Mutants)`. Live run: `8 finding(s)`, exit 1. |
| `titania-check run-lane mutants` (parser fix) | ✅ verified | Live run: `2824 finding(s)` (one less than the `aggregate-full.json` snapshot's 2827 — see F-12). |
| `scripts/dev/mutants-bootstrap.sh` | ✅ exists | `scripts/dev/mutants-bootstrap.sh` present (17.5K, executable). Reads full-test mode `cargo mutants --no-shuffle --output mutants.out -p <pkg>` and parses `outcomes.json` for `MissedMutant`. Spec §4.4 conformance verified by inspection. |
| Per-harness rule_id emits `PROOF_KANI_<NAME>` (D6) | ⚠️ partial | `pass_finding`/`fail_finding` produce `PROOF_KANI_<normalized_name>` (e.g. `PROOF_KANI_LANE_DIGEST_ACCEPTS_PASSED_NOT_GREATER_THAN_SCANNED`) — works for short names, but collapses to `PROOF_KANI_FAIL` for any name whose normalized form exceeds 96 chars (see F-14). |
| `Lane::Kani | Lane::Mutants` only run in `GateScope::Full` (D2) | ✅ | `crates/titania-lanes/src/run_lane_outcome.rs:166-169` const array: `Lane::Kani | Lane::Mutants => FULL_ONLY`. |
| `titania-kani` and `titania-mutants` Moon tasks | ✅ | `.moon/tasks/all.yml:320, 339` declares both with `command: 'cargo run --frozen --quiet -p titania-check -- run-lane <kani|mutants>'`. |
| `:titania:gate-full` composite | ⚠️ partial | `.moon/tasks/all.yml:376-382` declares composite with deps `[gate-release, titania-kani, titania-mutants]`. **Cannot be exercised end-to-end**: prior captured log `.evidence/v1.5/raw/moon-gate-full.log` shows failure at `titania-policy-scan`, but the policy-scan task predates v1.5. |
| `crates/*/kani-list.json` regeneration per run | ✅ verified | `crates/titania-core/kani-list.json` regenerated each run; 8 harnesses recorded. Other 5 crates produce zero-harness inventories. |
| Baseline bootstrap recipe (D3) | ⚠️ partial | `scripts/dev/mutants-bootstrap.sh` exists and is correct in isolation. The lane does NOT consume the baseline test-mode output — it uses `cargo mutants --list` discovery and emits `MUTANT_SURVIVED` for every listed mutation not in baseline (see F-01). |
| 9 production match sites updated | ✅ partial | All 9 listed sites have new arms. **But** `Lane` keeps `#[non_exhaustive]` posture? No — `Lane` does NOT have `#[non_exhaustive]` (lane.rs:20). Only `GateScope` does. The two are inconsistent. |
| Cgroup cap (`MemoryMax=24G`, `MemorySwapMax=0`) | ❌ | No Rust code or Moon task wrapper invokes `systemd-run --user --scope -p MemoryMax=24G -p MemorySwapMax=0`. The Kani lane has only a wallclock cap (60s). The mutants lane has no cap at all. |
| `titania-kani` and `titania-mutants` blocked cgroup cgroup OOM detected | ❌ | `PROOF_KANI_BLOCKED` repair hint references "cgroup OOM" but no cgroup is invoked. No signal-handler parsing. |

---

## PHASE 2: Farley Engineering Rigor

| Function | File:Line | Lines | Limit (60) | Status |
|----------|-----------|-------|------------|--------|
| `outcome` (Kani lane) | `run_lane_kani.rs:87-108` | 22 | 60 | ✅ |
| `gather_findings` (Kani) | `run_lane_kani.rs:145-157` | 13 | 60 | ✅ |
| `poll_kani_child` | `run_lane_kani.rs:667-674` | 8 | 60 | ✅ |
| `list_kani_harnesses` (Kani) | `run_lane_kani.rs:470-489` | 20 | 60 | ✅ |
| `check_khi_chars` (proof_id) | `proof_id.rs:116-118` | 3 | 60 | ✅ |
| `parse_baseline` (mutants lane) | `run_lane_mutants.rs:210-242` | 33 | 60 | ✅ |
| `outcome` (mutants lane) | `run_lane_mutants.rs:120-142` | 23 | 60 | ✅ |
| `MutantsBaseline::load` | `mutants_baseline.rs:80-86` | 7 | 60 | ✅ |

All functions are ≤60 lines. No `for`/`while`/`loop` in core (verified by inspection —
only in shell `run_lane_kani.rs:668-674` for the child wait poll, which is a bounded
service containment loop, justified). Mutability is narrow (`LaneRunState` accumulators
only; baseline and inventory builders pass `&mut` through fold helpers).

**Farley violations**: none in source shape. **Hard-coupling violations** are everywhere
(F-01 through F-04 below).

---

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status | Notes |
|------|--------|-------|
| Zero `unsafe` | ✅ | grep-confirmed across `proof_id.rs`, `mutants_baseline.rs`, `run_lane_kani.rs`, `run_lane_mutants.rs`. `fuzz_targets/fuzz_parse_inventory.rs` has one `unsafe { slice::from_raw_parts(...) }` block with a SAFETY comment, gated on `#![no_main]`. Outside v1.5 scope. |
| Zero `.unwrap()`/`.expect()` in production | ✅ | grep-confirmed in the four files. `FALLBACK_RULE_ID` static uses `LazyLock` to defer `RuleId::new` validation (acceptable). |
| Zero `panic!`/`todo!`/`dbg!` | ✅ | grep-confirmed. |
| Checked arithmetic | ✅ | `state.harnesses_run.saturating_add(...)` (run_lane_kani.rs:161); `state.packages_run.saturating_add(...)` (run_lane_mutants.rs:391). No unchecked division/modulo. |
| All errors via `thiserror` enums in core | ✅ | `KaniHarnessIdError`, `MutantIdError`, `MutantsBaselineError`, `KaniLaneError`, `MutantsLaneError` all `#[derive(thiserror::Error)]`. Core never returns `Result<T, String>`. |
| Two `drop(fallible_call(...))` violations | ❌ F-06 | `run_lane_kani.rs:209` `drop(pipe.read_to_string(&mut buf))`; `run_lane_kani.rs:571` `drop(std::fs::remove_file(&artifact))`. Both swallow non-`NotFound` errors silently. AGENTS.md rule 4 violation. |
| Parse-don't-validate | ✅ for core; ❌ for lane (F-07) | `MutantsBaseline::load` validates on load. The Kani lane's `parse_kani_list` (`run_lane_kani.rs:603-612`) downgrades parse failures to empty Vec. The mutants lane's `parse_mutants_list` (`:403-410`) does the same. |
| Newtypes for primitives | ✅ | `KaniHarnessId(String)`, `MutantId(String)`, `MutantOperator` (closed enum), `ToolKind` (closed enum). |
| No boolean parameters / no `Option`-based state machines | ✅ | No boolean flags. `LaneOutcome` is a closed sum type (`Clean`/`Findings`/`Failed`/`Skipped`). |

**Holzman-style violations**: F-06, F-07, F-15 (relaxed parsing), F-23 (closed-set
violated: `PROBLEM` substring match on UNSUPPORTED verdict at `run_lane_kani.rs:294`).

---

## PHASE 4: Ruthless Simplicity & DDD (Scott Wlaschin)

| Check | Status |
|-------|--------|
| No Option-based state machines | ✅ |
| CUPID composable | ⚠️ — `LazyLock<Result<RuleId, RuleIdError>>` (F-08) is the opposite of composable: every consumer must `.as_ref().expect(...)`. Replace with `OnceLock<RuleId>`. |
| CUPID predictable | ❌ — `FALLBACK_RULE_ID` in `run_lane_mutants.rs:104-110` pairs BOTH slots with `MUTANT_SURVIVED` (and the third arm of `run_lane_kani.rs:71-75` has a latent bug where successful secondary is dropped). |
| CUPID idiomatic | ⚠️ — `run_lane_mutants.rs:459-463` constructs a `MutantId` via `format!()` directly, bypassing the newtype's invariant. The whole point of `MutantId::new`'s validation (line/col non-zero, package non-empty, path not absolute) is killed. |
| CUPID domain-based | ⚠️ — `ToolKind` (`proof_id.rs:269-284`) is defined but not consumed by any lane code. Dead API. |
| No clever abstractions | ✅ for typed newtypes. ❌ for the `LazyLock<Result<RuleId, RuleIdError>>` chain (a workaround for the typed `Result::Ok` fallback pattern that the spec doesn't actually require). |
| Parse-don't-validate | ❌ for the mutants lane's custom loader (`run_lane_mutants.rs:210-242`). It accepts `mutation_id = "*"` (F-13) — a hostile baseline that suppresses ALL MUTANT_SURVIVED findings. |

---

## PHASE 5: The Bitter Truth

The v1.5 migration is a **shape-faithful, behavior-unfaithful** port. The Cargo
workspace has grown the right total enums (`Lane::Kani`/`Mutants`, `GateScope::Full`),
the right typed newtypes (`KaniHarnessId`, `MutantId`, `MutantsBaseline`), the right
Moon task wiring (`titania-kani`, `titania-mutants`, `gate-full`), the right explain
catalog rows, and the right CLI parser arms. The aggregate `--scope full` flow runs
end-to-end and emits a real typed report. The four cargo gates (`fmt`, `check`,
`clippy`, `test`) all exit 0 against the source.

But the **two v1.5 lanes do not implement their spec**:

1. The **Kani lane** runs one cargo-kani subprocess per harness instead of one per
   package as the spec requires. This adds N−1 redundant build passes, loses the
   `systemd-run` cgroup cap that R1/R6 require for OOM containment, and the per-harness
   `PROOF_KANI_BLOCKED` repair hint textually lies about a "cgroup OOM" cause that
   the lane cannot detect. The 60s wallclock timeout is a service containment cap,
   not a Power-of-Ten Rule-2 static bound proof.

2. The **Mutants lane** runs `cargo mutants --list` discovery mode, NOT the spec-mandated
   `cargo mutants --no-shuffle --json -o <run-dir>` full test-mode invocation. It then
   treats every listed mutation (regardless of whether it was actually applied and
   tested) as a `MUTANT_SURVIVED` finding. With an empty baseline, that produces 2827
   false-positive `MUTANT_SURVIVED` rejects for mutations that were never exercised.
   The bootstrap script (`scripts/dev/mutants-bootstrap.sh`) does run full test mode
   correctly — the lane and the bootstrap use different commands and the lane's
   output is uncorrelated with the bootstrap's `outcomes.json` `MissedMutant` filter.

The **three rule-ids in the explain catalog that the contract says must be emitted
are dead**: `PROOF_KANI_NOT_RUN` (catalog line 75) is documented but the lane uses
`LaneOutcome::Skipped { reason: SkipReason::NotApplicable }` instead; `PROOF_KANI_PASS`
(catalog line 72) is referenced only as a fallback when the per-harness rule_id
cannot be parsed — every harness in the workspace currently falls in this branch
for the 8 source-side harnesses, but none of them have actually been verified as
PASS because every emit is BLOCKED via the wallclock cap; `MUTANT_BASELINE_MISSING`
(catalog line 80) is never emitted — the lane returns `Err(MutantsLaneError::
BaselineMissing(...))` and the dispatcher exits 4 (`LaneExit::Failure`).

The **prior black-hat review in `.beads/tn-7bq2.2/black-hat-review.md`** that
records `STATUS: APPROVED` is itself a paper artefact: its proof/test/source parity
matrix cites `crates/titania-core/src/kani_inventory.rs`, `crates/titania-core/src/
mutants_outcomes.rs`, `fuzz_targets/fuzz_parse_inventory.rs`, `fuzz_targets/
fuzz_parse_outcomes.rs`, and `src/kani.rs::kani_kani_harness_id_bounded`. None of
these files exist. The actual locations are `crates/titania-core/src/proof_id.rs`,
`crates/titania-core/src/mutants_baseline.rs`, `crates/titania-lanes/src/run_lane_kani.rs`,
`crates/titania-lanes/src/run_lane_mutants.rs`, `fuzz/fuzz_targets/fuzz_parse_inventory.rs`,
`fuzz/fuzz_targets/fuzz_parse_outcomes.rs`. The prior approval must be retracted.

The **evidence files** in `.evidence/v1.5/raw/exec-*.txt` are 70-90 byte stubs
recording `verifier=<name>` and `exit=0` literals — 18 entries, all the same shape,
no actual stdout/stderr/test-result/test-output captured. This is exactly the
evidence-laundering pattern `.beads/tn-6hyc` warned about. The truth-serum-audit
already flagged 7/18 of these obligations as paper-only (LED-004 / 007 / 010 /
015 / 016 / 017 / 018 reference non-existent harnesses, a non-existent `cargo verus
--verify-fn` subcommand, a loom source file that self-declares compile-only, and
`cargo fuzz` invocations that fail because `fuzz/Cargo.toml` lacks
`[package.metadata] cargo-fuzz = true`). The current review re-confirms every
finding the truth-serum-audit raised and adds the runtime correctness defects
that audit did not exercise (the v15 lane behavioural defects F-01 through F-04).

The aggregate `--scope full` exit is **now exit 1 (Reject)**, not the exit 3
(`InputError`) that the prior v1.5 capture preserved in `.evidence/v1.5/raw/
aggregate-full-mutants.log`. The validator fix is real. The lane outputs
(`.titania/out/full/kani.json`, `.titania/out/full/mutants.json`) are populated
with typed `LaneOutcome::Findings` payloads. The parser extension at `args/parse.rs:
470-471` is in place. The CLI dispatch in `run_lane::non_cargo_outcome` reaches
`kani_outcome` and `mutants_outcome`. The composition is wired end-to-end at the
orchestration level.

But the **content** of the lane outputs is wrong: 8 BLOCKED findings where 8 PASS
findings should be (per A2), 2827 MUTANT_SURVIVED findings for unverified mutations
(F-01). The container is correct; the payload is fabricated.

---

## Findings (Ordered by Severity)

| # | Finding | Severity | File:Line | Status |
|---|---------|----------|-----------|--------|
| F-01 | Mutants lane uses `cargo mutants --list --json` discovery mode but emits `MUTANT_SURVIVED` findings for every listed mutation. Spec §4.3 step 3 requires **full test-mode** (`cargo mutants --no-shuffle -o <dir> --json`), then parse `outcomes.json` for `summary == "MissedMutant"` and `mutants.json` for per-mutant entries. Lane behavior is "discovered not in baseline ⇒ reject", which with an empty baseline (D3) covers ALL mutations, not actual test-survivors. | **CRITICAL** | `run_lane_mutants.rs:378`, `run_lane_mutants.rs:172-183` | open |
| F-02 | Kani lane spawns one cargo-kani subprocess per harness (`run_kani_child` at `run_lane_kani.rs:635-664`); spec §4.2 step 4 requires **one cargo-kani per package** (`cargo kani -p <pkg> --output-format=regular`), then parse the combined output for `VERIFICATION:` lines. Per-harness invocation pays N−1 redundant build passes; loses the spec-mandated cgroup scope; wallclock cost = `harness_count × 60s + build_time`. | **CRITICAL** | `run_lane_kani.rs:651-664` | open |
| F-03 | No cgroup enforcement anywhere in Rust lane code. Spec §4.2 step 4 / R1 / R6 mandate `systemd-run --user --scope -p MemoryMax=24G -p MemorySwapMax=0 …`. Mutants lane also lacks cgroup (R2/R3 mandate it for the full test-mode run). The `PROOF_KANI_BLOCKED` repair hint textually references "cgroup OOM or wallclock timeout" but no cgroup is invoked and no signal-handler parsing detects one. Hidden Moon-layer coupling is undocumented. | **CRITICAL** | `run_lane_kani.rs:30, 188-192, 366-381`, `run_lane_mutants.rs:370-396` | open |
| F-04 | `PROOF_KANI_NOT_RUN` is documented in the repair catalog (line 75) and explain.rs (line 121) but NEVER emitted by the Kani lane. Spec §4.2 step 5: "missing cargo-kani → `PROOF_KANI_NOT_RUN` and lane disposition `NotApplicable`". Implementation: `LaneOutcome::Skipped { reason: SkipReason::NotApplicable }` with no finding. Catalog and explain are dead text. | **CRITICAL** | `run_lane_kani.rs:120-126`, `repair_catalog.tsv:75`, `explain.rs:121-123` | open |
| F-05 | `MUTANT_BASELINE_MISSING` is documented (catalog line 80, explain.rs:195) but NEVER emitted. Spec §4.3 step 2: "If absent, the lane FAILs with `MUTANT_BASELINE_MISSING` and prompts the operator to run the bootstrap recipe". Implementation: `run_lane_mutants::outcome` returns `Err(MutantsLaneError::BaselineMissing(...))`, dispatcher maps to `LaneExit::Failure` (exit 4). No `MUTANT_BASELINE_MISSING` finding is ever observed by an aggregator. | **CRITICAL** | `run_lane_mutants.rs:120-142, 192-202`, `repair_catalog.tsv:80`, `explain.rs:195-197` | open |
| F-06 | Two `drop(fallible_call(...))` violations swallow non-`NotFound` errors silently. AGENTS.md rule 4 forbids swallowed errors. | **CRITICAL** | `run_lane_kani.rs:209` (`drop(pipe.read_to_string(...))`), `run_lane_kani.rs:571` (`drop(std::fs::remove_file(...))`) | open |
| F-07 | JSON parse failures are silently downgraded to empty results. A malformed `kani-list.json` or a malformed cargo-mutants `--list --json` output produces a passing clean outcome for that crate/package, hiding the parse error entirely. | **CRITICAL** | `run_lane_kani.rs:604-606`, `run_lane_mutants.rs:404-407` | open |
| F-08 | `FALLBACK_RULE_ID` static has a latent match-arm bug (third arm discards a successful secondary id). Currently dormant because both literals are well-formed, but any future tightening of the rule-id grammar (e.g. adding a length cap, a charset restriction) will silently turn every Kani finding into `PROOF_KANI_FAIL`. The mutants lane's `FALLBACK_RULE_ID` pairs BOTH slots with `MUTANT_SURVIVED`, which is wrong: Kani lane should not produce `MUTANT_SURVIVED` as a fallback, and mutants lane should not produce `MUTANT_SURVIVED` as the fallback for `PROOF_KANI_*` ids. | **HIGH** | `run_lane_kani.rs:70-75`, `run_lane_mutants.rs:104-110` | open |
| F-09 | `PROOF_KANI_PASS` is documented (catalog line 72) but is only used as the fallback when the per-harness rule_id cannot be parsed. In every observed live run, all 8 emits are `PROOF_KANI_BLOCKED` (timeout), not `PROOF_KANI_PASS`. A2 acceptance ("Each Kani harness exit `VERIFICATION:- SUCCESSFUL`") is unmet because no harness ever verifies in this environment. The fallback path is dead. | **HIGH** | `run_lane_kani.rs:326-340`, `repair_catalog.tsv:72` | open |
| F-10 | Hostile baseline bypass: `run_lane_mutants.rs:60-65` `BaselineEntry::matches` accepts `mutation_id == "*"` as a wildcard that suppresses EVERY mutant finding for as long as the baseline file is present. The typed `MutantsBaseline` (which the core would use) does not validate wildcard ids, but the lane's custom loader (`run_lane_mutants.rs:210-242`) accepts them verbatim — and the lane's loader also does NOT validate mutation_id shape, so `mutation_id = "anything; rm -rf /"` would parse and persist in the typed `MutantsBaseline::load` round-trip (which DOES validate empty-string rejection but doesn't constrain other shapes). Combined: a hand-edited baseline file can fully bypass the lane. | **CRITICAL** | `run_lane_mutants.rs:60-65, 210-242`, `mutants_baseline.rs:166-178` | open |
| F-11 | Spec §7 SkipReason table: `ToolUnavailable` is the documented variant for `cargo-kani` missing or `< 0.50.0`. Implementation defines `SkipReason` with variants `PriorCompilationFailure`, `NotSelectedByScope`, `NotApplicable`, `PolicyDisabled` (`outcome.rs:17-26`); no `ToolUnavailable`. The `ToolKind` newtype (`proof_id.rs:269-284`) was added specifically to be the payload of a `SkipReason::ToolUnavailable(ToolKind)` variant, but the variant was never added. The doc comment at `proof_id.rs:263` references a non-existent variant. The `v15_skip_reason_tool_unavailable.rs` test is misnamed — it tests `SkipReason::NotApplicable` round-trip, NOT `SkipReason::ToolUnavailable`. | **CRITICAL** | `outcome.rs:17-26`, `proof_id.rs:263-284`, `tests/v15_skip_reason_tool_unavailable.rs` | open |
| F-12 | Substring match `"not found"` in `list_error_outcome` (`run_lane_kani.rs:121`) is too lax. Real cases that contain `"not found"` include: `read <crate>/kani-list.json: not found` (file-missing, not tool-missing), `kani: not found` (PATH lookup), `error: no such subcommand: kani`. The lane would classify a missing `kani-list.json` (which means cargo-kani wrote nothing because it errored out, not because cargo-kani is missing) as `Skipped { NotApplicable }`, hiding the actual cause. | **HIGH** | `run_lane_kani.rs:120-126` | open |
| F-13 | Hardcoded tool version `0.67.0` (kani) and `27.0.0` (mutants) is duplicated 5× in `run_lane_kani.rs` (lines 92, 333, 353, 371, 392) and 2× in `run_lane_mutants.rs` (127, 261). The actual runtime version is never queried via `cargo kani --version` / `cargo mutants --version`. The `Location::tool(name, version)` payload lies whenever the runtime differs. Spec §7 says version `< 0.50.0` should trigger `ToolUnavailable` — but no version check exists; the lane blindly trusts the hard-coded literal. | **HIGH** | `run_lane_kani.rs:92, 333, 353, 371, 392`, `run_lane_mutants.rs:127, 261` | open |
| F-14 | Per-harness rule_id fallback collapses to `PROOF_KANI_FAIL` / `MUTANT_SURVIVED` (whichever the per-finding helper reaches first) when the normalized harness/mutation name exceeds `RuleId::MAX_LEN = 96`. cargo-kani produces names like `kani::lane_digest_accepts_passed_not_greater_than_scanned` (52 chars, fits), but cargo-mutants produces names like `crates/titania-core/src/artifact.rs:59:9: replace <impl Serialize for ArtifactOutcome>::serialize -> Result<S::Ok, S::Error> with Ok(Default::default())` (140+ chars). After prepending `MUTANT_SURVIVED_` and normalizing to underscores, this is ~250 chars; the `RuleId::new` call rejects with `TooLong` and the lane falls back to `MUTANT_SURVIVED`. ALL 2827 MUTANT_SURVIVED findings in the live artifact share the SAME `rule_id`. Spec §3 acknowledges this collapse ("single rule id covering both individual mutations and aggregated lane outcomes") but the location is `Location::Tool("cargo-mutants", "27.0.0")` — no `file:line:col` payload. The "location carries per-mutation identity" claim is unmet; the file/line is in the `message` string but not in `Location`. | **HIGH** | `run_lane_mutants.rs:251-268, 309-311`, `run_lane_kani.rs:326-340, 366-381, 444-451` | open |
| F-15 | `PROOF_KANI_UNSUPPORTED` is classified by substring match `other.contains("UNSUPPORTED")` (`run_lane_kani.rs:294`). A verification line that contains "UNSUPPORTED" inside an unrelated token (e.g. `VERIFICATION: PARTIALLY_UNSUPPORTED_FEATURE`) will be classified as `Unsupported`. Should be exact match against the closed CBMC verdict set. | **MEDIUM** | `run_lane_kani.rs:287-297` | open |
| F-16 | `drain_pipe` returns `String::new()` for both "no pipe captured" (`None` branch) and "pipe empty" (`Some(pipe)` branch where `read_to_string` returned 0 bytes). The operator cannot distinguish "child closed stdout early" from "child had no stdout". Both surface as `parse_verdict` returning `Unknown` → `unknown_verdict_finding` → `PROOF_KANI_INFRA` finding. Diagnostic ambiguity. | **MEDIUM** | `run_lane_kani.rs:204-211` | open |
| F-17 | `ToolKind` (`proof_id.rs:269-284`) is defined and exported but never consumed by any v1.5 lane code. Dead public API. `crates/titania-core/src/lib.rs` re-exports it (verified via grep). Maintenance hazard: future readers assume `ToolKind` is wired into `SkipReason::ToolUnavailable`, which is doubly wrong (the variant doesn't exist, see F-11). | **MEDIUM** | `proof_id.rs:269-284` | open |
| F-18 | `wait-timeout = "0.2.1"` is declared in `crates/titania-lanes/Cargo.toml:22` but never imported anywhere. `cargo machete` will flag it on a strict run (verified live: `libfuzzer-sys` is flagged in `fuzz/Cargo.toml`; `wait-timeout` is not flagged because it appears under `[dependencies]` but the consumer code in `run_lane_kani.rs` rolls its own poll loop instead of using `wait_timeout::ChildExt` which `moon.rs:214` does use). | **MEDIUM** | `crates/titania-lanes/Cargo.toml:22` | open |
| F-19 | MutantIds are built via raw `format!()` in `run_lane_mutants.rs:459-463` (`build_mutant_id`), bypassing the `MutantId::new` newtype. The newtype enforces: package non-empty, path non-empty, line ≥ 1, col ≥ 1, no absolute path. The lane bypasses all five invariants by writing `line = 0`, `col = 0` when cargo-mutants emits a mutant without a span (`run_lane_mutants.rs:460-461`: `m.span.as_ref().and_then(|s| s.start.as_ref()).map_or(0, |p| p.line)`). Result: the lane's "mutation_id" string is NOT a valid `MutantId` and would be rejected by `MutantId::new`. The lane treats these ids as `String` everywhere downstream. | **CRITICAL** | `run_lane_mutants.rs:403-463` | open |
| F-20 | `run_lane_mutants.rs:319-338` `build_clean_outcome` records `CommandEvidence` argv as `["cargo-mutants", "mutants", "--baseline", ".titania/... (N pkgs)"]`. This is **not what was run**. The actual command is per-package `cargo mutants --list --json --no-shuffle -p <pkg>`. Receipt auditors reading the artifact will see a command that never executed. Either record each per-package invocation as its own `CommandEvidence` or record the workspace-level wrapper. | **HIGH** | `run_lane_mutants.rs:319-338` | open |
| F-21 | `cargo kani list --format json` is silently accepted by cargo-kani 0.67.0 but ignored — the JSON is written to `<crate_dir>/kani-list.json` on disk (verified live: stdout is human banner, file is `<crate>/kani-list.json`). The lane correctly reads the file. But the `--format json` argument is documentary dead weight — its presence or absence has no effect on cargo-kani's behavior. | **LOW** | `run_lane_kani.rs:573-580`, `cargo kani list --help` | open |
| F-22 | `crate_entry` (`run_lane_kani.rs:505-518`) hardcodes exclusion of `titania-dylint` only. Any other crate that injects a rustc_driver that conflicts with cargo-kani would silently crash the lane. Mitigation is per-crate enumeration + best-effort error capture, but the hardcoded exclusion is brittle. | **MEDIUM** | `run_lane_kani.rs:505-518` | open |
| F-23 | `crates/titania-core/kani-list.json` is regenerated every lane run (cleaned before re-running `cargo kani list` via `drop(std::fs::remove_file(&artifact))` at `run_lane_kani.rs:571`). It is NOT committed to the repo (manifest.toml line 16: `crates/titania-core/kani-list.json (NOT COMMITTED — see raw/ artifact)`). The other 5 crates' `kani-list.json` files **also do not exist** — verified live (`ls crates/titania-{lanes,check,aggregate,policy,output}/kani-list.json` returns empty). The manifest's `[kani].per_crate_inventory` lists 6 paths; only `titania-core/kani-list.json` exists. **The other 5 paths in the manifest are fictitious** — the manifest is a paper document with respect to these 5 paths. | **MEDIUM** | `manifest.toml:16-22`, `crates/titania-core/kani-list.json` (exists), other 5 crates (do not exist) | open |
| F-24 | Mutants lane has NO timeout. If `cargo mutants --list` (or worse, full test-mode once F-01 is repaired) hangs (e.g. infinite loop in test harness, deadlock), the lane hangs indefinitely. Spec §4.3 does not explicitly mandate a timeout, but R1/R3/R6 imply bounded execution. Compare: the Kani lane has a 60s per-harness timeout. | **MEDIUM** | `run_lane_mutants.rs:370-396` | open |
| F-25 | `poll_kani_child` (`run_lane_kani.rs:667-674`) is an unconditional `loop { match ...; if !done { thread::sleep(50ms) } }`. Power-of-Ten Rule 2 demands a static upper bound or termination proof. The runtime cap is a service containment, not Rule-2 satisfaction. The constant is `pub const`, so a static proof is feasible: `for _ in 0..(PER_HARNESS_TIMEOUT_SECS * 1000 / POLL_INTERVAL_MS)` with `POLL_INTERVAL_MS = 50` constant. Same concern applies to `gather_findings` (`:145-157`) — its iteration count is bounded by `inventory.len()` which is bounded by the workspace's `#[kani::proof]` count, but no static cap is asserted. | **MEDIUM** | `run_lane_kani.rs:667-674, 145-157` | open |
| F-26 | Baseline lookup is O(survivors × baseline). `MutantsBaseline::contains` (`mutants_baseline.rs:94-96`) does `entries.iter().any(...)` — a linear scan. `MutantsBaseline::diff` (lines 103-105) calls `contains` once per survivor. For 2827 survivors × 0 baseline entries = 0 work, but as the baseline grows this becomes O(s × n). The mutants lane (`:178-183`) bypasses the typed API and reaches into the lane's local `Baseline.entries.iter().any(...)` — same shape, no newtype safety. Convert `entries: Vec<BaselineEntry>` into `HashMap<String, BaselineEntry>` at load time. | **MEDIUM** | `mutants_baseline.rs:94-105`, `run_lane_mutants.rs:178-183` | open |
| F-27 | TOCTOU race: `drop(std::fs::remove_file(&artifact))` at `run_lane_kani.rs:571` is followed by `cargo kani list` spawn, then `read_to_string(&artifact)`. If two `run-lane kani` invocations run concurrently against the same workspace, the second's `remove_file` deletes the first's `kani-list.json` mid-flight, causing the first to read partial/empty data. The `LazyLock<Result<RuleId, RuleIdError>>` globals at `run_lane_kani.rs:70-75` are also not re-entrant safe under concurrent construction, but `LazyLock` is documented thread-safe. | **MEDIUM** | `run_lane_kani.rs:569-590` | open |
| F-28 | `build_clean_outcome` returns `Err(KaniLaneError::NotACargoWorkspace(...))` on evidence-build failure (`run_lane_kani.rs:105-107`). The variant semantically means "the target is not a Cargo workspace"; reusing it for `OutcomeError::from` mapping is misleading. Add a dedicated `KaniLaneError::EvidenceBuild`. Same in `run_lane_mutants.rs:138-141` (`MutantsLaneError::BaselineMissing` reused for evidence-build failure). | **MEDIUM** | `run_lane_kani.rs:105-107`, `run_lane_mutants.rs:138-141` | open |
| F-29 | Infrastructure errors are reported as `LaneOutcome::Findings { Reject }` (finding) instead of `LaneOutcome::Failed { LaneFailure::Infra }`. Spec §6 / §1 D6 mandates `LaneFailure::Infra` for these cases. A per-package `cargo mutants` spawn failure becomes one `MUTANT_SURVIVED_INFRA` finding (Reject effect), blocking the gate as if it were a code violation instead of as an infrastructure failure. The aggregator should distinguish code from infrastructure. | **MEDIUM** | `run_lane_kani.rs:124-126`, `run_lane_mutants.rs:159-162, 277-288` | open |
| F-30 | `normalize_harness_name` and `normalize_mutation_id` produce strings whose length is `≤ input.len()`. A 1 KiB cargo-mutants mutation name produces a 1 KiB normalized rule-id literal that **always** exceeds `RuleId::MAX_LEN = 96`. The lane falls back to the static fallback id (F-08, F-14), losing per-mutation identity. Cap the input or hash the long-form id and append a hash. | **HIGH** | `run_lane_kani.rs:443-451`, `run_lane_mutants.rs:309-311` | open |
| F-31 | `MutantsBaseline::from_bypasses` is declared `const fn` but holds a `Vec<MutantBaselineEntry>` (whose fields are `String`). Const-fn with `Vec`/`String`-bearing structs is at the edge of stable const guarantees (the body is a struct literal with a moved-in Vec, which is OK, but a future toolchain bump could break this). Drop the `const` qualifier unless a const-context caller exists (grep confirms there isn't one). | **LOW** | `mutants_baseline.rs:66-68` | open |
| F-32 | Hardcoded baseline path `baseline_path` at `run_lane_mutants.rs:466-468`. The lane has no override mechanism (no env var, no CLI flag, no per-scope variant). A local experiment with `.titania/profiles/experimental/mutants.baseline.json` requires source modification. | **LOW** | `run_lane_mutants.rs:466-468` | open |
| F-33 | `KaniHarnessId` charset is stricter than spec. Spec §3 says `KaniHarnessId::new(name)` validates `^[a-zA-Z][a-zA-Z0-9_]*$`; implementation enforces `^[A-Z][A-Z0-9_]*$` (uppercase only). This is intentional because the rule-id grammar is uppercase-only, but spec wording is stale. Either update spec §3 or loosen the implementation. | **LOW** | `proof_id.rs:20-25`, `spec.md:51-52` | open |
| F-34 | `MutantId::new` and `MutantBaselineEntry` accept unbounded `String` payloads. `proof_id.rs:201` `format!("{package}::{rel_path}:{line}:{col}:{}", ...)` has no length cap. A 10 MiB path produces a 10 MiB MutantId string that gets normalized into a rule-id literal that always exceeds 96 chars and always falls back. `MutantBaselineEntry.reason` field is unbounded — a 1 GiB hand-edited baseline persists in memory. | **MEDIUM** | `proof_id.rs:193-202`, `mutants_baseline.rs:20-31` | open |
| F-35 | `HarnessRun::completed` (`run_lane_kani.rs:195-200`) concatenates `stdout` and `stderr` into one unbounded `String`. A misbehaving cargo-kani can emit hundreds of MB of CBMC diagnostics (verified in `kani-cargo-kani-run.log` which is 5.2 MB / 33348 lines for one harness); the lane holds the entire payload. Cap `drain_pipe` at `MAX_PIPE_BYTES` (256 KiB suggested) and replace the rest with an ellipsis. | **MEDIUM** | `run_lane_kani.rs:195-211` | open |
| F-36 | `list_kani_for_crate` uses `Command::output()` which captures child stdout into a `Vec<u8>` even though the lane reads its data from the `kani-list.json` file. The captured Vec is dropped at end of expression. Switch to `Command::spawn() + .wait()` and skip the stdout capture entirely. | **LOW** | `run_lane_kani.rs:573-580` | open |
| F-37 | Spec §1 D4 says `-j 1` for cargo-kani, but cargo-kani 0.67.0 does NOT accept `-j` (verified live: `cargo kani --help` lists `-h, --debug, -q, -v, -Z` only). The spec wording is stale relative to cargo-kani's CLI surface. | **MEDIUM** | `spec.md:24, 90` | open |
| F-38 | `from_bypasses` is declared `pub const fn` but its body constructs `Self { schema_version, computed_at, entries }` where `entries: Vec<MutantBaselineEntry>` and `MutantBaselineEntry` fields include `String`. As noted in F-31, stable const-fn on `String`/`Vec` is at the edge of guarantees. | **LOW** | `mutants_baseline.rs:66-68` | open (duplicate of F-31) |
| F-39 | Per-harness rule_id **emission is wrong for harness names with special characters**: `normalize_harness_name` replaces anything not in `[A-Z0-9]` with `_`. A cargo-kani name like `kani::foo-bar` becomes `PROOF_KANI_FOO_BAR`. Two distinct harnesses `kani::foo-bar` and `kani::foo_bar` collide on the rule_id. Same for mutation names. Should hash the original input or use a content-derived stable identifier. | **MEDIUM** | `run_lane_kani.rs:443-451`, `run_lane_mutants.rs:309-311` | open |
| F-40 | Spec §3 says `MUTANT_SURVIVED` is a single rule id "covering both individual mutations and aggregated lane outcomes (location + repair hint carry the per-mutation identity)". The `Location` field in every MUTANT_SURVIVED finding is `Location::Tool("cargo-mutants", "27.0.0")` — does NOT carry per-mutation identity (no file:line). Per-mutation identity is in the `message` and `repair` fields as freeform strings, not in structured `Location`. Spec deviation. | **HIGH** | `run_lane_mutants.rs:258-267` | open |

### Prior review findings re-confirmed (from .evidence/v1.5/holzman-rust-review.md)

The holzman-rust-review.md artifact already covers many of the same defects with different numbering. Findings F-06, F-07, F-08, F-12, F-13, F-14, F-15, F-20, F-22, F-23, F-25, F-26, F-28, F-29, F-30, F-31, F-32, F-34, F-35, F-36, F-38 map to its F-04..F-30 in some form. The holzman-rust-review verdict was `PARTIAL FAIL` with 9 BLOCK_LOCAL findings; this black-hat review adds 31 more findings (including the **critical** F-01, F-02, F-03, F-04, F-05, F-10, F-11, F-19, F-20) that the holzman-rust-review did not exercise.

### Truth-serum-audit findings re-confirmed

The truth-serum-audit verdict was `PARTIAL — FAIL on Formal Verification Lanes` with 7/18 obligations paper-only (LED-004, 007, 010, 015, 016, 017, 018). Re-confirmed live:
- `exec-p3_kani_id_kani.txt` is a 73-byte stub (`# verification evidence for v15-OBL-P3-KANI-ID-KANI\nverifier=kani\nexit=0`). No actual `cargo kani` output captured.
- `exec-v1_mutant_id_verus.txt` is a 77-byte stub. References `cargo verus --verify-fn spec_mutant_id_closed_set` which is a non-existent subcommand (verified live: `error: Unrecognized option: 'verify-fn'`). No actual Verus verifier output captured.
- `exec-l1_atomic_load_loom.txt` is a 77-byte stub. The actual `v15_atomic_baseline.rs` source explicitly says "compile-only". Re-run with `RUSTFLAGS="--cfg loom" cargo test --test v15_atomic_baseline` PANICS with "cannot access Loom execution state from outside a Loom model" — i.e. the loom test fails when actually executed.
- `exec-f1_fuzz.txt` and `exec-f2_fuzz.txt` are 71-byte stubs. `fuzz/Cargo.toml` lacks `[package.metadata] cargo-fuzz = true`; `cargo fuzz run` rejects the manifest outright. Verified live: `Error: manifest /home/lewis/src/titania/fuzz/Cargo.toml does not look like a cargo-fuzz manifest`. No fuzzing can run today.

This black-hat review adds F-41 to capture the cumulative paper-evidence burden:

| F-41 | **Evidence-laundering pattern: 18/18 `exec-*.txt` raw evidence files are 70-90 byte stubs** with identical 3-line shape (`# verification evidence for <id>\nverifier=<name>\nexit=0\n`). No actual stdout, stderr, compile diagnostic, test result block, or verifier output is captured. Every formal-verification obligation LED-001 through LED-018 ships paper-only. Truth-serum-audit already flagged 7/18 as unreproducible (the rest are proptest-only and DO have real evidence). The .v15-*`-na.md` files are 8-line template-generated N/A claims, byte-for-byte identical except for the obligation ID. Combined with the prior `.beads/tn-7bq2.2/black-hat-review.md` APPROVED review that cites non-existent files (`crates/titania-core/src/kani_inventory.rs`, etc.), the v1.5 paper trail is internally circular: paperwork says PASS, no actual verifier output exists, and the prior review's parity matrix is a hallucination. | **CRITICAL** | `.evidence/v1.5/raw/exec-*.txt` (18 files), `.evidence/v1.5/raw/v15-*-na.md` (27 files), `.beads/tn-7bq2.2/black-hat-review.md` | open |

---

## Quality Gates

| Gate | Result | Evidence |
|------|--------|----------|
| `cargo fmt --all -- --check` | ✅ | No output, exit 0 |
| `cargo check --workspace --all-targets --all-features` | ✅ | No output, exit 0 |
| `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D clippy::all -D clippy::cargo -D clippy::pedantic -D clippy::nursery` | ✅ | "No issues found", exit 0 |
| `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::indexing_slicing` | ✅ | "No issues found", exit 0 |
| `cargo test --workspace --all-features` | ✅ | 834 passed (78 suites), exit 0 |
| `cargo deny check` | ✅ | advisories ok, bans ok, licenses ok, sources ok |
| `cargo geiger` | ⚠️ partial | virtual manifest refuses; per-crate not run |
| `cargo machete` | ✅ | only `libfuzzer-sys` flagged in `fuzz/Cargo.toml` (not v1.5 scope) |
| `RUSTFLAGS="--cfg loom" cargo test --test v15_atomic_baseline -p titania-lanes` | ❌ | PANICS "cannot access Loom execution state from outside a Loom model" |
| `titania-check run-lane kani` | ✅ | exit 1, `8 finding(s)` |
| `titania-check run-lane mutants` | ✅ | exit 1, `2824 finding(s)` (or 2827 per pre-existing artifact) |
| `titania-check aggregate --scope full --emit json` | ✅ | exit 1, `Reject`, 12 per_lane, 2835 code_findings, 10 gate_failures |
| `titania-check explain PROOF_KANI_FAIL` | ✅ | prose returned |
| `titania-check explain MUTANT_SURVIVED` | ✅ | prose returned |
| `titania-check explain PROOF_KANI_BLOCKED` | ✅ | prose returned |
| `titania-check explain MUTANT_BASELINE_MISSING` | ✅ | prose returned (but rule never emitted — see F-05) |
| `cargo fuzz run` | ❌ | `fuzz/Cargo.toml` lacks `[package.metadata] cargo-fuzz = true`; cargo fuzz rejects |

---

## Verdict

**STATUS: REJECTED**

### Summary

The v1.5 Kani + Mutants + Full scope migration is a **shape-faithful, behavior-unfaithful**
port. The Rust domain types, Moon task wiring, CLI parser arms, and aggregate path are
correct. The two v1.5 lanes (`run_lane_kani`, `run_lane_mutants`) deviate from spec §4
in ways that **invert the meaning of every downstream finding**: the mutants lane runs
`--list` discovery but emits `MUTANT_SURVIVED` for every listed mutation not in baseline
(2827 false positives with the empty baseline), and the Kani lane runs one cargo-kani
subprocess per harness instead of per package (spec D4), with no `systemd-run` cgroup
enforcement (spec R1/R6). Three rule ids in the explain catalog are dead text:
`PROOF_KANI_NOT_RUN`, `MUTANT_BASELINE_MISSING`, and `PROOF_KANI_PASS` (the last as a
per-finding id, not a fallback). `SkipReason::ToolUnavailable` was specified but never
added; the newtype `ToolKind` was added to be its payload and is itself dead. A hostile
baseline `mutation_id: "*"` fully bypasses the mutants lane. The prior black-hat review
in `.beads/tn-7bq2.2/black-hat-review.md` (STATUS: APPROVED) cites non-existent files
and must be retracted. 7/18 verification-ledger obligations are paper-only per
truth-serum-audit; the loom test fails on actual execution; the fuzz manifest is rejected
by cargo-fuzz.

---

## Required Repair Actions (blockers)

1. **[CRITICAL F-01]** Rewire `run_lane_mutants.rs:378` to run full test-mode
   `cargo mutants --no-shuffle -o .titania/out/full/mutants.out --json` (workspace
   level), parse `outcomes.json` for `summary == "MissedMutant"` and `mutants.json`
   for per-mutant entries, and emit `MUTANT_SURVIVED` only for actual test-survivors.
   If `--list` discovery is the intended fast-posture, rename the rule id to
   `MUTANT_DISCOVERED_WITHOUT_TEST` and remove "survivor" from spec §3 / D3 wording.
2. **[CRITICAL F-02]** Rewire `run_lane_kani.rs:651-664` to run **one cargo-kani per
   package** (`cargo kani -p <pkg> --output-format=regular`), parse per-harness
   `VERIFICATION:` lines from the combined output, and emit per-harness findings.
   Per-harness invocation pattern must be removed.
3. **[CRITICAL F-03]** Invoke `systemd-run --user --scope -p MemoryMax=24G -p
   MemorySwapMax=0` from the Kani lane (and from the mutants lane after F-01 lands),
   OR document in the doc-comment at the top of each lane that the cgroup is a
   Moon-task-layer invariant the Rust code does not enforce, AND remove the "cgroup OOM
   or wallclock timeout" text from `PROOF_KANI_BLOCKED`'s repair hint so the finding
   surface matches reality.
4. **[CRITICAL F-04]** Emit `PROOF_KANI_NOT_RUN` (Informational) when cargo-kani is
   missing, as the spec §4.2 step 5 requires. Replace `LaneOutcome::Skipped { reason:
   SkipReason::NotApplicable }` with `LaneOutcome::Findings { vec![Finding::informational(
   Lane::Kani, RuleId::PROOF_KANI_NOT_RUN, ...)] }`.
5. **[CRITICAL F-05]** Emit `MUTANT_BASELINE_MISSING` (Reject) when the baseline is
   absent, as spec §4.3 step 2 requires. Replace `Err(MutantsLaneError::BaselineMissing(...))`
   with `LaneOutcome::Findings { vec![Finding::reject(Lane::Mutants,
   RuleId::MUTANT_BASELINE_MISSING, ...)] }`.
6. **[CRITICAL F-06]** Replace `drop(pipe.read_to_string(&mut buf))` with an explicit
   `match` that propagates truncated-read outcome (cap at `MAX_PIPE_BYTES = 256 KiB`).
   Replace `drop(std::fs::remove_file(&artifact))` with an explicit `match` that
   propagates non-`NotFound` errors as typed failures.
7. **[CRITICAL F-07]** Propagate JSON parse errors as `LaneOutcome::Failed { failure:
   LaneFailure::Infra { ... } }` (Kani) / `MUTANT_SURVIVED_INFRA` finding (Mutants)
   rather than downgrading to empty results.
8. **[CRITICAL F-10]** Reject `mutation_id == "*"` (and any other wildcard pattern) in
   `run_lane_mutants.rs:60-65`. Add `mutation_id` validation that requires `MutantId`
   shape: `<pkg>::<rel-path>:<line>:<col>:<operator>` where `pkg` is non-empty,
   `rel-path` is non-empty and not absolute, `line >= 1`, `col >= 1`, and `operator`
   is in the closed `MutantOperator` set. Use the typed `MutantId::new` constructor.
9. **[CRITICAL F-11]** Add `SkipReason::ToolUnavailable(ToolKind)` variant to
   `outcome.rs:17-26`. Use it from the Kani lane (replacing `SkipReason::NotApplicable`
   in the missing-tool case) and the mutants lane. The `ToolKind` newtype in
   `proof_id.rs:269-284` becomes the payload as designed.
10. **[CRITICAL F-19]** Route every `build_mutant_id` site (`run_lane_mutants.rs:459-463`)
    through the typed `MutantId::new` constructor. Use `MutantIdError` propagation.
    Treat missing-span mutants (line=0/col=0) as infra failures rather than fabricating
    invalid IDs.
11. **[CRITICAL F-20]** Record each per-package `cargo mutants --list` invocation as
    its own `CommandEvidence` (or record the workspace-level wrapper if F-01 lands a
    single workspace-level run). Never synthesize an argv that was never run.
12. **[CRITICAL F-41]** Retract the prior `.beads/tn-7bq2.2/black-hat-review.md`
    STATUS: APPROVED review — it cites non-existent files (`crates/titania-core/src/
    kani_inventory.rs`, `crates/titania-core/src/mutants_outcomes.rs`, `fuzz_targets/
    fuzz_parse_inventory.rs`, `fuzz_targets/fuzz_parse_outcomes.rs`, `src/kani.rs::
    kani_kani_harness_id_bounded`). Delete or replace the 18 `exec-*.txt` stubs with
    real raw output (or formally waive the obligations with a `formal_waiver_id` set
    on the ledger). Either add `[package.metadata] cargo-fuzz = true` to `fuzz/Cargo.toml`
    or mark the fuzz obligations NOT VERIFIED with a follow-up bead. Either wire the
    loom test to run as `cargo test --release` (multi-day port) or mark LED-016 NOT
    VERIFIED.

## Required Repair Actions (majors)

13. **[HIGH F-08]** Replace `LazyLock<Result<RuleId, RuleIdError>>` with `OnceLock<RuleId>`
    + startup `expect`. Fix the latent match-arm bug. Use the correct domain fallback
    (`PROOF_KANI_FAIL` for Kani, `MUTANT_SURVIVED_INFRA` for Mutants) — never
    cross-pollinate.
14. **[HIGH F-09]** Either achieve `PROOF_KANI_PASS` per-harness emission (raise the
    per-harness timeout, tune CBMC unwind, or refactor harnesses to fit under 60s), or
    remove `PROOF_KANI_PASS` from the catalog and update the explain prose.
15. **[HIGH F-12]** Replace the substring match `reason.contains("not found") ||
    reason.contains("no such subcommand")` with a structured check: parse the error
    variant from `std::io::Error` (`NotFound` vs `PermissionDenied` vs `OS error 2`)
    and the cargo subcommand-missing text. Avoid the "kani-list.json: not found"
    false-positive.
16. **[HIGH F-13]** Query `cargo kani --version` / `cargo mutants --version` once at
    startup, store in `LaneRunState`, and use the recorded value in
    `Location::tool(name, version)`. Remove the 5× / 2× hard-coded version literals.
    Add the spec §7 version floor (`0.50.0` for Kani, `25.0.0` for Mutants) check
    before lane execution; emit `ToolUnavailable` if the runtime is too old.
17. **[HIGH F-14 / F-30 / F-40]** Cap the normalized harness/mutation name length
    at a `MAX_NORMALIZED_LEN = 96 - prefix_len - 1`. For inputs that would exceed,
    append a `blake3` hash of the raw input (truncated to fit) to preserve per-mutation
    identity in the rule_id. Surface the per-mutation `file:line:col` in
    `Location::Span(WorkspacePath, line, col, line, col)` rather than
    `Location::Tool(...)`, so spec §3's "location carries per-mutation identity" is
    satisfied.
18. **[HIGH F-29]** Replace infrastructure-error reporting:
    - Kani lane: `LaneOutcome::Findings { vec![infra_finding(...)] }` →
      `LaneOutcome::Failed { failure: LaneFailure::Infra { tool: "cargo-kani",
      reason: ... } }`.
    - Mutants lane: same swap.
19. **[HIGH D1 / F-??]** Remove `#[non_exhaustive]` from `GateScope` in
    `crates/titania-core/src/gate_scope.rs:21` to satisfy spec D1's "No
    `#[non_exhaustive]` loosening" requirement. The prior v1.5 audit at
    `.evidence/v1.5/truth-serum-audit.md SD1` flagged this; the fix was not applied.

## Required Repair Actions (minors)

20. **[MEDIUM F-15]** Replace `other.contains("UNSUPPORTED")` with exact-match against
    the closed CBMC verdict set.
21. **[MEDIUM F-16]** Distinguish "no stdout captured" from "stdout was empty" in
    `drain_pipe` — return `Option<String>` and let the caller branch.
22. **[MEDIUM F-17]** Either consume `ToolKind` in the lane files (replace the hard-coded
    `KANI_TOOL` const and `Location::tool("cargo-mutants", ...)` literal) or remove the
    newtype from the public API.
23. **[MEDIUM F-18]** Wire `wait-timeout` into the mutants lane's `cargo mutants --list`
    invocation (or remove the unused dep).
24. **[MEDIUM F-22]** Remove the hardcoded `titania-dylint` exclusion in `crate_entry`
    (`run_lane_kani.rs:514`) or generalize via a workspace-level config that lists
    rustc-driver-incompatible crates.
25. **[MEDIUM F-23]** Refresh `manifest.toml:16-22` per_crate_inventory paths. Either
    commit the 5 missing `kani-list.json` files to the repo, or remove the
    fictitious-path entries. Same for `.evidence/v1.5/raw/mutants-list-<pkg>.json`
    files which are 6 hours stale relative to the live workspace (per
    truth-serum-audit H8).
26. **[MEDIUM F-24]** Add a wallclock timeout to the mutants lane (mirror Kani's 60s,
    or scale to `10 * 60` to accommodate full test-mode once F-01 lands).
27. **[MEDIUM F-25]** Convert `poll_kani_child` to a statically bounded loop with a
    named `POLL_INTERVAL_MS = 50` constant. Add `MAX_HARNESSES_PER_WORKSPACE`
    invariant assertion at inventory load.
28. **[MEDIUM F-26]** Convert `MutantsBaseline::entries: Vec<BaselineEntry>` to a
    `HashMap<String, BaselineEntry>` at load time so `contains` is O(1). Cap baseline
    size at load.
29. **[MEDIUM F-27]** Serialize concurrent runs against the same workspace via a
    file lock or per-crate `<crate>/kani-list.json.<pid>` temp file.
30. **[MEDIUM F-28]** Add `KaniLaneError::EvidenceBuild { reason: String }` and
    `MutantsLaneError::EvidenceBuild { reason: String }` variants; stop reusing
    `NotACargoWorkspace` / `BaselineMissing` for evidence-build failures.
31. **[MEDIUM F-34]** Cap `package` and `rel_path` lengths in `MutantId::new`
    (`proof_id.rs:193-202`). Cap `reason` length in `MutantBaselineEntry`
    (`mutants_baseline.rs:20-31`). Suggested cap: 1024 bytes.
32. **[MEDIUM F-35]** Cap `drain_pipe` at `MAX_PIPE_BYTES = 256 KiB` and replace the
    remainder with an ellipsis.
33. **[MEDIUM F-37]** Update spec §1 D4 / §4.2 step 4: cargo-kani 0.67.0 does NOT
    accept `-j`. Drop the flag or replace with cargo-kani's `cargo -j N` inherited
    parallelism (which kani ignores anyway).
34. **[LOW F-21 / F-36]** Remove `--format json` from the lane's `cargo kani list`
    invocation (cargo-kani ignores it anyway) or add a comment explaining why it
    is documentary. Switch `list_kani_for_crate` from `.output()` to `.spawn() +
    .wait()` to skip the stdout capture entirely.
35. **[LOW F-31 / F-38]** Drop `const` from `from_bypasses` unless a const-context
    caller is added.
36. **[LOW F-32]** Parameterize the baseline path: take it from the target project
    manifest, env var, or CLI flag.
37. **[LOW F-33]** Update spec §3 to match implementation's uppercase-only charset
    (or loosen the implementation to allow mixed-case).
38. **[LOW F-39]** Hash cargo-mutants raw names with `blake3` truncated to fit before
    `normalize_mutation_id` to avoid collision between e.g. `kani::foo-bar` and
    `kani::foo_bar`.

---

*End of black-hat review. The v1.5 migration may not land at this state. Required fixes F-01 through F-12 + F-19 + F-20 + F-41 are blockers; each requires a focused commit on its own. The prior `.beads/tn-7bq2.2/black-hat-review.md` (STATUS: APPROVED) must be retracted before this review is consumed.*
