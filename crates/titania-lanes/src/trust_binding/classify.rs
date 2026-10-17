//! Trust markers, block reasons, and binding verdicts.
//!
//! A [`TrustMarker`] names one assumed-trust construct in a verification
//! source. A [`BindingVerdict`] reports source linkage independently from
//! verification of executable production behavior: [`BindingVerdict::SourceLinked`]
//! is never production-body proof, and only
//! [`BindingVerdict::VerifiedProduction`] carries a bounded claim over the
//! real body.

/// One assumed-trust construct that may appear in a verification source.
///
/// Every marker is allowed only with an independent explicit [`Waiver`](crate::trust_binding::Waiver);
/// a waiver keeps the classification at source-linked, never verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustMarker {
    /// `assume_specification[...]` bridge over a production symbol.
    AssumeSpecification,
    /// A bare `assume(...)` inside proof or executable code.
    Assume,
    /// `external_body`: the body is supplied outside the verifier.
    ExternalBody,
    /// `admit()`: the obligation is explicitly admitted, not discharged.
    Admit,
    /// A `trusted` attribute: the item is trusted without checking.
    Trusted,
}

impl TrustMarker {
    /// All markers in audit order.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::AssumeSpecification, Self::Assume, Self::ExternalBody, Self::Admit, Self::Trusted]
    }

    /// Short stable name used in waiver matching and finding messages.
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

/// Why an obligation cannot be reported as production behavior verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    /// The proof file is a smoke fixture; fixtures never satisfy obligations.
    SmokeOnly,
    /// A trust marker lacks an independent explicit waiver.
    UnapprovedTrust,
    /// No `#[path]` binding and no production-symbol reference.
    MissingBinding,
    /// Recorded evidence names a stale production revision.
    MirrorDrift,
    /// Proof items never reference the bound production symbol.
    VacuousProof,
    /// Evidence exists but the verifier run itself failed.
    MissingEvidence,
}

impl BlockReason {
    /// Stable lane rule id emitted for this block reason.
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

/// Audit outcome for one proof obligation.
///
/// Source linkage ([`BindingVerdict::SourceLinked`]) and production behavior
/// verification ([`BindingVerdict::VerifiedProduction`]) are distinct
/// variants by construction: no linkage value can be read as body proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingVerdict {
    /// A passing verifier run over the real body at the current revision.
    /// Names the implementation binding and the trusted base (tool).
    VerifiedProduction {
        /// Workspace-relative proof file that carries the binding.
        binding: String,
        /// Trusted base: verifier tool and version, e.g. `verus 0.0.1-test`.
        trusted_base: String,
    },
    /// Bound (or waived) linkage with no body proof. Allowed state, but
    /// never production behavior verification.
    SourceLinked {
        /// Markers covered by explicit waivers for this obligation.
        waived_markers: Vec<TrustMarker>,
    },
    /// The obligation must fail closed for the named reason.
    Blocked {
        /// Machine-readable reason; rendered into the lane finding.
        reason: BlockReason,
    },
}

impl BindingVerdict {
    /// `true` only for [`BindingVerdict::VerifiedProduction`].
    #[must_use]
    pub const fn is_verified_production(&self) -> bool {
        matches!(self, Self::VerifiedProduction { .. })
    }

    /// `true` for any blocked obligation.
    #[must_use]
    pub const fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked { .. })
    }
}
