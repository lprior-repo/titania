# Codebase map — tn-f7ti

## In-scope files (read before writing)

- contracts/proof_obligations.yaml — the obligation registry (extended with
  per-entry authority; smoke fixture marked non-authoritative).
- verification/verus/receipt_schema.rs — registered production proof; bridges
  to production via assume_specification over
  crates/titania-core/src/receipt/schema.rs.
- verification/verus/formal_setup_smoke.rs — explicitly non-authoritative
  setup fixture (titania-verus-binding: fixture-smoke).
- crates/titania-core/src/receipt/schema.rs — production subject
  (is_supported_receipt_schema_version, envelope version 2).
- crates/titania-lanes/src/lib.rs — lane library root (one-line module
  wiring only).
- crates/titania-lanes/src/policy_scan/exceptions.rs — error-taxonomy and
  boundary style reference (thiserror, injected readers).

## New files (this bead)

- crates/titania-lanes/src/proof_registry.rs — pure audit control.
- crates/titania-lanes/tests/proof_registry.rs — behavior suite (13 checks).

## Out of scope

- Moon task graph, kani/mutant lanes, dylint infrastructure, receipt
  envelope semantics, any change to production schema logic.
