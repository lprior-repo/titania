#![forbid(unsafe_code)]

use crate::assure::model::{
    AssureResult, BeadId, ContractClauseId, DecisionPath, Digest, EnumVariantId, ExpectedOutcome,
    Expr, FactDomain, PathId, TypeId, VarId,
};
use crate::assure::oracle::{OracleProvenance, OracleRecord};
use crate::assure::path::{PathCheckInput, PathCheckReport, Valuation, check_paths};

pub use crate::assure::tenant_access_trust::{claim_ceiling, jwt_verified_assumption};

pub const PILOT_BEAD: &str = "vb-tenant-access-v1";

pub fn path_check_report() -> AssureResult<PathCheckReport> {
    check_paths(&path_check_input()?)
}

pub fn path_check_input() -> AssureResult<PathCheckInput> {
    Ok(PathCheckInput {
        domains: domains()?,
        paths: decision_paths()?,
        required_error_outcomes: required_error_outcomes(),
        oracles: oracle_records()?,
    })
}

pub fn domains() -> AssureResult<Vec<FactDomain>> {
    Ok(vec![tenant_claim_domain()?, membership_domain()?])
}

pub fn decision_paths() -> AssureResult<Vec<DecisionPath>> {
    Ok(vec![
        path_missing_tenant_claim()?,
        path_tenant_mismatch()?,
        path_membership_lookup_failed()?,
        path_no_membership()?,
        path_granted()?,
    ])
}

pub fn required_error_outcomes() -> Vec<String> {
    [
        "MissingTenantClaim",
        "TenantMismatch",
        "MembershipLookupFailed",
        "NoMembership",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

pub fn oracle_records() -> AssureResult<Vec<OracleRecord>> {
    Ok(vec![
        oracle(
            "EX_NEG_MISSING_CLAIM",
            "PATH_MISSING_TENANT_CLAIM",
            tenant_claim("Missing")?,
            membership("Exists")?,
            ExpectedOutcome::Err {
                error: "MissingTenantClaim".to_string(),
            },
        )?,
        oracle(
            "EX_NEG_TENANT_MISMATCH",
            "PATH_TENANT_MISMATCH",
            tenant_claim("DiffersFromRequested")?,
            membership("Exists")?,
            ExpectedOutcome::Err {
                error: "TenantMismatch".to_string(),
            },
        )?,
        oracle(
            "EX_NEG_LOOKUP_FAILED",
            "PATH_MEMBERSHIP_LOOKUP_FAILED",
            tenant_claim("MatchesRequested")?,
            membership("LookupFailed")?,
            ExpectedOutcome::Err {
                error: "MembershipLookupFailed".to_string(),
            },
        )?,
        oracle(
            "EX_NEG_NO_MEMBERSHIP",
            "PATH_NO_MEMBERSHIP",
            tenant_claim("MatchesRequested")?,
            membership("NotExists")?,
            ExpectedOutcome::Err {
                error: "NoMembership".to_string(),
            },
        )?,
        oracle(
            "EX_POS_GRANTED",
            "PATH_GRANTED",
            tenant_claim("MatchesRequested")?,
            membership("Exists")?,
            ExpectedOutcome::Ok {
                value: "AccessGranted".to_string(),
            },
        )?,
    ])
}

fn tenant_claim_domain() -> AssureResult<FactDomain> {
    FactDomain::enum_domain(
        var_tenant_claim_rel()?,
        TypeId::new("TenantClaimRel")?,
        vec![
            tenant_claim("Missing")?,
            tenant_claim("MatchesRequested")?,
            tenant_claim("DiffersFromRequested")?,
        ],
    )
}

fn membership_domain() -> AssureResult<FactDomain> {
    FactDomain::enum_domain(
        var_membership()?,
        TypeId::new("MembershipFact")?,
        vec![
            membership("Exists")?,
            membership("NotExists")?,
            membership("LookupFailed")?,
        ],
    )
}

fn path_missing_tenant_claim() -> AssureResult<DecisionPath> {
    decision_path(
        "PATH_MISSING_TENANT_CLAIM",
        Expr::Eq {
            var: var_tenant_claim_rel()?,
            value: tenant_claim("Missing")?,
        },
        ExpectedOutcome::Err {
            error: "MissingTenantClaim".to_string(),
        },
        "REQ_TENANT_ACCESS_MISSING_CLAIM",
    )
}

fn path_tenant_mismatch() -> AssureResult<DecisionPath> {
    decision_path(
        "PATH_TENANT_MISMATCH",
        Expr::Eq {
            var: var_tenant_claim_rel()?,
            value: tenant_claim("DiffersFromRequested")?,
        },
        ExpectedOutcome::Err {
            error: "TenantMismatch".to_string(),
        },
        "REQ_TENANT_ACCESS_MISMATCH",
    )
}

fn path_membership_lookup_failed() -> AssureResult<DecisionPath> {
    decision_path(
        "PATH_MEMBERSHIP_LOOKUP_FAILED",
        Expr::And(vec![
            Expr::Eq {
                var: var_tenant_claim_rel()?,
                value: tenant_claim("MatchesRequested")?,
            },
            Expr::Eq {
                var: var_membership()?,
                value: membership("LookupFailed")?,
            },
        ]),
        ExpectedOutcome::Err {
            error: "MembershipLookupFailed".to_string(),
        },
        "REQ_TENANT_ACCESS_LOOKUP_FAILED",
    )
}

fn path_no_membership() -> AssureResult<DecisionPath> {
    decision_path(
        "PATH_NO_MEMBERSHIP",
        Expr::And(vec![
            Expr::Eq {
                var: var_tenant_claim_rel()?,
                value: tenant_claim("MatchesRequested")?,
            },
            Expr::Eq {
                var: var_membership()?,
                value: membership("NotExists")?,
            },
        ]),
        ExpectedOutcome::Err {
            error: "NoMembership".to_string(),
        },
        "REQ_TENANT_ACCESS_NO_MEMBERSHIP",
    )
}

fn path_granted() -> AssureResult<DecisionPath> {
    decision_path(
        "PATH_GRANTED",
        Expr::And(vec![
            Expr::Eq {
                var: var_tenant_claim_rel()?,
                value: tenant_claim("MatchesRequested")?,
            },
            Expr::Eq {
                var: var_membership()?,
                value: membership("Exists")?,
            },
        ]),
        ExpectedOutcome::Ok {
            value: "AccessGranted".to_string(),
        },
        "REQ_TENANT_ACCESS_GRANTED",
    )
}

fn decision_path(
    id: &str,
    guard: Expr,
    outcome: ExpectedOutcome,
    clause: &str,
) -> AssureResult<DecisionPath> {
    Ok(DecisionPath {
        id: PathId::new(id)?,
        guard,
        outcome,
        maps_to: vec![ContractClauseId::new(clause)?],
    })
}

fn oracle(
    id: &str,
    path: &str,
    tenant_claim_rel: EnumVariantId,
    membership: EnumVariantId,
    expected: ExpectedOutcome,
) -> AssureResult<OracleRecord> {
    Ok(OracleRecord {
        id: crate::assure::model::OracleId::new(id)?,
        bead: BeadId::new(PILOT_BEAD)?,
        maps_to: vec![ContractClauseId::new(path)?],
        path: PathId::new(path)?,
        input: valuation(tenant_claim_rel, membership)?,
        expected,
        provenance: OracleProvenance::VcsPreexisting {
            commit: "merge-base".to_string(),
            present_in_merge_base: true,
            signer: None,
            signature_verified: false,
        },
        generated_by_assurec: false,
        artifact_path: format!("contracts/oracles/{PILOT_BEAD}/{id}.json"),
        included_in_oracle_bank: true,
        consumed_by_oracle_replay: true,
        digest: Digest::new(format!("sha256:{id}"))?,
    })
}

fn valuation(
    tenant_claim_rel: EnumVariantId,
    membership: EnumVariantId,
) -> AssureResult<Valuation> {
    Ok([
        (var_tenant_claim_rel()?, tenant_claim_rel),
        (var_membership()?, membership),
    ]
    .into_iter()
    .collect())
}

fn var_tenant_claim_rel() -> AssureResult<VarId> {
    VarId::new("tenant_claim_rel")
}

fn var_membership() -> AssureResult<VarId> {
    VarId::new("membership")
}

fn tenant_claim(value: &str) -> AssureResult<EnumVariantId> {
    EnumVariantId::new(value)
}

fn membership(value: &str) -> AssureResult<EnumVariantId> {
    EnumVariantId::new(value)
}
