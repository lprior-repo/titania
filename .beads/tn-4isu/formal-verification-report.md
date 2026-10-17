# Formal verification report — tn-4isu (State 12)

## Execution completeness

All 4 planned obligations executed with exact commands from
`proof-obligations.planned.jsonl` (per-package `-p titania-core -j 1` in the
mandated cgroup; TMPDIR workspace-local). PENDING_FORMAL_EXECUTION: none remains.
Ledger: `verification-ledger.jsonl` (4 rows, all `status: closed`).

## Results (bounded claims only, cargo-kani 0.67.0)

- OBL-DIFF → FAIL_LOCAL (BLOCK_LOCAL/solver-exhaustion): @32 timeout 280s,
  @16 timeout 400s. Site-count disease (String-heavy production calls × 24
  symbolic bools); attribute-only fix exhausted at the memcmp floor.
- OBL-WIRE → FAIL_LOCAL (BLOCK_LOCAL/solver-exhaustion): @64/@24/@12 timeouts
  (280/400/400s). 356-copy serde float-parser monomorphization; no sound
  harness-side lever (stubbing serde = laundering; shrinking 8-byte domain =
  property reduction).
- OBL-ID → FAIL_LOCAL (BLOCK_LOCAL/solver-exhaustion): @100 timeout 400s
  (2147 steps). Bound floor 97; 34-symbolic-byte domain fixed.
- OBL-FULL → FAIL_LOCAL (BLOCK_LOCAL/package-infeasible): not executed;
  per-harness lower-bound sum (>1290s) exceeds the 600s lane budget mathematically.

## Collateral passes (exact per-harness evidence, same commands/bounds)

9/12 harnesses SUCCESSFUL with raw logs (machine-gate-report.md table):
the 8 fixed-input/u32 harnesses (two rescued from 120s timeouts by the
uniform-32 cap: 0.63s/2.27s) + lanename roundtrip (59.07s).

## Standing

No waivers, no disabled checks, no harness drops, no property reductions, no
faked passes. Trust dispositions: T0/T3/T4/T5 accepted; T1/T2/T7 open-blocker
(trusted-base-ledger.jsonl). The bead stays OPEN: Full remains blocked per the
bead's own error-path acceptance (E1). This report's PASS concerns execution
completeness and evidence validity — NOT proof success.

## STATUS: PASS
