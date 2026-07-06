//! v1-quality receipt types from v1-spec.md §10.
//!
//! These are distinct from the existing receipt.rs types which serve a different
//! purpose (per-lane digest tracking). `LaneReceipt` and the v1 `QualityReceipt`
//! are used in the Report domain model.
//!
//! ## Schema-version canonical name
//!
//! The canonical schema version for the spec §10 `QualityReceipt` is
//! [`QualityReceiptV1::SCHEMA_VERSION`]: a `u16 = 1`. It is intentionally
//! distinct from [`crate::RECEIPT_ENVELOPE_SCHEMA_VERSION`] (a `u32 = 2`),
//! which versions the separate internal [`crate::ReceiptEnvelope`] wire shape
//! and is NOT part of v1-spec.md §10.

use serde::{Deserialize, Serialize};

use crate::{Digest, GateScope, Lane, error::ReceiptError, receipt::ReceiptDigests};

/// Per-lane receipt summary inside a [`QualityReceiptV1`].
///
/// Constructed via [`LaneReceipt::new`] — direct field access is forbidden
/// to prevent illegal state construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct LaneReceipt(LaneReceiptInner);

impl LaneReceipt {
    /// Construct a new [`LaneReceipt`].
    #[must_use]
    pub const fn new(lane: Lane, evidence_digest: Digest, clean: bool) -> Self {
        Self(LaneReceiptInner { lane, evidence_digest, clean })
    }

    /// Which lane this receipt covers.
    #[must_use]
    pub const fn lane(&self) -> &Lane {
        &self.0.lane
    }

    /// Blake3 digest of the lane's `LaneEvidence`.
    #[must_use]
    pub const fn evidence_digest(&self) -> &Digest {
        &self.0.evidence_digest
    }

    /// Whether the lane produced zero findings.
    #[must_use]
    pub const fn clean(&self) -> bool {
        self.0.clean
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
struct LaneReceiptInner {
    lane: Lane,
    evidence_digest: Digest,
    clean: bool,
}

impl<'de> Deserialize<'de> for LaneReceipt {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let inner = LaneReceiptInner::deserialize(de)?;
        Ok(Self(inner))
    }
}

/// Validate a receipt schema version against the v1 wire contract.
///
/// # Errors
///
/// Returns a serde error when the payload schema version is not v1.
fn validate_schema_version<E: serde::de::Error>(schema_version: u16) -> Result<(), E> {
    (schema_version == QualityReceiptV1::SCHEMA_VERSION).then_some(()).ok_or_else(|| {
        E::custom(format!(
            "unsupported schema version: expected {}, got {schema_version}",
            QualityReceiptV1::SCHEMA_VERSION,
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
/// v1 Quality Receipt — stable evidence envelope for a gate run.
///
/// This is the spec §10 `QualityReceipt`, distinct from the existing
/// `receipt::QualityReceipt` which tracks per-lane digests for receipt files.
///
/// Constructed via [`QualityReceiptV1::new`] — direct field access is forbidden
/// to prevent illegal state construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct QualityReceiptV1(QualityReceiptV1Inner);

impl QualityReceiptV1 {
    /// Canonical schema version for the spec §10 `QualityReceipt`.
    ///
    /// Always `1` for v1. This is the single source of truth for the
    /// `schema_version` field emitted in the wire JSON. Compare with
    /// [`crate::RECEIPT_ENVELOPE_SCHEMA_VERSION`] only when validating the
    /// separate internal [`crate::ReceiptEnvelope`] namespace.
    pub const SCHEMA_VERSION: u16 = 1;

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
        Ok(Self(QualityReceiptV1Inner {
            schema_version: Self::SCHEMA_VERSION,
            scope,
            source_digest: source,
            cargo_lock_digest: lock,
            policy_digest: policy,
            toolchain_digest: toolchain,
            lanes,
        }))
    }

    /// Schema version. Always `1` for v1.
    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.0.schema_version
    }

    /// Which scope was run.
    #[must_use]
    pub const fn scope(&self) -> &GateScope {
        &self.0.scope
    }

    /// Blake3 digest of the source tree.
    #[must_use]
    pub const fn source_digest(&self) -> &Digest {
        &self.0.source_digest
    }

    /// Blake3 digest of Cargo.lock.
    #[must_use]
    pub const fn cargo_lock_digest(&self) -> &Digest {
        &self.0.cargo_lock_digest
    }

    /// Blake3 digest of the policy config.
    #[must_use]
    pub const fn policy_digest(&self) -> &Digest {
        &self.0.policy_digest
    }

    /// Blake3 digest of the toolchain (rustc + cargo versions).
    #[must_use]
    pub const fn toolchain_digest(&self) -> &Digest {
        &self.0.toolchain_digest
    }

    /// Per-lane receipt summaries.
    #[must_use]
    pub const fn lanes(&self) -> &[LaneReceipt] {
        &self.0.lanes
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
struct QualityReceiptV1Inner {
    schema_version: u16,
    scope: GateScope,
    source_digest: Digest,
    cargo_lock_digest: Digest,
    policy_digest: Digest,
    toolchain_digest: Digest,
    lanes: Box<[LaneReceipt]>,
}

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
