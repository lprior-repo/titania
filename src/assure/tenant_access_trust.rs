#![forbid(unsafe_code)]

use crate::assure::evidence::{AssumptionLedgerEntry, ClaimCeiling, EvidenceDependency};
use crate::assure::model::{AssumptionId, AssureResult, BeadId, ClaimCeilingId, ContractClauseId};

pub fn claim_ceiling() -> AssureResult<ClaimCeiling> {
    Ok(ClaimCeiling {
        id: ClaimCeilingId::new("CC_TENANT_ACCESS_V1")?,
        proves: vec![
            "Authorization decision correctness given JwtVerified and declared repo fact"
                .to_string(),
        ],
        not_proven: [
            "JWT signature verification",
            "JWT expiry verification",
            "JWT parser correctness",
            "database truthfulness",
            "revocation",
            "temporal session behavior",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        blocks_claims: vec![
            ContractClauseId::new("REQ_AUTH_FULL_001")?,
            ContractClauseId::new("REQ_EXPIRED_TOKEN_REJECTED")?,
            ContractClauseId::new("REQ_INVALID_SIGNATURE_REJECTED")?,
        ],
    })
}

pub fn jwt_verified_assumption() -> AssureResult<AssumptionLedgerEntry> {
    Ok(AssumptionLedgerEntry {
        id: AssumptionId::new("ASM_JWT_001")?,
        claim: "JwtVerified is only constructible by the upstream JWT verification bead after signature, expiry, issuer, and audience checks".to_string(),
        required_for: vec![ContractClauseId::new("REQ_AUTH_FULL_001")?],
        evidence_dependency: Some(EvidenceDependency {
            bead: BeadId::new("jwt-verified")?,
            required: true,
        }),
        owner: "auth-platform".to_string(),
        expires: "2026-07-01".to_string(),
        blocks_landing_if_unclosed: true,
    })
}
