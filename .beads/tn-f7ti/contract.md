# Contract — tn-f7ti (canonical implementation contract)

## C1 — Registered fail-closed (requirement REQ-REGISTRY-COMPLETE)

The inventory SHALL reject a registered production obligation whose file
is absent, unreadable, or free of proof tokens, naming the obligation.
Source: contracts/proof_obligations.yaml. Implementation:
crates/titania-lanes/src/proof_registry.rs::audit with
check_registered_entry. Behavior proof: deleted_registered_file_fails_the_
named_obligation, proof_free_replacement_fails_the_named_obligation,
registered_proofs_are_accounted_for_by_stable_identity.

## C2 — Smoke never counts; unregistered needs disposition (requirement REQ-SMOKE-DISPOSITION)

A smoke-tagged fixture SHALL NOT satisfy a production obligation, and a
discovered unregistered production proof SHALL fail until disposition.
Implementation: check_entry_content authority fit plus discovery sweep in
audit(), backed by audit_workspace(). Behavior proof:
smoke_fixture_cannot_satisfy_a_production_obligation,
unregistered_production_proof_requires_disposition,
live_workspace_inventory_is_clean.

## C3 — Registry integrity (both requirements)

Unknown registry sentences fail the read; removed required rows fail the
run. Implementation: parse_registry strict schema plus check_required_rows
over REQUIRED_PRODUCTION_OBLIGATIONS. Behavior proof:
unknown_registry_fields_fail_closed,
removed_registry_row_hiding_a_required_obligation_blocks,
checked_in_registry_parses_with_smoke_authority.

## Non-goals

Coverage integrity only. Nothing here claims executable correctness of any
checked program, and no verifier run is represented as one.
