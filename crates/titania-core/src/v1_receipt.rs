//! v1-quality receipt types from v1-spec.md §10.
//!
//! These are distinct from the existing receipt.rs types which serve a different
//! purpose (per-lane digest tracking). `LaneReceipt` and the v1 `QualityReceipt`
//! are used in the Report domain model.

use serde::{Deserialize, Serialize};

use crate::{Digest, GateScope, Lane, ReceiptError};

/// Per-lane receipt summary inside a [`QualityReceipt`].
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

    /// Lane covered by this receipt.
    #[must_use]
    pub const fn lane(&self) -> Lane {
        self.lane
    }

    /// Digest of the lane evidence.
    #[must_use]
    pub const fn evidence_digest(&self) -> &Digest {
        &self.evidence_digest
    }

    /// Whether the lane was clean.
    #[must_use]
    pub const fn clean(&self) -> bool {
        self.clean
    }
}

/// v1 Quality Receipt — stable evidence envelope for a gate run.
///
/// This is the spec §10 `QualityReceipt`, distinct from the existing
/// `receipt::QualityReceipt` which tracks per-lane digests for receipt files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
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
    #[allow(clippy::too_many_arguments)]
    /// Construct a v1 [`QualityReceipt`]. Always uses `schema_version = 1`.
    #[must_use]
    pub const fn new(
        scope: GateScope,
        source_digest: Digest,
        cargo_lock_digest: Digest,
        policy_digest: Digest,
        toolchain_digest: Digest,
        lanes: Box<[LaneReceipt]>,
    ) -> Self {
        Self {
            schema_version: RECEIPT_SCHEMA_VERSION,
            scope,
            source_digest,
            cargo_lock_digest,
            policy_digest,
            toolchain_digest,
            lanes,
        }
    }

    /// Schema version.
    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    /// Gate scope.
    #[must_use]
    pub const fn scope(&self) -> GateScope {
        self.scope
    }

    /// Source digest.
    #[must_use]
    pub const fn source_digest(&self) -> &Digest {
        &self.source_digest
    }

    /// Cargo.lock digest.
    #[must_use]
    pub const fn cargo_lock_digest(&self) -> &Digest {
        &self.cargo_lock_digest
    }

    /// Policy digest.
    #[must_use]
    pub const fn policy_digest(&self) -> &Digest {
        &self.policy_digest
    }

    /// Toolchain digest.
    #[must_use]
    pub const fn toolchain_digest(&self) -> &Digest {
        &self.toolchain_digest
    }

    /// Per-lane receipts.
    #[must_use]
    pub const fn lanes(&self) -> &[LaneReceipt] {
        &self.lanes
    }
}

#[derive(Deserialize)]
struct QualityReceiptWire {
    schema_version: u16,
    scope: GateScope,
    source_digest: Digest,
    cargo_lock_digest: Digest,
    policy_digest: Digest,
    toolchain_digest: Digest,
    lanes: Box<[LaneReceipt]>,
}

impl<'de> Deserialize<'de> for QualityReceiptV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = QualityReceiptWire::deserialize(deserializer)?;
        if wire.schema_version != RECEIPT_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(ReceiptError::UnsupportedSchemaVersion(
                u32::from(wire.schema_version),
            )));
        }
        Ok(Self::new(
            wire.scope,
            wire.source_digest,
            wire.cargo_lock_digest,
            wire.policy_digest,
            wire.toolchain_digest,
            wire.lanes,
        ))
    }
}

/// Schema version constant for v1 Quality Receipts.
const RECEIPT_SCHEMA_VERSION: u16 = 1;
