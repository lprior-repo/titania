# Error taxonomy — tn-4isu

Kani-side outcomes (not Rust `Result`s; the repair touches no fallible code):

- `timed_out_after_600s`: solver exhaustion, no counterexample (recorded ground
  truth for 12/12). Repair target.
- `OOM / cgroup kill`: memory exhaustion under MemoryMax=24G. Repair target;
  unwind reduction shrinks symex memory.
- `VERIFICATION: FAILED` with unwinding assertion: chosen bound too short ⇒
  revert/increase that harness only. Loud failure, never silent unsoundness.
- `VERIFICATION: FAILED` with real assertion/counterexample: genuine defect —
  stop, file follow-up, do not "fix" by weakening the assert.
- `UNDETERMINED / UNSUPPORTED`: blocks; record exact feature.
- Missing tool (`cargo-kani` absent or < 0.50.0): explicit blocker.

Lane-side mapping (unchanged): timeout/OOM ⇒ `PROOF_KANI_BLOCKED` per harness;
no-verdict ⇒ `PROOF_KANI_NO_VERDICT`; all-list-failed ⇒ `PROOF_KANI_INFRA`.
