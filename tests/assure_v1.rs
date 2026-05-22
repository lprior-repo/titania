use std::error::Error;

use xtask::assure::evidence::{EvidenceSignature, EvidenceStatus};
use xtask::assure::model::{
    ClaimCeilingId, ContractClauseId, DecisionPath, Digest, EvidenceId, ExpectedOutcome, Expr,
    ObligationId, PathId,
};
use xtask::assure::oracle::{OracleCheckFailure, OracleProvenance, check_oracles};
use xtask::assure::path::{PathCheckFailure, PathCheckInput, check_paths};
use xtask::assure::tenant_access;
use xtask::assure::{EvidenceRecord, TcbLedgerEntry};

#[test]
fn tenant_access_fixture_is_total_disjoint_and_oracle_mapped() -> Result<(), Box<dyn Error>> {
    let report = tenant_access::path_check_report()?;

    require(report.is_pass(), "TenantAccess path check must pass")?;
    require(
        report.valuations_checked == 9,
        "TenantAccess must enumerate 9 valuations",
    )?;
    require_path_hits(&report, "PATH_MISSING_TENANT_CLAIM", 3)?;
    require_path_hits(&report, "PATH_TENANT_MISMATCH", 3)?;
    require_path_hits(&report, "PATH_MEMBERSHIP_LOOKUP_FAILED", 1)?;
    require_path_hits(&report, "PATH_NO_MEMBERSHIP", 1)?;
    require_path_hits(&report, "PATH_GRANTED", 1)
}

#[test]
fn overlapping_decision_paths_fail_closed() -> Result<(), Box<dyn Error>> {
    let mut input = tenant_access::path_check_input()?;
    input.paths.push(DecisionPath {
        id: PathId::new("PATH_DUPLICATE_GRANTED")?,
        guard: Expr::And(vec![
            Expr::Eq {
                var: xtask::assure::VarId::new("tenant_claim_rel")?,
                value: xtask::assure::EnumVariantId::new("MatchesRequested")?,
            },
            Expr::Eq {
                var: xtask::assure::VarId::new("membership")?,
                value: xtask::assure::EnumVariantId::new("Exists")?,
            },
        ]),
        outcome: ExpectedOutcome::Ok {
            value: "AccessGranted".to_string(),
        },
        maps_to: vec![ContractClauseId::new("REQ_TENANT_ACCESS_GRANTED")?],
    });

    let report = check_paths(&input)?;

    require(
        report
            .failures
            .iter()
            .any(|failure| matches!(failure, PathCheckFailure::OverlappingValuation { .. })),
        "duplicate grant path must be reported as overlap",
    )
}

#[test]
fn uncovered_decision_valuation_fails_closed() -> Result<(), Box<dyn Error>> {
    let input = tenant_access::path_check_input()?;
    let without_no_membership = input
        .paths
        .clone()
        .into_iter()
        .filter(|path| path.id.as_str() != "PATH_NO_MEMBERSHIP")
        .collect();
    let mutated = PathCheckInput {
        paths: without_no_membership,
        ..input
    };

    let report = check_paths(&mutated)?;

    require(
        report
            .failures
            .iter()
            .any(|failure| matches!(failure, PathCheckFailure::UncoveredValuation { .. })),
        "missing no-membership path must leave one valuation uncovered",
    )
}

#[test]
fn unreachable_decision_path_fails_closed() -> Result<(), Box<dyn Error>> {
    let mut input = tenant_access::path_check_input()?;
    input.paths.push(DecisionPath {
        id: PathId::new("PATH_UNREACHABLE")?,
        guard: Expr::And(vec![
            Expr::Eq {
                var: xtask::assure::VarId::new("tenant_claim_rel")?,
                value: xtask::assure::EnumVariantId::new("Missing")?,
            },
            Expr::Neq {
                var: xtask::assure::VarId::new("tenant_claim_rel")?,
                value: xtask::assure::EnumVariantId::new("Missing")?,
            },
        ]),
        outcome: ExpectedOutcome::Err {
            error: "Impossible".to_string(),
        },
        maps_to: vec![ContractClauseId::new("REQ_IMPOSSIBLE")?],
    });

    let report = check_paths(&input)?;

    require(
        report.failures.iter().any(|failure| {
            matches!(
                failure,
                PathCheckFailure::UnreachablePath { path } if path.as_str() == "PATH_UNREACHABLE"
            )
        }),
        "contradictory path must be unreachable",
    )
}

#[test]
fn generated_oracle_marked_human_theater_fails_provenance_check() -> Result<(), Box<dyn Error>> {
    let mut oracles = tenant_access::oracle_records()?;
    let mut forged = oracles
        .iter()
        .find(|oracle| oracle.id.as_str() == "EX_POS_GRANTED")
        .cloned()
        .ok_or("missing EX_POS_GRANTED fixture")?;
    forged.id = xtask::assure::model::OracleId::new("EX_FORGED_HUMAN_LABEL")?;
    forged.generated_by_assurec = true;
    forged.provenance = OracleProvenance::Generated {
        generator_digest: Digest::new("sha256:assurec")?,
    };
    oracles.push(forged);

    let report = check_oracles(&oracles);

    require(
        !report.is_pass(),
        "generated oracle cannot close landing provenance",
    )?;
    require(
        report.failures.iter().any(|failure| {
            matches!(
                failure,
                OracleCheckFailure::GeneratedOracle { oracle } if oracle == "EX_FORGED_HUMAN_LABEL"
            )
        }),
        "generated oracle must be reported",
    )?;
    require(
        report.failures.iter().any(|failure| matches!(
            failure,
            OracleCheckFailure::UntrustedProvenance { oracle } if oracle == "EX_FORGED_HUMAN_LABEL"
        )),
        "generated oracle provenance must be untrusted",
    )
}

#[test]
fn claim_ceiling_blocks_full_auth_claims() -> Result<(), Box<dyn Error>> {
    let ceiling = tenant_access::claim_ceiling()?;

    require(
        ceiling.blocks_claim(&ContractClauseId::new("REQ_AUTH_FULL_001")?),
        "full auth claim must be blocked",
    )?;
    require(
        ceiling.blocks_claim(&ContractClauseId::new("REQ_EXPIRED_TOKEN_REJECTED")?),
        "expired-token claim must be blocked",
    )?;
    require(
        ceiling.blocks_claim(&ContractClauseId::new("REQ_INVALID_SIGNATURE_REJECTED")?),
        "invalid-signature claim must be blocked",
    )?;
    require(
        !ceiling.blocks_claim(&ContractClauseId::new("REQ_TENANT_ACCESS_GRANTED")?),
        "TenantAccess decision claim must remain in scope",
    )
}

#[test]
fn jwt_verified_assumption_blocks_full_auth_without_upstream_evidence() -> Result<(), Box<dyn Error>>
{
    let assumption = tenant_access::jwt_verified_assumption()?;

    require(
        assumption.blocks_claim(&ContractClauseId::new("REQ_AUTH_FULL_001")?),
        "JwtVerified assumption must block full-auth claim",
    )?;
    require(
        assumption
            .evidence_dependency
            .as_ref()
            .is_some_and(|dependency| {
                dependency.required && dependency.bead.as_str() == "jwt-verified"
            }),
        "JwtVerified assumption must require upstream jwt-verified bead",
    )
}

#[test]
fn only_signed_ci_or_verified_attestation_evidence_can_land() -> Result<(), Box<dyn Error>> {
    require(
        !EvidenceStatus::LocalUntrusted.accepted_for_landing(),
        "local evidence must not land",
    )?;
    require(
        !EvidenceStatus::CiUnsigned.accepted_for_landing(),
        "unsigned CI evidence must not land",
    )?;
    require(
        EvidenceStatus::CiSigned.accepted_for_landing(),
        "signed CI evidence must land",
    )?;
    require(
        EvidenceStatus::VerifiedAttestation.accepted_for_landing(),
        "verified attestation must land",
    )?;

    let local = evidence_record(EvidenceStatus::LocalUntrusted, true)?;
    let ci_signed = evidence_record(EvidenceStatus::CiSigned, true)?;
    let unsigned = evidence_record(EvidenceStatus::CiSigned, false)?;

    require(!local.accepted_for_landing(), "local record must not land")?;
    require(
        ci_signed.accepted_for_landing(),
        "signed CI record must land",
    )?;
    require(
        !unsigned.accepted_for_landing(),
        "unverified signature must not land",
    )
}

#[test]
fn tcb_entries_need_owner_reason_expiry_and_tests() -> Result<(), Box<dyn Error>> {
    let complete = TcbLedgerEntry {
        id: xtask::assure::model::TcbId::new("TCB_001")?,
        component: "assure-oracle-eval".to_string(),
        reason: "Evaluates finite Path IR independently of Rust codegen".to_string(),
        owner: "assurance-platform".to_string(),
        introduced_in: "commit-sha".to_string(),
        expires_review: "2026-07-01".to_string(),
        required_tests: vec!["totality negative tests".to_string()],
    };
    let missing_tests = TcbLedgerEntry {
        required_tests: Vec::new(),
        ..complete.clone()
    };

    require(
        complete.has_landing_metadata(),
        "complete TCB entry must pass metadata check",
    )?;
    require(
        !missing_tests.has_landing_metadata(),
        "missing TCB tests must fail metadata check",
    )
}

fn evidence_record(
    status: EvidenceStatus,
    signature_verified: bool,
) -> Result<EvidenceRecord, Box<dyn Error>> {
    Ok(EvidenceRecord {
        id: EvidenceId::new("EV_001")?,
        obligation: ObligationId::new("PO_001")?,
        command: "moon run assurance:kani-vb-123".to_string(),
        runner: "ci".to_string(),
        runner_identity: "github-actions:repo:org/repo:workflow:assurance.yml".to_string(),
        spec_digest: Digest::new("sha256:spec")?,
        ir_digest: Digest::new("sha256:ir")?,
        oracle_bank_digest: Digest::new("sha256:oracle")?,
        source_digest: Digest::new("sha256:source")?,
        generator_digest: Digest::new("sha256:generator")?,
        tool_versions: Default::default(),
        cwd_digest: Digest::new("sha256:cwd")?,
        env_digest: Digest::new("sha256:env")?,
        exit_code: 0,
        stdout_digest: Digest::new("sha256:stdout")?,
        stderr_digest: Digest::new("sha256:stderr")?,
        signature: EvidenceSignature {
            kind: "sigstore-or-ci-attestation".to_string(),
            verified: signature_verified,
        },
        status,
        claim_ceiling_id: ClaimCeilingId::new("CC_TENANT_ACCESS_V1")?,
    })
}

fn require_path_hits(
    report: &xtask::assure::PathCheckReport,
    path: &str,
    expected: usize,
) -> Result<(), Box<dyn Error>> {
    let path_id = PathId::new(path)?;
    require(
        report
            .path_hits
            .get(&path_id)
            .is_some_and(|hits| *hits == expected),
        "unexpected path hit count",
    )
}

fn require(condition: bool, message: &'static str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
