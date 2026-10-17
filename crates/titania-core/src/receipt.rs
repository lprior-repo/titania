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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptLaneExit {
    Clean,
    Violations,
    Usage,
    Failure,
}

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

    #[must_use]
    pub const fn lane(&self) -> &LaneName {
        &self.lane
    }

    #[must_use]
    pub const fn exit(&self) -> ReceiptLaneExit {
        self.exit
    }

    #[must_use]
    pub const fn scanned(&self) -> u32 {
        self.scanned
    }

    #[must_use]
    pub const fn passed(&self) -> u32 {
        self.passed
    }

    #[must_use]
    pub const fn finding_count(&self) -> u32 {
        self.finding_count
    }
}

fn check_passed_not_above_scanned(passed: u32, scanned: u32) -> Result<(), ReceiptError> {
    (passed <= scanned).then_some(()).ok_or(ReceiptError::PassedExceedsScanned { passed, scanned })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiptPeriod {
    started_at: u64,
    finished_at: u64,
}

impl ReceiptPeriod {
    pub fn new(started_at: u64, finished_at: u64) -> Result<Self, ReceiptError> {
        (finished_at >= started_at)
            .then_some(Self { started_at, finished_at })
            .ok_or(ReceiptError::FinishedBeforeStarted { started_at, finished_at })
    }

    #[must_use]
    pub const fn started_at(self) -> u64 {
        self.started_at
    }

    #[must_use]
    pub const fn finished_at(self) -> u64 {
        self.finished_at
    }
}

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

fn check_supported_schema_version(schema_version: u32) -> Result<(), ReceiptError> {
    (schema_version == RECEIPT_ENVELOPE_SCHEMA_VERSION)
        .then_some(())
        .ok_or(ReceiptError::UnsupportedSchemaVersion(schema_version))
}

impl ReceiptEnvelope {
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

    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    #[must_use]
    pub const fn target_root(&self) -> &RecordedTargetRoot {
        &self.target_root
    }

    #[must_use]
    pub const fn started_at(&self) -> u64 {
        self.started_at
    }

    #[must_use]
    pub const fn finished_at(&self) -> u64 {
        self.finished_at
    }

    #[must_use]
    pub fn lane_results(&self) -> &[LaneDigest] {
        &self.lane_results
    }

    #[must_use]
    pub const fn source_digest(&self) -> &Digest {
        &self.source_digest
    }

    #[must_use]
    pub const fn lock_digest(&self) -> &Digest {
        &self.lock_digest
    }

    #[must_use]
    pub const fn policy_digest(&self) -> &Digest {
        &self.policy_digest
    }

    #[must_use]
    pub const fn toolchain_digest(&self) -> &Digest {
        &self.toolchain_digest
    }
}
