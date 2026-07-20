# rust-contract transcript for tn-7bq2.1

## Bead
tn-7bq2.1 (parent: tn-7bq2 v1.5 epic)

## Skill
rust-contract

## Start
2026-07-15T11:00:00Z

## End
2026-07-15T11:05:46Z

## Inputs
- .evidence/v1.5/spec.md

## Outputs
- .beads/tn-7bq2.1/boundary-map.md
- .beads/tn-7bq2.1/codebase-map.md
- .beads/tn-7bq2.1/contract.md
- .beads/tn-7bq2.1/delivery-scope.jsonl
- .beads/tn-7bq2.1/domain-model.md
- .beads/tn-7bq2.1/error-taxonomy.md
- .beads/tn-7bq2.1/hazard-analysis.md
- .beads/tn-7bq2.1/proof-seeds.jsonl
- .beads/tn-7bq2.1/traceability-matrix.jsonl
- .beads/tn-7bq2.1/type-contracts.md
- .beads/tn-7bq2.1/workflow-model.md

## Findings
- Pre-impl evidence: 8 Kani harnesses verified; cargo-mutants surfaces 480 candidates / 236 build-survivors in titania-core (per .evidence/v1.5/raw/).
- D1 (lane stays total) and D8 (Moon task names) locked after operator consultation.
- D3 (zero-survivor baseline) refined to full test-mode (NOT --check) after operator confirmation.
- 17 proof seeds emitted (proof-seeds.jsonl); Schema upgraded to proof-seed/v1 after validator gate failure on schema fields.
- 9 production match-lane sites identified for v1.5 blast radius; documented in codebase-map.md.
- 10 risks captured (H1-H10) and traced to mitigations.

## Open
- PROOF_KANI_* family strings consumed via explain catalog (no new RuleId variants in core).
- MutantsBaselineException shape: { mutation_id, accepted_by_rule, reason, expires_on } requires owner/reason/expiry; expiry enforced as drift resistance.
