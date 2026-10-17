# Test plan — tn-f7ti

writer_skill: test-planner
writer_invocation_id: INV-10-test-planner

## Proof/Refinement Coverage Matrix

| Proof ID | Refinement ID | Behavior checks | Kind |
|----------|---------------|-----------------|------|
| PO-RC-verus | RRO-RC-verus | registered_proofs_are_accounted_for_by_stable_identity, deleted_registered_file_fails_the_named_obligation | happy + error |
| PO-RC-kani | RRO-RC-kani | proof_free_replacement_fails_the_named_obligation | error |
| PO-RC-flux | RRO-RC-flux | removed_registry_row_hiding_a_required_obligation_blocks | error |
| PO-RC-proptest | RRO-RC-proptest | unknown_registry_fields_fail_closed, checked_in_registry_parses_with_smoke_authority | boundary |
| PO-SD-verus | RRO-SD-verus | smoke_fixture_cannot_satisfy_a_production_obligation, explicit_smoke_tag_wins_over_proof_tokens | error + edge |
| PO-SD-kani | RRO-SD-kani | comment_and_string_mentions_are_not_proof, extension_alone_is_never_evidence_of_proof | edge |
| PO-SD-flux | RRO-SD-flux | unregistered_production_proof_requires_disposition, genuine_vr_content_counts_as_proof_by_content | error + happy |
| PO-SD-proptest | RRO-SD-proptest | live_workspace_inventory_is_clean | live E2E |

Design rules: failing-first (suite red on missing module before green
after); exact assertions on named variants (no is_ok-only checks);
deterministic in-memory readers plus one live-workspace check; verifier
harnesses never counted as behavior tests.
