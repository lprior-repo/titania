# Transcript 002 — state 2 explore (tn-c6n-inv-002)

Mapped the receipt domain by reading production sources (receipt.rs,
lane_name.rs, target_root.rs, schema.rs, digests.rs, serde_support.rs,
error.rs, lib.rs), existing tests (receipt_invariants.rs, 9 green), the
registered verus binding files, and the proof-obligation registry. Confirmed
all retained invariants are already enforced by smart constructors whose
serde paths reuse the same constructors. Gaps are executable evidence only:
no tests exercise LaneName/RecordedTargetRoot rejection, boundary
acceptances, exit round-trip, or error exhaustiveness; no proptest properties;
no recorded pinned-verus output. Scoped kani/moon-ci/mutants/mirrors out with
reasons in codebase-map.md. No production edit anticipated; tests-only
delivery unless test-writing exposes a real enforcement gap.
