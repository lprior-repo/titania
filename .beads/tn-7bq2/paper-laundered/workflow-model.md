# v1.5 Workflow Model

> Workflows as typed state-machine traces. The v1.0 specs treat each lane
> as a single transition from `Pending` → `Outcome`; v1.5 adds two
> parallel workflows (Kani, Mutants) that aggregate sub-states per
> harness/mutant. Both share the `LaneOutcome` shape so the aggregate
> layer treats them uniformly.

## Lane-level workflow (shared)

```
Pending
  ├─ Spawn tool ─ success, exit 0 ──────────→ outcome_from_tool(tool_clean)
  ├─ Spawn tool ─ success, exit !=0 ────────→ outcome_from_tool(tool_reject)
  ├─ Spawn tool ─ spawn failure ────────────→ outcome_from_tool(infra)
  ├─ Lane-skipped branch ───────────────────→ LaneOutcome::Skipped{reason}
  └─ Lane-error branch (panic path, must never fire) ──→ ∀ no panic
```

`LaneOutcome` variants unchanged from v1.0:
- `Clean{evidence}`
- `Findings{evidence, findings: Vec<Finding>}`
- `Skipped{reason: SkipReason}`
- `Failed{failure: LaneFailure}`

## Mutants lane workflow (v1.5)

```
Pending
  ├─ cargo-mutants --version missing ──→ Skipped{reason: ToolUnavailable(CargoMutants)}
  ├─ Baseline file missing ────────────→ Findings{
  │     [Finding{MUTANT_BASELINE_MISSING, …}]
  │   } RejectKind::MutantSurvivor
  └─ Baseline loaded ─→
       Load cargo mutants output
       ├─ Read `outcomes.json` for totals
       ├─ Read `mutants.json` for per-mutant data
       ├─ Build survivors: [MutantId]
       └─ Diff against baseline ──→
            survivors \ baseline = new_survivors: [MutantId]
            ├─ new_survivors.is_empty() ──→ Clean{evidence = "cargo mutants: 0 new survivors / <total>"}
            └─ new_survivors non-empty ──→ Findings{
                  [Finding{MUTANT_SURVIVED, mid, loc, repair_hint}, …]
                } RejectKind::MutantSurvivor
```

### Bootstrap workflow (mutant-accept)

```
Reading the first .titania/out/full/mutants.json → entries[t].findings[]
  ├─ For every surviving mutant with a "killable" predicate
  │     → write a unit/property test that fails after mutation
  │     → re-run cargo mutants to confirm caught
  ├─ For every surviving mutant with an "unavoidable" predicate
  │     → add entry to `.titania/profiles/strict-ai/mutants.baseline.json`:
  │       {
  │         "mutation-id": "<MutantId>",
  │         "accepted-by-rule":
  │           "mutant-accept/<owner>/<reason>/<expiry-iso8601>",
  │         "reason": "<human-readable>",
  │         "expires_on": "<iso8601>"
  │       }
  └─ All survivors either killed or accepted
        → baseline commits, gate-full starts enforcing.
```

## Kani lane workflow (v1.5)

```
Pending
  ├─ cargo-kani missing or version < 0.50.0 ─→ Skipped{reason: ToolUnavailable(CargoKani)}
  └─ cargo-kani present ─→
       Enumerate packages: [PkgName]
       For each pkg:
         ├─ cd crates/<pkg> && cargo kani list --format json
         │     → harness_inventory: [KaniHarnessId]
         └─ cargo kani -p <pkg> -j 1 --output-format regular inside cgroup
               Per harness:
                 ├─ "VERIFICATION:- SUCCESSFUL" ─→ Finding{PROOF_KANI_PASS}
                 ├─ "VERIFICATION:- FAILED"     ─→ Finding{PROOF_KANI_FAIL}
                 ├─ "UNDETERMINED"               ─→ Finding{PROOF_KANI_FAIL}
                 ├─ Counterexample trace        ─→ Finding{PROOF_KANI_FAIL} with trace excerpt
                 ├─ "unsupported feature"        ─→ Finding{PROOF_KANI_UNSUPPORTED}
                 ├─ OOM/timeout                  ─→ Finding{PROOF_KANI_BLOCKED} with cgroup log
                 └─ tool spawn failure           ─→ finding_kind(Findings; RejectKind::KaniFail)
       Aggregate:
         ├─ every harness Successful OR UnsupportedFeature ──→ Clean
         ├─ any harness Fail OR Blocked OR NotRun ───────────→ Findings[..] RejectKind::KaniFail
         └─ write `.titania/out/full/kani.json`
```

## Full gate workflow

```
GateScope::Full → Moon :titania:gate-full
  ├─ :titania:lint-src          (lint source — unchanged from v1)
  ├─ titania-clippy-all          (strict clippy — unchanged)
  ├─ :titania:gate-release       (Release composite — unchanged)
  ├─ titania-kani                (NEW — runs Kani lane)
  └─ titania-mutants             (NEW — runs Mutants lane)
```

Aggregate step collects all scope lane artifacts and produces one
`Report::Pass` if every artifact is `LaneOutcome::Clean` (subject to
the v1.0 acceptance rules), or `Report::Reject` listing
`code_findings: Vec<Finding>` and `gate_failures: Vec<LaneFailure>`.

## Failure handling (railway)

Each lane is a `Result<LaneOutcome, RunLaneError>`. The functional core
runs pure mappings; the shell maps errors to typed `LaneFailure`
variants. Domain rule: `Result<T, String>` is forbidden in core; only
`thiserror`-derived enums.

| Error source | Mapping in shell |
|--------------|------------------|
| `cargo kani list --format json` failed | `LaneFailure::Infra { tool: "cargo-kani", message, exit_code }` |
| `cargo kani -p <pkg> ...` failed | `LaneFailure::Infra { tool: "cargo-kani", message, exit_code }` |
| `cargo mutants --version` missing | `SkipReason::ToolUnavailable(ToolKind::CargoMutants)` |
| `mutants.out/outcomes.json` missing or malformed | `LaneFailure::Infra { tool: "cargo-mutants" }` |
| Baseline file malformed | `MutantsBaselineError::Invalid` → `LaneOutcome::Findings( MUTANT_BASELINE_MISSING )` |
| CBMC OOM (`systemd-run` exit 137) | finding `PROOF_KANI_BLOCKED{ with cgroup log path }` |

No panic anywhere on the railway. No `unwrap`/`expect`/`panic!`/
`todo!`/`unimplemented!`/`unreachable!` outside tests.
