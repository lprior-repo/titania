#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustMarker {
    AssumeSpecification,
    Assume,
    ExternalBody,
    Admit,
    Trusted,
}

impl TrustMarker {
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::AssumeSpecification, Self::Assume, Self::ExternalBody, Self::Admit, Self::Trusted]
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::AssumeSpecification => "assume_specification",
            Self::Assume => "assume",
            Self::ExternalBody => "external_body",
            Self::Admit => "admit",
            Self::Trusted => "trusted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    SmokeOnly,
    UnapprovedTrust,
    MissingBinding,
    MirrorDrift,
    VacuousProof,
    MissingEvidence,
}

impl BlockReason {
    #[must_use]
    pub const fn rule_id(self) -> &'static str {
        match self {
            Self::SmokeOnly => "TRUST_SMOKE_COVERAGE",
            Self::UnapprovedTrust => "TRUST_UNAPPROVED_MARKER",
            Self::MissingBinding => "TRUST_MISSING_BINDING",
            Self::MirrorDrift => "TRUST_MIRROR_DRIFT",
            Self::VacuousProof => "TRUST_VACUOUS_PROOF",
            Self::MissingEvidence => "TRUST_MISSING_EVIDENCE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingVerdict {
    VerifiedProduction { binding: String, trusted_base: String },
    SourceLinked { waived_markers: Vec<TrustMarker> },
    Blocked { reason: BlockReason },
}

impl BindingVerdict {
    #[must_use]
    pub const fn is_verified_production(&self) -> bool {
        matches!(self, Self::VerifiedProduction { .. })
    }

    #[must_use]
    pub const fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked { .. })
    }
}
