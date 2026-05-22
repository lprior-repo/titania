#![forbid(unsafe_code)]

use crate::assure::model::{AssureError, AssureResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VarId(String);

impl VarId {
    pub fn new(value: impl Into<String>) -> AssureResult<Self> {
        validate_id("var id", value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EnumVariantId(String);

impl EnumVariantId {
    pub fn new(value: impl Into<String>) -> AssureResult<Self> {
        validate_id("enum variant id", value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TypeId(String);

impl TypeId {
    pub fn new(value: impl Into<String>) -> AssureResult<Self> {
        validate_id("type id", value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PathId(String);

impl PathId {
    pub fn new(value: impl Into<String>) -> AssureResult<Self> {
        validate_id("path id", value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ContractClauseId(String);

impl ContractClauseId {
    pub fn new(value: impl Into<String>) -> AssureResult<Self> {
        validate_id("contract clause id", value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OracleId(String);

impl OracleId {
    pub fn new(value: impl Into<String>) -> AssureResult<Self> {
        validate_id("oracle id", value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BeadId(String);

impl BeadId {
    pub fn new(value: impl Into<String>) -> AssureResult<Self> {
        validate_id("bead id", value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Digest(String);

impl Digest {
    pub fn new(value: impl Into<String>) -> AssureResult<Self> {
        validate_id("digest", value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EvidenceId(String);

impl EvidenceId {
    pub fn new(value: impl Into<String>) -> AssureResult<Self> {
        validate_id("evidence id", value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObligationId(String);

impl ObligationId {
    pub fn new(value: impl Into<String>) -> AssureResult<Self> {
        validate_id("obligation id", value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ClaimCeilingId(String);

impl ClaimCeilingId {
    pub fn new(value: impl Into<String>) -> AssureResult<Self> {
        validate_id("claim ceiling id", value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TcbId(String);

impl TcbId {
    pub fn new(value: impl Into<String>) -> AssureResult<Self> {
        validate_id("TCB id", value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssumptionId(String);

impl AssumptionId {
    pub fn new(value: impl Into<String>) -> AssureResult<Self> {
        validate_id("assumption id", value).map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn validate_id(kind: &'static str, value: impl Into<String>) -> AssureResult<String> {
    let value = value.into();
    if value.is_empty() {
        return Err(AssureError::EmptyId { kind });
    }
    if value.chars().all(is_supported_id_char) {
        Ok(value)
    } else {
        Err(AssureError::InvalidId { kind, value })
    }
}

fn is_supported_id_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.' | '/')
}
