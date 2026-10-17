# Codebase map — tn-c6n (receipt domain, production-bound scope)

## In-scope production (read-only unless a gap forces a fix)

- `crates/titania-core/src/receipt.rs` — `ReceiptLaneExit` (4 variants),
  `LaneDigest::new` (passed<=scanned), `ReceiptPeriod::new` (ordering),
  `ReceiptEnvelope::{new, from_parts}` (schema gate + assembly).
- `crates/titania-core/src/receipt/lane_name.rs` — `LaneName::new`
  (empty/NUL rejection), `as_str`, serde round-trip through `new`.
- `crates/titania-core/src/receipt/target_root.rs` —
  `RecordedTargetRoot::new` (empty/relative/NUL rejection), `manifest_path`.
- `crates/titania-core/src/receipt/schema.rs` —
  `RECEIPT_ENVELOPE_SCHEMA_VERSION = 2`, `pub(super) const fn
  is_supported_receipt_schema_version`.
- `crates/titania-core/src/receipt/digests.rs` — `ReceiptDigests::new`
  (total constructor), `into_parts`.
- `crates/titania-core/src/receipt/serde_support.rs` — envelope `Deserialize`:
  schema check first, then `from_parts` (reuses all constructors).
- `crates/titania-core/src/error.rs` — `ReceiptError` (9 variants, stable
  messages); `crates/titania-core/src/lib.rs` — public re-exports used by tests.

## In-scope verification/test surface (written by this bead)

- `crates/titania-core/tests/receipt_domain_invariants.rs` (new) — behavior
  tests calling production constructors; each file kept under 300 lines.
- `crates/titania-core/tests/receipt_domain_properties.rs` (new) — proptest
  properties over production constructors.
- `crates/titania-core/tests/receipt_invariants.rs` (existing, 9 tests green) —
  not modified; paired controls for envelope paths.
- `verification/verus/receipt_schema.rs` (existing, registered) — executed with
  pinned `verus`; not modified (registry-owned; removal would trip
  registry-completeness rules owned by `tn-f7ti`).
- `contracts/proof_obligations.yaml` (existing registry) — read-only.

## Explicitly out of scope

- kani harnesses/runs (owned by `tn-7bq2`; tool absent on host).
- Full `moon ci`, mutants (owned by `tn-4isu`/`tn-7bq2`).
- New Verus mirror files (rejected by bead correction; VACUUM risk).
- `QualityReceipt` v1-spec surface (covered by existing tests; untouched).
- Aggregate/check/lanes crates (callers, not the invariant core).

## Risks

- Single-operator delivery (no second human reviewer available in this run).
- `verus` binding run classifies as source-linked per `tn-083n`; behavior
  weight rests on executable tests (stated in every artifact that cites it).
