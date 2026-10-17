# Workflow model — tn-4isu

1. Triage: per-harness `cargo kani -p titania-core -j 1 --harness <name>` in
   cgroup, lightest-first, capped per-harness timeouts. Record time/verdict/log.
2. Reduce: edit `#[kani::unwind]` on harnesses 11 (→16) and 12 (→24) only.
3. Re-run: same per-harness commands; SUCCESSFUL with unwinding checks enabled
   proves bound sufficiency (sound-by-construction).
4. Reconcile: full per-package `cargo kani -p titania-core -j 1` in cgroup
   within the 600s lane budget; expect 12/12 SUCCESSFUL.
5. If any harness still exhausts: freeze, record exact numbers, report bead
   BLOCKED. No cap lift, no harness drop.
