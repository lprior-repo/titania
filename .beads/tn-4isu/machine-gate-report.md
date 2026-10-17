# Machine gate report — tn-4isu (State 12)

Worktree: `/home/lewis/src/titania/.muse/worktrees/subagent-v2-01a14623-5a8f-76e0-be4a-be4eb153aaf7-01a146dc-32cc-7d23-8f2f-dcaec7895c38`
Revision: `927893f922fe2b9c658a0c1574086afe7d6a6dc6` + uncommitted harness-attribute edits (see regression-diff.md).
Toolchain: cargo-kani 0.67.0, rustc 1.97.0-nightly. Cgroup per run:
MemoryHigh=20G/MemoryMax=24G/MemorySwapMax=0, `-j 1`. TMPDIR=target/kani-tmp
(/tmp shared-tmpfs quota).

## Static gates (all exit 0)

| Gate | Command | Exit | Log |
|---|---|---|---|
| fmt | cargo fmt --all -- --check | 0 | evidence/gate-fmt2.log |
| check | cargo check --workspace --all-targets --all-features | 0 | evidence/gate-check.log |
| clippy (source-only strict, AGENTS.md) | cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings ... (full flag list in log header) | 0 | evidence/gate-clippy.log |
| test (touched crate) | cargo test -p titania-core --all-features | 0 | evidence/cargo-test-core.log (96 passed, 0 failed) |
| doc | RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps | 0 | evidence/gate-doc.log |

OUT OF SCOPE (not run, per delegating task): moon ci, mutants lanes, workspace-wide kani.

## Kani per-harness results (all per-package `-p titania-core -j 1`, cgroup-capped)

| Harness | Bound | Verdict | Solver time | Raw log |
|---|---|---|---|---|
| lane_name_rejects_empty_string | 32 | SUCCESSFUL | <5s | evidence/kani-light-*.log |
| lane_name_rejects_nul_byte | 32 | SUCCESSFUL | <5s | evidence/kani-light-*.log |
| lane_digest_rejects_passed_greater_than_scanned | 32 | SUCCESSFUL | 0.48s | evidence/kani-triage-canary.log |
| lane_digest_accepts_passed_not_greater_than_scanned | 32 (was: default, 120s TIMEOUT) | SUCCESSFUL | 0.63s | evidence/kani-verify-*.u32.log |
| recorded_target_root_rejects_empty_string | 32 | SUCCESSFUL | <5s | evidence/kani-light-*.log |
| recorded_target_root_rejects_relative_path | 32 | SUCCESSFUL | <5s | evidence/kani-light-*.log |
| recorded_target_root_rejects_nul_byte | 32 (was: default, 120s TIMEOUT) | SUCCESSFUL | 2.27s | evidence/kani-verify-*.u32.log |
| recorded_target_root_accepts_absolute_path | 32 | SUCCESSFUL | <5s | evidence/kani-light-*.log |
| kani_kani_lane_name_roundtrip | 32 | SUCCESSFUL | 59.07s | evidence/kani-triage-lanename.log |
| kani_kani_harness_id_bounded | 100 | TIMEOUT 400s, 2147 unwind steps (~5/s), killed mid-symex | — | evidence/kani-verify-id-u100.log |
| kani_mutants_baseline_diff_zero_neg | 32: TIMEOUT 280s; 16: TIMEOUT 400s, 13663 unwind steps, no verdict | — | evidence/kani-triage-diff-u32.log, kani-verify-diff-u16.log |
| outcome_wire_invariants | 64: TIMEOUT 280s; 24: TIMEOUT 400s; 12: TIMEOUT 400s, 356-copy float-parser unwind profile | — | evidence/kani-triage-wire-u64.log, kani-verify-wire-u24.log, kani-verify-wire-u12.log |

Score: 9/12 SUCCESSFUL with raw logs; 3/12 FAIL_LOCAL (solver exhaustion with
exact numbers — verification-ledger.jsonl rows V-DIFF/V-WIRE/V-ID, all closed).

## OBL-FULL (package reconciliation): NOT EXECUTED, proven infeasible

Lower-bound arithmetic: per-harness non-completion floors are
400s+400s+400s (heavies) + 59s (lanename) + ~15s (lights) + ~60s (compile) =
>1290s of sequential `-j 1` work vs the 600s lane budget. A package run would
consume 600s to document a certain timeout; the per-harness floors already
prove it. OBL-FULL ledger row cites these three timeout logs + this arithmetic.
