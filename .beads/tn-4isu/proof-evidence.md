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

## Post-split execution (current checkout, main worktree)

Tool versions: cargo-kani 0.67.0, CBMC 6.8.0, repo toolchain
nightly-2026-04-27 (rust-toolchain.toml), verus 0.2026.05.05.d03e906,
cargo-verus present. Deviation from plan: ad-hoc runs below executed WITHOUT
the systemd-run cgroup caps (recorded as T11/finding PROOF-UNCAPPED-RUNS);
all commands used exact single-harness selection with default -j 1.
`TMPDIR=/home/lewis/.cache/muse-tmp`, kani target dirs on home disk,
`RUSTC_WRAPPER=` bypass.

Batch A: `cargo kani -p titania-core` with 10 `--harness` flags (one per
harness), overall exit 1. Raw log: evidence/kani-verify-batcha-current.log.

| Harness | Bound | Result | Verification time |
|---|---|---|---|
| kani_kani_lane_name_roundtrip | 32 | SUCCESSFUL | 43.5s |
| kani_harness_id_valid_bounded | 32 | FAILED (single unwinding-assertion check, try_from_fn_erased unwind.0; 970+ other checks pass) | 1.5s |
| recorded_target_root_accepts_absolute_path | 32 | SUCCESSFUL | 0.4s |
| recorded_target_root_rejects_nul_byte | 32 | SUCCESSFUL | 2.2s |
| recorded_target_root_rejects_relative_path | 32 | SUCCESSFUL | 0.6s |
| recorded_target_root_rejects_empty_string | 32 | SUCCESSFUL | 0.3s |
| lane_digest_accepts_passed_not_greater_than_scanned | 32 | SUCCESSFUL | 0.7s |
| lane_digest_rejects_passed_greater_than_scanned | 32 | SUCCESSFUL | 0.4s |
| lane_name_rejects_nul_byte | 32 | SUCCESSFUL | 0.2s |
| lane_name_rejects_empty_string | 32 | SUCCESSFUL | 0.2s |

Wire serialise runs (both killed before verdict, no VERIFICATION line):
`--harness outcome_wire_serialise_bounded` at annotation unwind 32 reached
13.2GB RSS after 2h CPU; same harness with `--unwind 16` CLI override reached
18.2GB RSS in 29min. cbmc invoked with cadical and slice-formula in both.
System memory stood at 119/123GB, so both solvers were terminated to protect
the box; post-kill ps shows zero cbmc/cargo-kani processes and memory back
to 90GB used.

Concrete compensation (main target dir, exit 0): `cargo test -p titania-core
--test artifact_roundtrip --test json_roundtrip` gives 5/5 plus 22/22,
including per-variant LaneOutcome round-trips and garbage rejection.

Verus toolchain smoke: `verus --no-cheating` on a vstd-prelude hello file
reports `verification results:: 1 verified, 0 errors`. Tooling evidence only;
no proof claim. A bare file without the vstd import fails first with the
expected verus_builtin diagnostic, confirming the driver enforces its prelude.

Historical note: a prior-session handoff reports kani_harness_id_invalid_first
SUCCESSFUL in 37s on current code. Not personally observed this session, so it
is recorded here as reported-only, with a re-run queued under OBL-ID-SPLIT
rather than a ledger verdict.
