# Refinement verification report — tn-c6n (State 12)

STATUS: PASS

Executed refinement lane: proptest + behavior obligations `tn-c6n.O-P1`…`O-P6`
with bridges `tn-c6n.B-P1`…`B-P6` (all `mapping_status: verified`).

- Commands (workdir: worktree):
  - `cargo test -p titania-core --test receipt_domain_properties` → 6
    passed, exit 0 (`evidence/cargo-test-properties-s12.log`).
  - `cargo test -p titania-core --test receipt_domain_invariants` → 14
    passed, exit 0 (`evidence/cargo-test-invariants-s12.log`).
- Every behavior-affecting bridge row carries concrete source refs,
  disjoint behavior/harness refs, and the executed evidence above; ledger
  rows in `verification-ledger.jsonl` record PASS with exit status 0.
- Failing-first was demonstrated at State 9 (temporary red control: 14–1
  red, then 14–0/6–0 green; control removed from final files).

No refinement obligation remains planned; no behavior-affecting waiver
exists. Residuals (kani/flux formal lanes, single-operator review) are
documented, not closed.
