use thiserror::Error;
use titania_core::{Digest, GateScope, LaneEvidence, QualityReceipt, ReceiptDigests, ReceiptError};

#[derive(Debug, Error)]
pub enum ReceiptBuilderError {
    #[error(transparent)]
    Receipt(#[from] ReceiptError),
    #[error("failed to serialize lane evidence for digesting: {0}")]
    EvidenceSerialization(#[from] serde_json::Error),
}

pub fn build_quality_receipt(
    scope: GateScope,
    digests: ReceiptDigests,
    lanes: Box<[titania_core::LaneReceipt]>,
) -> Result<QualityReceipt, ReceiptBuilderError> {
    QualityReceipt::new(scope, digests, lanes).map_err(Into::into)
}

pub fn compute_evidence_digest(evidence: &LaneEvidence) -> Result<Digest, ReceiptBuilderError> {
    let payload = serde_json::to_vec(evidence)?;
    Ok(Digest::from_bytes(&payload))
}
