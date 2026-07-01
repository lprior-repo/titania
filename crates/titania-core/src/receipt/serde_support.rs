use serde::{Deserialize, Deserializer};

use super::{
    LaneDigest, ReceiptDigests, ReceiptEnvelope, ReceiptPeriod, RecordedTargetRoot,
    schema::is_supported_receipt_schema_version,
};
use crate::{Digest, error::ReceiptError};

#[derive(Deserialize)]
struct ReceiptEnvelopeSchemaWire {
    schema_version: u32,
}

#[derive(Deserialize)]
struct ReceiptEnvelopeWire {
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

impl<'de> Deserialize<'de> for ReceiptEnvelope {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(de)?;
        let schema =
            ReceiptEnvelopeSchemaWire::deserialize(&value).map_err(serde::de::Error::custom)?;
        if !is_supported_receipt_schema_version(schema.schema_version) {
            return Err(serde::de::Error::custom(ReceiptError::UnsupportedSchemaVersion(
                schema.schema_version,
            )));
        }
        let wire = ReceiptEnvelopeWire::deserialize(value).map_err(serde::de::Error::custom)?;
        Self::from_parts(
            wire.schema_version,
            wire.target_root,
            ReceiptPeriod::new(wire.started_at, wire.finished_at)
                .map_err(serde::de::Error::custom)?,
            wire.lane_results,
            ReceiptDigests::new(
                wire.source_digest,
                wire.lock_digest,
                wire.policy_digest,
                wire.toolchain_digest,
            ),
        )
        .map_err(serde::de::Error::custom)
    }
}
