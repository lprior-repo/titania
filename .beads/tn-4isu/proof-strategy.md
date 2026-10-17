# Proof strategy — tn-4isu

Bounded-implementation lane only (kani). For each recorded timeout:

1. Attribute cost: unwind over-approximation (harnesses 11, 12) vs symbolic
   state (harness 9). Research-notes §Diagnosis records the attribution.
2. Reduce unwind where the production loop-trip analysis proves margin
   (type-contracts.md): 11 → 16, 12 → 24. Keep 9 at 100.
3. Execute per-harness commands from `proof-obligations.planned.jsonl` under
   the mandated cgroup. SUCCESSFUL with unwinding assertions on = bound
   sufficient = obligation closed. Unwinding-assertion FAILURE = revert that
   harness and record a numbered blocker.
4. Reconcile with one per-package run (OBL-FULL) inside the 600s lane budget.

No deductive (verus), refinement (flux-rs), property (proptest), concurrency
(loom), UB-interpreter (miri), or fuzz (cargo-fuzz) obligations: the change is
unwind attributes on existing harnesses, behavior_affecting=false throughout.
