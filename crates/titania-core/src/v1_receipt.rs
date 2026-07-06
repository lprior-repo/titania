//! v1-quality receipt types from v1-spec.md §10.
//!
//! These are distinct from the existing receipt.rs types which serve a different
//! purpose (per-lane digest tracking). `LaneReceipt` and the v1 `QualityReceipt`
//! are used in the Report domain model.

use serde::{Deserialize, Serialize};

use crate::{Digest, GateScope, Lane, error::ReceiptError, receipt::ReceiptDigests};

/// Per-lane receipt summary inside a [`QualityReceiptV1`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LaneReceipt {
    /// Which lane this receipt covers.
    pub lane: Lane,
    /// Blake3 digest of the lane's `LaneEvidence`.
    pub evidence_digest: Digest,
    /// Whether the lane produced zero findings.
    pub clean: bool,
}

impl LaneReceipt {
    /// Construct a new [`LaneReceipt`].
    #[must_use]
    pub const fn new(lane: Lane, evidence_digest: Digest, clean: bool) -> Self {
        Self { lane, evidence_digest, clean }
    }
}

// Manual deserialization validates the v1 schema marker before constructing the
// domain receipt.
impl<'de> Deserialize<'de> for QualityReceiptV1 {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let wire = Wire::deserialize(de)?;
        validate_schema_version::<D::Error>(wire.schema_version)?;
        Self::new(
            wire.scope,
            ReceiptDigests::new(
                wire.source_digest,
                wire.cargo_lock_digest,
                wire.policy_digest,
                wire.toolchain_digest,
            ),
            wire.lanes,
        )
        .map_err(serde::de::Error::custom)
    }
}
/// Validate a receipt schema version against the v1 wire contract.
///
/// # Errors
///
/// Returns a serde error when the payload schema version is not v1.
fn validate_schema_version<E: serde::de::Error>(schema_version: u16) -> Result<(), E> {
    (schema_version == V1_SCHEMA_VERSION).then_some(()).ok_or_else(|| {
        E::custom(format!(
            "unsupported schema version: expected {V1_SCHEMA_VERSION}, got {schema_version}"
        ))
    })
}

#[derive(serde::Deserialize)]
struct Wire {
    schema_version: u16,
    scope: GateScope,
    source_digest: Digest,
    cargo_lock_digest: Digest,
    policy_digest: Digest,
    toolchain_digest: Digest,
    lanes: Box<[LaneReceipt]>,
}
/// v1 Quality Receipt — stable evidence envelope for a gate run.
///
/// This is the spec §10 `QualityReceipt`, distinct from the existing
/// `receipt::QualityReceipt` which tracks per-lane digests for receipt files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QualityReceiptV1 {
    /// Schema version. Always `1` for v1.
    pub schema_version: u16,
    /// Which scope was run.
    pub scope: GateScope,
    /// Blake3 digest of the source tree.
    pub source_digest: Digest,
    /// Blake3 digest of Cargo.lock.
    pub cargo_lock_digest: Digest,
    /// Blake3 digest of the policy config.
    pub policy_digest: Digest,
    /// Blake3 digest of the toolchain (rustc + cargo versions).
    pub toolchain_digest: Digest,
    /// Per-lane receipt summaries.
    pub lanes: Box<[LaneReceipt]>,
}
impl QualityReceiptV1 {
    /// Construct a v1 [`QualityReceiptV1`]. Always uses `schema_version = 1`.
    ///
    /// # Errors
    /// - [`ReceiptError::EmptyLaneReceiptList`] if `lanes` is empty.
    pub fn new(
        scope: GateScope,
        digests: ReceiptDigests,
        lanes: Box<[LaneReceipt]>,
    ) -> Result<Self, ReceiptError> {
        let (source, lock, policy, toolchain) = digests.into_parts();
        let lanes =
            (!lanes.is_empty()).then_some(lanes).ok_or(ReceiptError::EmptyLaneReceiptList)?;
        Ok(Self {
            schema_version: V1_SCHEMA_VERSION,
            scope,
            source_digest: source,
            cargo_lock_digest: lock,
            policy_digest: policy,
            toolchain_digest: toolchain,
            lanes,
        })
    }
}

/// Schema version constant for v1 Quality Receipts.
const V1_SCHEMA_VERSION: u16 = 1;
