# Proof evidence — tn-4isu (State 5)

## Pre-change triage (current revision, before unwind edits)

All runs: `cargo kani -p titania-core -j 1 --harness <name> --output-format regular`
inside `systemd-run --user --scope --collect -p MemoryHigh=20G -p MemoryMax=24G
-p MemorySwapMax=0`, `cargo-kani 0.67.0`, workdir = isolated worktree.
`TMPDIR` pointed at `target/kani-tmp` (workspace-local; `/tmp` is 80% full
with other workers' files, first wire attempt failed with EDQUOT — environmental,
not a proof result).

| Harness | Bound | Result | Solver/symex time | Raw log |
|---|---|---|---|---|
| lane_digest_rejects_passed_greater_than_scanned (canary) | default | SUCCESSFUL | 0.48s | evidence/kani-triage-canary.log |
| kani_kani_lane_name_roundtrip | 32 | SUCCESSFUL | 59.07s (symex 44.21s) | evidence/kani-triage-lanename.log |
| outcome_wire_invariants | 64 | TIMEOUT at 280s cap, still unwinding `run_utf8_validation` past iteration 32, solver never started | — | evidence/kani-triage-wire-u64.log |
| kani_mutants_baseline_diff_zero_neg | 32 | TIMEOUT at 280s cap, 7518 unwind steps (memchr 5960, memcmp 1487), solver never started | — | evidence/kani-triage-diff-u32.log |

Unwind attribution: the two timeouts stall in CBMC symex (loop unwinding), not
in SAT solving — the exact phase an unwind reduction shrinks. Post-change runs
(OBL-DIFF @16, OBL-WIRE @24) execute at State 12.

## What did NOT change

Same 12 harnesses, same symbolic inputs, same `kani::assume` shaping, same
asserts and cover points, same production code. Only two `#[kani::unwind]`
attributes + one prose comment changed. Cover points remain non-vacuity
evidence; every property keeps its production-code assert.

## Trust surface

Pre-existing input shaping (`kani::assume`) is unchanged and covered by the
harnesses' own cover points. Model bounds T1/T2 are new; T3/T4 pre-existing.
Full marker table: `trusted-base-ledger.jsonl`. No deductive axioms, no
annotation-based trust, no harness doubling, no weakening flags.
