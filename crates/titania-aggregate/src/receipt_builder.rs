//! Pure builders for v1 [`QualityReceipt`] digest envelopes.
//!
//! This module keeps receipt digest derivation separate from report assembly:
//! callers build a receipt here, then pass it into `report_assembly`.

use thiserror::Error;
use titania_core::{Digest, GateScope, LaneEvidence, QualityReceipt, ReceiptDigests, ReceiptError};

/// Errors produced while building v1 quality receipts.
#[derive(Debug, Error)]
pub enum ReceiptBuilderError {
    /// Core receipt invariant failed.
    #[error(transparent)]
    Receipt(#[from] ReceiptError),
    /// Lane evidence could not be serialized into the canonical digest payload.
    #[error("failed to serialize lane evidence for digesting: {0}")]
    EvidenceSerialization(#[from] serde_json::Error),
}

/// Build a v1 [`QualityReceipt`] from caller-supplied aggregate digests and lane receipts.
///
/// This function intentionally performs no lane execution and no report classification.
///
/// # Errors
/// Returns [`ReceiptBuilderError::Receipt`] when the core receipt constructor rejects
/// the supplied lane receipt list.
pub fn build_quality_receipt(
    scope: GateScope,
    digests: ReceiptDigests,
    lanes: Box<[titania_core::LaneReceipt]>,
) -> Result<QualityReceipt, ReceiptBuilderError> {
    QualityReceipt::new(scope, digests, lanes).map_err(Into::into)
}

/// Compute the stable digest of parsed lane evidence.
///
/// The v1 receipt records the digest of the typed [`LaneEvidence`] payload, not a
/// caller-provided constant. Serialization is via `serde_json` because lane
/// artifacts are JSON and all fields are strongly typed before hashing.
///
/// # Errors
/// Returns [`ReceiptBuilderError::EvidenceSerialization`] if serializing the
/// evidence payload fails.
pub fn compute_evidence_digest(evidence: &LaneEvidence) -> Result<Digest, ReceiptBuilderError> {
    let payload = serde_json::to_vec(evidence)?;
    Ok(Digest::from_bytes(&payload))
}
