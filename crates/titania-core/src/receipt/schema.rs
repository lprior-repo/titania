//! Schema-version constant for the internal `ReceiptEnvelope` namespace.
//!
//! This constant belongs to the [`crate::ReceiptEnvelope`] type — the
//! per-target-project lane-digest tracking envelope that lives in this crate's
//! `receipt` module. It is **NOT** the `QualityReceipt.schema_version` from
//! v1-spec.md §10. The spec §10 `QualityReceipt` has its own canonical
//! `schema_version: u16 = 1`, exposed as
//! [`crate::QualityReceiptV1::SCHEMA_VERSION`].
//!
//! Keeping these two schema namespaces as distinct, separately named constants
//! prevents accidental cross-wiring between the two independent envelopes.

/// Current schema version of the [`crate::ReceiptEnvelope`] wire shape.
///
/// Distinct from the spec §10 `QualityReceipt.schema_version`. See the
/// module-level docs for why both exist.
pub const RECEIPT_ENVELOPE_SCHEMA_VERSION: u32 = 2;

/// Return true when a [`crate::ReceiptEnvelope`] schema version is supported by
/// this build.
///
/// This validates the envelope namespace only; it does not touch the spec §10
/// `QualityReceipt` schema.
#[must_use]
pub(super) const fn is_supported_receipt_schema_version(schema_version: u32) -> bool {
    schema_version == RECEIPT_ENVELOPE_SCHEMA_VERSION
}
