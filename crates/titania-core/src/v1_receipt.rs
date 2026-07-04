//! v1-quality receipt types from v1-spec.md §10.
//!
//! These are distinct from the existing receipt.rs types which serve a different
//! purpose (per-lane digest tracking). `LaneReceipt` and the v1 `QualityReceipt`
//! are used in the Report domain model.

use serde::{Deserialize, Serialize};

use crate::{Digest, GateScope, Lane};

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

/// v1 Quality Receipt — stable evidence envelope for a gate run.
///
/// This is the spec §10 `QualityReceipt`, distinct from the existing
/// `receipt::QualityReceipt` which tracks per-lane digests for receipt files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[expect(
        clippy::too_many_arguments,
        reason = "The v1 receipt schema has four independent digest fields plus scope and lane summaries."
    )]
    /// Construct a v1 [`QualityReceiptV1`]. Always uses `schema_version = 1`.
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
}

/// Schema version constant for v1 Quality Receipts.
const RECEIPT_SCHEMA_VERSION: u16 = 1;
