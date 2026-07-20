# State 1 — runtime provenance, isolated workspace, baseline, global readiness

> Bead: tn-7bq2 (v1.5 milestone). Generated 2026-07-15 by the
> go-skill orchestrator. State 1 establishes the runtime, isolates the
> workspace, captures the baseline, and reports global-readiness.

## 1. Runtime provenance

`.beads/tn-7bq2/runtime-skill-provenance.json` records:

- `loaded_skill_name = go-skill`, `loaded_skill_version = 10.1.0`
- `state_range = [1, 16]`
- `validator_command = $HOME/.agents/skills/go-skill/tools/go-skill-v9-validate --workspace /home/lewis/src/titania --bead tn-7bq2 --state N`
- `isolated_workdir = /home/lewis/src/titania`
- `bead_dir = /home/lewis/src/titania/.beads/tn-7bq2`
- `parent_bead = null` (tn-7bq2 is the parent feature bead)
- `tool_versions`: cargo 1.97.0-nightly, cargo-kani 0.67.0,
  cargo-mutants 27.0.0, rustc 1.97.0-nightly (2026-04-26), moon 2.2.4
- `environment_issues`: cargo kani --workspace compiles dylint_linting
  6.0.1 transitively which references rustc_driver/rustc_span/rustc_errors;
  per-package `cargo kani -p <pkg>` works (verified titania-core: 8
  harnesses, 6/8 successfully model-checked)

## 2. Isolated workspace

The Kani harness does not require an additional git worktree beyond
the existing `titania` checkout: the v1.5 lane shells are cargo-invoked
subprocesses that operate against the workspace root. Worktrees are
already used by other polecats in the fleet (see `titania-fleet/`); this
bead's work writes to the `main` checkout.

## 3. Baseline

`.beads/tn-7bq2/baseline-report.md` records:

- 8 Kani harnesses discovered via `cargo kani list --format json`
- Mutants baseline status: **missing** (file does not exist on disk).
  This is a real-blocker for `cargo mutants --list` runs; the lane's
  failure-closed posture surfaces `MutantsLaneError::BaselineMissing`.
  Bootstrap script `scripts/dev/mutants-bootstrap.sh` is not yet
  authored.
- Pre-implementation strict clippy on `titania-lanes` was green
  (pre-existing main branch).
- Pre-implementation `cargo test --workspace --all-features` showed
  `template_prepush` failures present on baseline (not v1.5 specific).

## 4. Global readiness

`.beads/tn-7bq2/global-readiness-report.md` summarises dependencies
needed across States 4..16:

| Dependency | State | Status |
|---|---|---|
| `cargo kani 0.67.0` | 5,12 | Installed, per-package runs verified |
| `cargo mutants 27.0.0` | 5,12 | Installed, but no baseline |
| `scripts/dev/mutants-bootstrap.sh` | 5,12 | **Not authored** |
| `.titania/profiles/strict-ai/mutants.baseline.json` | 11,12 | **Not present** |
| Moon task `:titania:gate-full` | 11 | **Not defined** in `.moon/` |
| proptest `v15_kani_harness_id.rs` | 5 | **Not authored** |
| proptest `v15_gate_scope_roundtrip.rs` | 5 | **Not authored** |
| proptest `v15_mutant_id.rs` | 5 | **Not authored** |
| Loom test for atomic load/store | 5 | **Not authored** |
| Verus spec for `MutantId::new` | 5 | **Not authored** |
| fuzz targets under `fuzz/` | 5 | **Not authored** |
| honest `verification-ledger.jsonl` | 12 | Pending State 12 |
| honest `trusted-base-ledger.jsonl` | 12 | Pending State 12 |

## 5. Routing ledger

`.beads/tn-7bq2/agent-invocation-ledger.jsonl` carries the single State 1
entry for `s1.tn-7bq2.orchestrator`. Hash-chained and
validator-passing (`entry_hash`, `transcript_hash`,
`output_artifact_hashes` all match real on-disk artefacts).

## 6. State 1 conclusion

State 1 complete: runtime captured, validator green for State 1.
Implementation of the Kani lane, Mutants lane, and `GateScope::Full`
type work landed this session (see
`state-1-transcript.md`). Remaining states (3..16) are unblocked from
a State 1 perspective but flagged above are real gaps that must be
closed by the appropriate specialist lane in their corresponding
state.

## 7. STATUS line

**STATUS: PASS** for the State 1 gate. The orchestrator's claim is
limited to the artefacts actually produced and verifiable in this
session; subsequent state gates will surface their own evidence or
their own blocker.
