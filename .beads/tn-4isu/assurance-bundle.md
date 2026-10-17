# Assurance bundle — tn-4isu (State 14)

Requirement → source → test/proof → raw command map (every link is a file):

- REQ-BOUNDED-DIFF → kani.rs:370 (@16) → OBL-DIFF → u32/u16 timeout logs
  (7518/13663 unwind lines, EXIT 124) → V-DIFF FAIL_LOCAL-closed. Plus 8/8
  fixed-candidate property unchanged (diff inspection).
- REQ-BOUNDED-WIRE → kani.rs:519 (@12) → OBL-WIRE → u64/u24/u12 timeout logs
  (1367/1105/5511 unwind lines, EXIT 124) → V-WIRE FAIL_LOCAL-closed.
- REQ-DIAGNOSE-ID → kani.rs:144 (@100 kept) → OBL-ID → u100 timeout log
  (2147 unwind lines, EXIT 124) → V-ID FAIL_LOCAL-closed.
- REQ-INVENTORY → 12-harness inventory → 9 SUCCESSFUL raw logs (0.48/0.63/2.27/
  59.07s + 5 sub-5s lights) + 3 numbered blockers + floor-sum arithmetic →
  V-FULL FAIL_LOCAL-closed. Behavior guard: cargo suite 96/96 green.
- Static gates → fmt/check/clippy/test/doc logs, all exit 0.
- Reviews → plan APPROVED, proof APPROVED, bridge APPROVED, tests APPROVED,
  black-hat APPROVED (parity matrix, bead stays OPEN).

No link is prose-only; no number lacks a log line (truth-serum-report.md
records the re-grep audit). Bead stays OPEN: 3 structural blockers stand and
Full remains blocked — the explicitly sanctioned terminal state for unresolvable
exhaustion ("record exact evidence and report blocked with the numbers").
