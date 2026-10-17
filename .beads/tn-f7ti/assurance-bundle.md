# Assurance bundle — tn-f7ti

writer_skill: evidence-packaging
writer_invocation_id: INV-16-evidence-packaging

Every requirement maps to contract, proof, refinement, test, and source
evidence with raw logs retained:

- REQ-REGISTRY-COMPLETE / C1 → contract.md C1; PO-RC-* + RRO-RC-*
  (verified); 6 behavior checks; proof_registry.rs audit path;
  evidence/proof_registry-tests.log exit 0.
- REQ-SMOKE-DISPOSITION / C2 → contract.md C2; PO-SD-* + RRO-SD-*
  (verified); 7 behavior checks incl. live inventory; classify_source and
  audit_workspace; same retained log.
- Machine gates: fmt/check/clippy/doc exit 0 (evidence/*.log);
  touched-crate suite 262 passed with 1 pre-existing base-red failure
  documented in machine-gate-report.md and regression-diff.md.
- Reviews: plan, proof, bridge, test-plan, test-suite, black-hat all
  APPROVED with independent provenance; ledger hash chain intact.

Coverage integrity only; no executable-correctness claim is made anywhere
in this bundle.
