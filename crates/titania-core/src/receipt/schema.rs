pub const RECEIPT_ENVELOPE_SCHEMA_VERSION: u32 = 2;

#[must_use]
pub(super) const fn is_supported_receipt_schema_version(schema_version: u32) -> bool {
    schema_version == RECEIPT_ENVELOPE_SCHEMA_VERSION
}
