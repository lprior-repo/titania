#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

pub use crate::assure::ids::{
    AssumptionId, BeadId, ClaimCeilingId, ContractClauseId, Digest, EnumVariantId, EvidenceId,
    ObligationId, OracleId, PathId, TcbId, TypeId, VarId,
};

pub type AssureResult<T> = Result<T, AssureError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssureError {
    EmptyId { kind: &'static str },
    InvalidId { kind: &'static str, value: String },
    EmptyEnumDomain { var: String },
    UnknownVar { var: String },
    InvalidEnumVariant { var: String, value: String },
    CounterOverflow { context: &'static str },
}

impl fmt::Display for AssureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId { kind } => write!(formatter, "{kind} must not be empty"),
            Self::InvalidId { kind, value } => {
                write!(formatter, "{kind} contains unsupported characters: {value}")
            }
            Self::EmptyEnumDomain { var } => {
                write!(formatter, "finite fact domain has no variants: {var}")
            }
            Self::UnknownVar { var } => write!(formatter, "unknown finite fact variable: {var}"),
            Self::InvalidEnumVariant { var, value } => {
                write!(
                    formatter,
                    "invalid enum variant {value} for finite fact {var}"
                )
            }
            Self::CounterOverflow { context } => {
                write!(formatter, "assurance counter overflow during {context}")
            }
        }
    }
}

impl std::error::Error for AssureError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeExpr {
    Bool,
    Enum {
        name: TypeId,
        variants: Vec<EnumVariantId>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactDomain {
    pub var: VarId,
    pub ty: TypeExpr,
}

impl FactDomain {
    pub fn enum_domain(
        var: VarId,
        name: TypeId,
        variants: Vec<EnumVariantId>,
    ) -> AssureResult<Self> {
        if variants.is_empty() {
            return Err(AssureError::EmptyEnumDomain {
                var: var.as_str().to_string(),
            });
        }
        Ok(Self {
            var,
            ty: TypeExpr::Enum { name, variants },
        })
    }

    pub fn enum_variants(&self) -> AssureResult<&[EnumVariantId]> {
        match &self.ty {
            TypeExpr::Enum { variants, .. } if variants.is_empty() => {
                Err(AssureError::EmptyEnumDomain {
                    var: self.var.as_str().to_string(),
                })
            }
            TypeExpr::Enum { variants, .. } => Ok(variants.as_slice()),
            TypeExpr::Bool => Err(AssureError::InvalidEnumVariant {
                var: self.var.as_str().to_string(),
                value: "bool-domain-not-supported-in-v1-enumeration".to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expr {
    True,
    False,
    Eq { var: VarId, value: EnumVariantId },
    Neq { var: VarId, value: EnumVariantId },
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Not(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedOutcome {
    Ok { value: String },
    Err { error: String },
}

impl ExpectedOutcome {
    #[must_use]
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok { .. })
    }

    #[must_use]
    pub fn is_err(&self) -> bool {
        matches!(self, Self::Err { .. })
    }

    #[must_use]
    pub fn error_name(&self) -> Option<&str> {
        match self {
            Self::Err { error } => Some(error.as_str()),
            Self::Ok { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionPath {
    pub id: PathId,
    pub guard: Expr,
    pub outcome: ExpectedOutcome,
    pub maps_to: Vec<ContractClauseId>,
}
