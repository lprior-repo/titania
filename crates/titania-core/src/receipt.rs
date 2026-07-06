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
pub use schema::RECEIPT_ENVELOPE_SCHEMA_VERSION;
pub use target_root::RecordedTargetRoot;

/// Receipt-local subprocess outcome.
///
/// This mirrors the lane exit-code contract without making `titania-core`
/// depend on the `titania-lanes` crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptLaneExit {
    /// Lane completed with no violations.
    Clean,
    /// Lane completed and found policy violations.
    Violations,
    /// Lane could not run because invocation or CLI usage was invalid.
    Usage,
    /// Lane execution failed before a verdict could be produced.
    Failure,
}

/// Per-lane digest summary embedded in a [`ReceiptEnvelope`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct LaneDigest {
    lane: LaneName,
    exit: ReceiptLaneExit,
    scanned: u32,
    passed: u32,
    finding_count: u32,
}

#[derive(Deserialize)]
struct LaneDigestWire {
    lane: LaneName,
    exit: ReceiptLaneExit,
    scanned: u32,
    passed: u32,
    finding_count: u32,
}

impl<'de> Deserialize<'de> for LaneDigest {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let wire = LaneDigestWire::deserialize(de)?;
        Self::new(wire.lane, wire.exit, wire.scanned, wire.passed, wire.finding_count)
            .map_err(serde::de::Error::custom)
    }
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
        check_passed_not_above_scanned(passed, scanned)?;
        Ok(Self { lane, exit, scanned, passed, finding_count })
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

/// Check that a lane did not pass more items than it scanned.
///
/// # Errors
/// Returns [`ReceiptError::PassedExceedsScanned`] when `passed > scanned`.
fn check_passed_not_above_scanned(passed: u32, scanned: u32) -> Result<(), ReceiptError> {
    (passed <= scanned).then_some(()).ok_or(ReceiptError::PassedExceedsScanned { passed, scanned })
}

/// Validated start and finish timestamps for a receipt run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiptPeriod {
    started_at: u64,
    finished_at: u64,
}

impl ReceiptPeriod {
    /// Construct receipt timing from Unix-second timestamps.
    ///
    /// # Errors
    /// - [`ReceiptError::FinishedBeforeStarted`] if `finished_at < started_at`.
    pub fn new(started_at: u64, finished_at: u64) -> Result<Self, ReceiptError> {
        (finished_at >= started_at)
            .then_some(Self { started_at, finished_at })
            .ok_or(ReceiptError::FinishedBeforeStarted { started_at, finished_at })
    }

    /// Run start time, in Unix seconds.
    #[must_use]
    pub const fn started_at(self) -> u64 {
        self.started_at
    }

    /// Run finish time, in Unix seconds.
    #[must_use]
    pub const fn finished_at(self) -> u64 {
        self.finished_at
    }
}

/// Stable quality receipt envelope for one target-project run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReceiptEnvelope {
    schema_version: u32,
    target_root: RecordedTargetRoot,
    started_at: u64,
    finished_at: u64,
    lane_results: Vec<LaneDigest>,
    source_digest: Digest,
    lock_digest: Digest,
    policy_digest: Digest,
    toolchain_digest: Digest,
}

/// Check receipt schema support.
///
/// # Errors
/// Returns [`ReceiptError::UnsupportedSchemaVersion`] when the schema version
/// is not the current v1 receipt schema.
fn check_supported_schema_version(schema_version: u32) -> Result<(), ReceiptError> {
    (schema_version == RECEIPT_ENVELOPE_SCHEMA_VERSION)
        .then_some(())
        .ok_or(ReceiptError::UnsupportedSchemaVersion(schema_version))
}

impl ReceiptEnvelope {
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
            RECEIPT_ENVELOPE_SCHEMA_VERSION,
            RecordedTargetRoot::from_target_project(target_root),
            period,
            lane_results,
            digests,
        )
    }

    /// Construct a receipt from explicit schema and normalized parts.
    ///
    /// # Errors
    /// - [`ReceiptError::UnsupportedSchemaVersion`] if `schema_version` is not current.
    /// - [`ReceiptError::FinishedBeforeStarted`] if `period` has invalid ordering.
    fn from_parts(
        schema_version: u32,
        target_root: RecordedTargetRoot,
        period: ReceiptPeriod,
        lane_results: Vec<LaneDigest>,
        digests: ReceiptDigests,
    ) -> Result<Self, ReceiptError> {
        check_supported_schema_version(schema_version)?;
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
