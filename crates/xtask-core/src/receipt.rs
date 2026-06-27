//! `QualityReceipt`: deterministic record of what passed.

use serde::{Deserialize, Serialize};

use crate::lane::{GateScope, LaneReceipt};

/// Deterministic, unsigned record of what quality checks passed.
///
/// No signature, no expiry, no deploy semantics.
/// CI enforces by running `xtask gate --scope full` and checking exit code.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QualityReceipt {
    /// Schema version for forward compatibility.
    pub schema_version: u16,
    /// Which scope was run.
    pub scope: GateScope,
    /// BLAKE3 of the gated source tree (canonical file set).
    pub source_digest: crate::Digest,
    /// BLAKE3 of Cargo.lock.
    pub cargo_lock_digest: crate::Digest,
    /// BLAKE3 of all policy files.
    pub policy_digest: crate::Digest,
    /// BLAKE3 of the resolved toolchain.
    pub toolchain_digest: crate::Digest,
    /// BLAKE3 of vendored dependency sources (if supply-chain lane ran).
    pub dependency_source_digest: Option<crate::Digest>,
    /// BLAKE3 of the pinned advisory DB snapshot (if supply-chain lane ran).
    pub advisory_db_digest: Option<crate::Digest>,
    /// BLAKE3 of the feature profile (if feature-matrix lane ran).
    pub feature_profile_digest: Option<crate::Digest>,
    /// BLAKE3 of the mutation baseline (if mutants lane ran).
    pub mutation_baseline_digest: Option<crate::Digest>,
    /// Per-lane receipts.
    pub lanes: Box<[LaneReceipt]>,
}
