use serde::{Deserialize, Serialize};

use crate::{Digest, GateScope, Lane, error::ReceiptError, receipt::ReceiptDigests};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct LaneReceipt(LaneReceiptInner);

impl LaneReceipt {
    #[must_use]
    pub const fn new(lane: Lane, evidence_digest: Digest, clean: bool) -> Self {
        Self(LaneReceiptInner { lane, evidence_digest, clean })
    }

    #[must_use]
    pub const fn lane(&self) -> &Lane {
        &self.0.lane
    }

    #[must_use]
    pub const fn evidence_digest(&self) -> &Digest {
        &self.0.evidence_digest
    }

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct QualityReceiptV1(QualityReceiptV1Inner);

impl QualityReceiptV1 {
    pub const SCHEMA_VERSION: u16 = 1;

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

    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.0.schema_version
    }

    #[must_use]
    pub const fn scope(&self) -> &GateScope {
        &self.0.scope
    }

    #[must_use]
    pub const fn source_digest(&self) -> &Digest {
        &self.0.source_digest
    }

    #[must_use]
    pub const fn cargo_lock_digest(&self) -> &Digest {
        &self.0.cargo_lock_digest
    }

    #[must_use]
    pub const fn policy_digest(&self) -> &Digest {
        &self.0.policy_digest
    }

    #[must_use]
    pub const fn toolchain_digest(&self) -> &Digest {
        &self.0.toolchain_digest
    }

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
