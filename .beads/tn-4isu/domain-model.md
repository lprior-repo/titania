# Domain model — tn-4isu

Ubiquitous language: harness (one `#[kani::proof]` fn), unwind bound (max loop
iterations CBMC explores), per-package run (ONE `cargo kani -p <pkg>`), verdict
(`VERIFICATION: SUCCESSFUL|FAILED|...`), BLOCKED (timeout/OOM/no-verdict).

Entities: the 12 harnesses in `crates/titania-core/src/kani.rs`; the lane runner
(`run_lane_kani.rs`) mapping per-package stdout to typed `PROOF_KANI_*` findings.

Value objects: `KaniHarnessId` (cap 96, byte-scan validation), `MutantsBaseline`
(entries ≤ 8 in harness scope), `LaneOutcome` (wire round-trip over ≤ 8 bytes).

No new domain types are introduced by this bead. No lifecycle/state change.
Forbidden states: a reported SUCCESSFUL without a matching raw Kani log line on
the current revision; a reduced harness (fewer asserts/inputs) discharged
against the original obligation.
