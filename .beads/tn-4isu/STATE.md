# STATE.md — tn-4isu

Bead: `tn-4isu` — kani: resolve recorded Full-scope solver exhaustion within resource bounds.
Revision: `927893f922fe2b9c658a0c1574086afe7d6a6dc6`.
Workspace (isolated): `/home/lewis/src/titania/.muse/worktrees/subagent-v2-01a14623-5a8f-76e0-be4a-be4eb153aaf7-01a146dc-32cc-7d23-8f2f-dcaec7895c38`.
Source checkout (read-only reference): `/home/lewis/src/titania`.
Claim: `bd update tn-4isu --claim` exit 0 (2026-10-16).

## Ground truth (accepted, not re-proven)

- `tn-mudh` notes: 12/12 Kani harness timeouts after 600s, no counterexamples, Full-scope only.
- `tn-7bq2` reopened: required Full-gate pass never established; closure history is not pass evidence.
- Do NOT rerun workspace-wide kani to confirm; diagnose on current revision and repair search bounds.

## Current state

- State 1 (setup): in progress. Artifacts below created in this worktree.
- Go-skill pipeline runs inline in this delegation environment: only
  `general-purpose` / `workflow-subagent` child types exist, so no named
  specialist subagent can be spawned without mislabeling. Each state records
  the skill body actually followed in `routing-ledger.jsonl` with the honest
  executor id. Validator STATUS is reported verbatim; any dispatch-integrity
  finding is carried as residual risk, not silently waived.

## Harness inventory (current revision, `crates/titania-core/src/kani.rs`)

12 `#[kani::proof]` harnesses, all in package `titania-core`:

| # | Harness | Unwind | Symbolic surface |
|---|---|---|---|
| 1 | `lane_name_rejects_empty_string` | default | none (fixed input) |
| 2 | `lane_name_rejects_nul_byte` | default | none (fixed input) |
| 3 | `lane_digest_rejects_passed_greater_than_scanned` | default | 2× u32 + assume |
| 4 | `lane_digest_accepts_passed_not_greater_than_scanned` | default | 2× u32 + assume |
| 5 | `recorded_target_root_rejects_empty_string` | default | none |
| 6 | `recorded_target_root_rejects_relative_path` | default | none |
| 7 | `recorded_target_root_rejects_nul_byte` | default | none |
| 8 | `recorded_target_root_accepts_absolute_path` | default | none |
| 9 | `kani_kani_harness_id_bounded` | 100 | 32 bytes + len + 2 bytes |
| 10 | `kani_kani_lane_name_roundtrip` | 32 | none (iterates Full lanes) |
| 11 | `kani_mutants_baseline_diff_zero_neg` | 32 | 24 bools + 8 fixed candidates |
| 12 | `outcome_wire_invariants` | 64 | 8 bytes + len |

Timeout model: the lane runs ONE `cargo kani -p titania-core` with
`PER_PACKAGE_TIMEOUT_SECS = 600`; a solver stall in any of harnesses 9/11/12
blocks all 12 as `PROOF_KANI_BLOCKED`. Repair lever: sound unwind reduction
(Kani checks unwinding assertions by default, so an insufficient bound fails
loudly instead of passing silently) plus per-harness triage runs.

## Resource rules (mandatory)

- Per-package runs only; `-j 1`; cgroup
  `systemd-run --user --scope --collect -p MemoryHigh=20G -p MemoryMax=24G -p MemorySwapMax=0`.
- Never unbounded workspace kani. No blanket timeout increase.
- No `moon ci`, no mutants lanes. No commit/push/sync.

## Resume

Read this file, then `routing-ledger.jsonl` (newest first), then newest
`evidence/kani-*.log`.
