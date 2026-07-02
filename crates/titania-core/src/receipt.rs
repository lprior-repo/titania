//! Quality receipts for target-project gate runs.
//!
//! A receipt is the stable, serializable evidence envelope that says which
//! project was judged and which digests/results were observed. Construction
//! keeps invalid lane summaries and unsupported schemas out of the domain.

use serde::{Deserialize, Deserializer, Serialize};

use crate::{Digest, TargetProject, error::ReceiptError};
mod digests;
mod lane_name;
mod schema;
mod serde_support;
mod target_root;

pub use digests::ReceiptDigests;
pub use lane_name::LaneName;
pub use schema::RECEIPT_SCHEMA_VERSION;
pub use target_root::RecordedTargetRoot;

/// Receipt-local subprocess outcome.
///
/// Mirrors the lane exit-code contract without making `titania-core` depend
/// on the `titania-lanes` crate. `Clean` and `NotApplicable` are both
/// process-success dispositions but carry distinct report meaning: a clean
/// lane scanned and passed; a non-applicable lane had no valid subject
/// to judge. `Violations`, `Usage`, and `Failure` map to non-zero process
/// exits with distinct semantics (findings, config error, upstream failure).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptLaneExit {
    /// Lane scanned and emitted zero findings.
    Clean,
    /// Lane had no valid subject to judge; process exit still 0.
    NotApplicable,
    /// Lane emitted at least one finding; process exit 1.
    Violations,
    /// Lane was invoked with bad arguments or config; process exit 2.
    Usage,
    /// Lane could not run because of an upstream or fixture failure;
    /// process exit 3.
    Failure,
}

/// Per-lane digest summary embedded in a [`QualityReceipt`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct LaneDigest {
    /// Stable lane name this digest summarizes.
    lane: LaneName,
    /// Process/disposition outcome the lane reported.
    exit: ReceiptLaneExit,
    /// Number of items the lane scanned.
    scanned: u32,
    /// Number of items the lane accepted as clean.
    passed: u32,
    /// Number of findings the lane emitted.
    finding_count: u32,
}

#[derive(Deserialize)]
struct LaneDigestWire {
    /// Stable lane name this digest summarizes.
    lane: LaneName,
    /// Process/disposition outcome the lane reported.
    exit: ReceiptLaneExit,
    /// Number of items the lane scanned.
    scanned: u32,
    /// Number of items the lane accepted as clean.
    passed: u32,
    /// Number of findings the lane emitted.
    finding_count: u32,
}

impl<'de> Deserialize<'de> for LaneDigest {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let wire = LaneDigestWire::deserialize(de)?;
        Self::new(wire.lane, wire.exit, wire.scanned, wire.passed, wire.finding_count)
            .map_err(serde::de::Error::custom)
    }
}

/// Verify that `passed <= scanned`, returning `Some(())` on success.
#[must_use]
const fn passed_le_scanned(passed: u32, scanned: u32) -> Option<()> {
    if passed <= scanned { Some(()) } else { None }
}
impl LaneDigest {
    /// Construct a validated per-lane receipt summary.
    ///
    /// # Errors
    /// - [`ReceiptError::PassedExceedsScanned`] if `passed > scanned`.
    pub fn new(
        lane: LaneName,
        exit: ReceiptLaneExit,
        scanned: u32,
        passed: u32,
        finding_count: u32,
    ) -> Result<Self, ReceiptError> {
        let Some(()) = passed_le_scanned(passed, scanned) else {
            return Err(ReceiptError::PassedExceedsScanned { passed, scanned });
        };
        Ok(Self {
            lane,
            exit,
            scanned,
            passed,
            finding_count,
        })
    }

    /// Lane name.
    #[must_use]
    pub const fn lane(&self) -> &LaneName {
        &self.lane
    }

    /// Lane exit outcome.
    #[must_use]
    pub const fn exit(&self) -> ReceiptLaneExit {
        self.exit
    }

    /// Files/items scanned by the lane.
    #[must_use]
    pub const fn scanned(&self) -> u32 {
        self.scanned
    }

    /// Files/items accepted by the lane.
    #[must_use]
    pub const fn passed(&self) -> u32 {
        self.passed
    }

    /// Findings emitted by the lane.
    #[must_use]
    pub const fn finding_count(&self) -> u32 {
        self.finding_count
    }
}

/// Validated start and finish timestamps for a receipt run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiptPeriod {
    /// Unix-second timestamp at run start.
    started_at: u64,
    /// Unix-second timestamp at run finish; guaranteed `>= started_at`.
    finished_at: u64,
}

/// Verify that `finished_at >= started_at`, returning `Some(())` on success.
#[must_use]
const fn finished_ge_started(finished_at: u64, started_at: u64) -> Option<()> {
    if finished_at >= started_at { Some(()) } else { None }
}
/// Accessors that take `&self` for call-site uniformity.
impl ReceiptPeriod {
    #![allow(
        clippy::trivially_copy_pass_by_ref,
        reason = "Accessors take &self for call-site uniformity with the rest of the public API."
    )]
    /// Construct receipt timing from Unix-second timestamps.
    ///
    /// # Errors
    /// - [`ReceiptError::FinishedBeforeStarted`] if `finished_at < started_at`.
    pub const fn new(started_at: u64, finished_at: u64) -> Result<Self, ReceiptError> {
        match finished_ge_started(finished_at, started_at) {
            Some(()) => Ok(Self { started_at, finished_at }),
            None => Err(ReceiptError::FinishedBeforeStarted {
                started_at,
                finished_at,
            }),
        }
    }

    /// Run start time, in Unix seconds.
    #[must_use]
    pub const fn started_at(&self) -> u64 {
        self.started_at
    }

    /// Run finish time, in Unix seconds.
    #[must_use]
    pub const fn finished_at(&self) -> u64 {
        self.finished_at
    }
}

/// Confirm a deserialized schema version matches the current schema.
#[must_use]
const fn schema_matches(version: u32) -> Option<()> {
    if version == RECEIPT_SCHEMA_VERSION { Some(()) } else { None }
}

/// Stable quality receipt envelope for one target-project run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QualityReceipt {
    /// Schema version this receipt was serialized with.
    schema_version: u32,
    /// Target project root that was judged.
    target_root: RecordedTargetRoot,
    /// Unix-second timestamp at run start.
    started_at: u64,
    /// Unix-second timestamp at run finish.
    finished_at: u64,
    /// Per-lane digest summaries, in execution order.
    lane_results: Vec<LaneDigest>,
    /// blake3 digest of the source tree at evaluation time.
    source_digest: Digest,
    /// blake3 digest of the resolved `Cargo.lock`.
    lock_digest: Digest,
    /// blake3 digest of the policy/profiles content.
    policy_digest: Digest,
    /// blake3 digest of the toolchain manifest content.
    toolchain_digest: Digest,
}

impl QualityReceipt {
    /// Construct a receipt produced by the current schema.
    ///
    /// # Errors
    /// - [`ReceiptError::FinishedBeforeStarted`] if `finished_at < started_at`.
    pub fn new(
        target_root: &TargetProject,
        period: ReceiptPeriod,
        lane_results: Vec<LaneDigest>,
        digests: ReceiptDigests,
    ) -> Result<Self, ReceiptError> {
        Self::from_parts(
            RECEIPT_SCHEMA_VERSION,
            RecordedTargetRoot::from_target_project(target_root),
            period,
            lane_results,
            digests,
        )
    }

    /// Build a [`QualityReceipt`] from already-validated parts. Used by
    /// `new` and by the deserializer round-trip path.
    ///
    /// # Errors
    /// Returns [`ReceiptError::UnsupportedSchemaVersion`] when the supplied
    /// schema version does not match [`RECEIPT_SCHEMA_VERSION`].
    fn from_parts(
        schema_version: u32,
        target_root: RecordedTargetRoot,
        period: ReceiptPeriod,
        lane_results: Vec<LaneDigest>,
        digests: ReceiptDigests,
    ) -> Result<Self, ReceiptError> {
        let Some(()) = schema_matches(schema_version) else {
            return Err(ReceiptError::UnsupportedSchemaVersion(schema_version));
        };
        let ReceiptPeriod { started_at, finished_at } = period;
        let (source_digest, lock_digest, policy_digest, toolchain_digest) = digests.into_parts();
        Ok(Self {
            schema_version,
            target_root,
            started_at,
            finished_at,
            lane_results,
            source_digest,
            lock_digest,
            policy_digest,
            toolchain_digest,
        })
    }

    /// Receipt schema version.
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Target project that was judged.
    #[must_use]
    pub const fn target_root(&self) -> &RecordedTargetRoot {
        &self.target_root
    }

    /// Run start time, in Unix seconds.
    #[must_use]
    pub const fn started_at(&self) -> u64 {
        self.started_at
    }

    /// Run finish time, in Unix seconds.
    #[must_use]
    pub const fn finished_at(&self) -> u64 {
        self.finished_at
    }

    /// Per-lane summaries.
    #[must_use]
    pub fn lane_results(&self) -> &[LaneDigest] {
        &self.lane_results
    }

    /// Source digest.
    #[must_use]
    pub const fn source_digest(&self) -> &Digest {
        &self.source_digest
    }

    /// Cargo.lock digest.
    #[must_use]
    pub const fn lock_digest(&self) -> &Digest {
        &self.lock_digest
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
}