# Refinement verification report — tn-4isu (State 12)

All 4 bridge rows (RRO-DIFF/WIRE/ID/FULL) remain behavior_affecting=false with
mapping_status=materialized: the change is unwind attributes on `cfg(kani)`
harnesses, so no executable-behavior refinement exists to verify. Evidence
mapping per row:

- RRO-DIFF → kani-verify-diff-u16.log (timeout 400s) + kani-triage-diff-u32.log
  (timeout 280s). Refinement claim (16 suffices) NOT established → open-blocker.
- RRO-WIRE → kani-verify-wire-u12.log (timeout 400s) + u24/u64 triage logs.
  Refinement claim (12 suffices) NOT established → open-blocker.
- RRO-ID → kani-verify-id-u100.log (timeout 400s). Diagnose-only; bound kept.
- RRO-FULL → per-harness floor arithmetic (machine-gate-report.md); package run
  not executed (certain timeout).

No behavior-test refs exist because no behavior changed (bridge review accepted
this as correct, not a gap). No refinement-harness substitution: every property
is still checked against production constructors directly.

## STATUS: PASS
