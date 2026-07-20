# v1.5 Domain Model

> Companion to `.evidence/v1.5/spec.md`. This file formalizes the
> ubiquitous language, value objects, and aggregates the v1.5 milestone
> introduces. Scope: `Lane::Kani`, `Lane::Mutants`, `GateScope::Full`,
> `PROOF_KANI_*` rule ids, `MUTANT_SURVIVED`/`MUTANT_BASELINE_MISSING`
> rule ids, mutants-baseline as a typed artifact.

## Ubiquitous language

| Term | Definition | Source |
|------|------------|--------|
| **Kani harness** | A Rust function decorated with `#[kani::proof]` or `#[kani::proof_for_contract]` inside the titania workspace, normally behind `cfg(kani)`. The atomic unit of proof coverage. | §1 of spec |
| **Kani harness id** | The uppercase identifier emitted as `PROOF_KANI_<NAME>`, derived from the Kani harness function name. | §3 of spec |
| **Mutant** | One mutation cargo-mutants can apply (e.g., `==` → `!=`, integer ±1, `&&` → `\|\|`). | §2 of spec |
| **Test-survivor (cargo-mutants)** | A mutant that, under full `cargo mutants` (test-running) mode, builds AND passes the full `cargo test` of the mutated package without being killed. **Not** the `--check` build-only "success". | §4.3 of spec; R3 |
| **Mutant id** | A stable identifier shaped `<pkg>::<rel-path>:<line>:<col>:<operator>`. Stable across reruns for the same source mutation. | §3 of spec |
| **Mutant-accept policy exception** | A baseline entry declaring a survivor is acceptable by domain rule. Same shape as v1.0 policy exceptions, with `accepted-by-rule: mutant-accept/<owner>/<reason>/<expiry>`. | §4.4 of spec |
| **Full gate** | The composite gate that runs the v1.0 release lanes plus Kani and Mutants. | §8 of spec |
| **Rejection kind** | A typed sum-type tag on `LaneOutcome::Reject` that names which domain classification the rejection falls under. v1.5 adds `KaniFail` and `MutantSurvivor`. | §6 of spec |

## Aggregates

### Kani runs (per package)

```
KaniRun {
  package: PkgName,
  harness_inventory: [KaniHarnessId],
  per_harness: HashMap<KaniHarnessId, HarnessOutcome>,
  cgroup_log: PathBuf,
  started_at_utc: TimestampIso8601,
  completed_at_utc: TimestampIso8601,
}

HarnessOutcome = Successful | Failed{counterexample: String?} | UnsupportedFeature{warnings: [String]} | Blocked{reason: String} | NotRun{tool_missing: bool};
```

The `KaniRun` lives in titania-core (pure) once built. The shell builds it.

### Mutants run (per package)

```
MutantsRun {
  package: PkgName,
  total_mutants: Natural,
  baseline_path: PathBuf,
  baseline_entries: [MutantBaselineEntry],
  new_survivors: [MutantId],
  judged_total: Natural,                       // total_mutants - baseline_entries.len()
  started_at_utc: TimestampIso8601,
  completed_at_utc: TimestampIso8601,
}

MutantBaselineEntry {
  mutation_id: MutantId,
  accepted_by_rule: String,                   // "mutant-accept/<owner>/<reason>/<expiry>"
  reason: String,                             // human-readable
  expires_on: Option<DateIso8601>,
}
```

The `MutantsRun` lives in titania-core once parsed. The shell parses the
`mutants.out/outcomes.json` and `mutants.out/mutants.json` artifacts into
this aggregate.

### Mutants-baseline artifact

A typed JSON file at `.titania/profiles/strict-ai/mutants.baseline.json`:

```
{
  "schema_version": 1,
  "computed_at": "2026-07-15T08:42:00Z",
  "entries": [MutantBaselineEntry, ...]
}
```

Empty baseline ⇒ zero tolerance for any mutation that survives tests.

## Commands

| Command | Initiator | Pre | Post |
|---------|-----------|-----|------|
| `EnumerateKaniHarnesses(pkg)` | shell | package path | `[KaniHarnessId]` |
| `RunKaniHarness(pkg, harness, cap_cgroup)` | shell | `KaniHarnessId` | `HarnessOutcome` |
| `LoadMutantsBaseline()` | shell | baseline path | `Result<MutantsBaseline, BaselineError>` |
| `EnumerateMutantsTestSurvivors(pkg)` | shell | package path | `[MutantId]` |
| `DiffAgainstBaseline(survivors, baseline)` | core (pure) | `[MutantId]`, `MutantsBaseline` | `[MutantId]` (new survivors) |

## Events

| Event | When emitted | LaneOutcome reflection |
|-------|--------------|------------------------|
| `PROOF_KANI_PASS` | per-harness `VERIFICATION:- SUCCESSFUL` | contributing `PROOF_KANI_PASS` finding |
| `PROOF_KANI_FAIL` | per-harness `VERIFICATION:- FAILED` or counterexample | `PROOF_KANI_FAIL` finding + reject kind `KaniFail` |
| `PROOF_KANI_BLOCKED` | per-harness OOM/timeout | `PROOF_KANI_BLOCKED` finding + reject kind `KaniFail` |
| `PROOF_KANI_NOT_RUN` | `cargo-kani` missing or too old | lane disposition `NotApplicable` |
| `PROOF_KANI_UNSUPPORTED` | unsupported-feature warning | `PROOF_KANI_UNSUPPORTED` finding; gate stays green |
| `MUTANT_SURVIVED` | per-new-survivor | `MUTANT_SURVIVED` finding + reject kind `MutantSurvivor` |
| `MUTANT_BASELINE_MISSING` | first-run operator error | `MUTANT_BASELINE_MISSING` finding + reject kind `MutantSurvivor` |

## Policies

- **P-Zero-Survivor Baseline**: every new mutation must be killed by the
  test suite, OR accepted as an entry in the baseline with a typed
  policy exception. Empty baseline is the target.
- **P-Cgroup-Cap**: every cargo-kani run is wrapped in `systemd-run
  --user --scope` with `MemoryMax=24G`, `MemorySwapMax=0`, and `-j 1`.
- **P-No-Workspace-Kani**: `cargo kani --workspace` is forbidden. The
  lane enumerates crates and runs `cargo kani -p <pkg>` per crate to
  avoid the rustc-driver collision with `titania-dylint` (R6).
- **P-FullTest-Mutants**: `cargo mutants --check` is forbidden. Full
  `cargo mutants` mode is required to obtain true test-survivors (R3).
- **P-Kani-Before-Dylint**: in the `gate-full` composite, Kani runs
  before the dylint lane so collisions surface earlier (R6).
