use titania_lanes::{
    LaneReport,
    trust_binding::{
        AuditInputs, AuditResult, BindingVerdict, BlockReason, Obligation, ScannedFile,
        TrustMarker, VerifierEvidence, Waiver, audit_all, scan_proof_content,
    },
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn receipt_obligation(proof_file: &str) -> Obligation {
    Obligation::new(
        "receipt-schema-version",
        "crates/titania-core/src/receipt/schema.rs",
        "is_supported_receipt_schema_version",
        proof_file,
    )
}

const RECEIPT_MIRROR: &str = r#"use vstd::prelude::*;

#[path = "../../crates/titania-core/src/receipt/schema.rs"]
mod production;

verus! {

pub assume_specification[production::is_supported_receipt_schema_version](schema_version: u32) -> (supported: bool)
    ensures
        supported == (schema_version == 2),
;

fn schema_accepts_current() -> (supported: bool)
    ensures
        supported == true,
{
    production::is_supported_receipt_schema_version(2)
}

proof fn schema_version_literal_is_stable()
    ensures
        2 == 2,
{
    assert(2 == 2);
}

}

fn main() {}
"#;

const SMOKE_FIXTURE: &str = r#"// titania-verus-binding: fixture-smoke
use vstd::prelude::*;

verus! {

proof fn formal_setup_smoke()
    ensures
        1 + 1 == 2,
{
    assert(1 + 1 == 2);
}

}

fn main() {}
"#;

const TAUTOLOGY_ONLY: &str = r#"use vstd::prelude::*;

#[path = "../../crates/titania-core/src/receipt/schema.rs"]
mod production;

verus! {

proof fn literal_is_stable()
    ensures
        2 == 2,
{
    assert(2 == 2);
}

}

fn main() {}
"#;

const COPIED_MODEL: &str = r#"use vstd::prelude::*;

verus! {

pub fn is_supported_receipt_schema_version(schema_version: u32) -> (supported: bool)
    ensures
        supported == (schema_version == 2),
{
    schema_version == 2
}

}

fn main() {}
"#;

fn bound_assume_file() -> &'static str {
    r#"use vstd::prelude::*;

#[path = "../../crates/titania-core/src/receipt/schema.rs"]
mod production;

verus! {

fn checked_version(schema_version: u32) -> (supported: bool)
{
    assume(schema_version == 2);
    production::is_supported_receipt_schema_version(schema_version)
}

}

fn main() {}
"#
}

fn bound_external_body_file() -> &'static str {
    r#"use vstd::prelude::*;

#[path = "../../crates/titania-core/src/receipt/schema.rs"]
mod production;

verus! {

#[verifier::external_body]
fn checked_version(schema_version: u32) -> (supported: bool)
    ensures
        supported == (schema_version == 2),
{
    production::is_supported_receipt_schema_version(schema_version)
}

}

fn main() {}
"#
}

fn bound_admit_file() -> &'static str {
    r#"use vstd::prelude::*;

#[path = "../../crates/titania-core/src/receipt/schema.rs"]
mod production;

verus! {

proof fn version_bound(schema_version: u32)
    ensures
        production::is_supported_receipt_schema_version(schema_version) == (schema_version == 2),
{
    admit();
}

}

fn main() {}
"#
}

fn bound_trusted_file() -> &'static str {
    r#"use vstd::prelude::*;

#[path = "../../crates/titania-core/src/receipt/schema.rs"]
mod production;

verus! {

#[trusted]
fn checked_version(schema_version: u32) -> (supported: bool)
{
    production::is_supported_receipt_schema_version(schema_version)
}

}

fn main() {}
"#
}

fn passing_evidence(revision: &str) -> VerifierEvidence {
    VerifierEvidence::new("receipt-schema-version", "verus", "0.0.1-test", revision, 0)
}

fn explicit_waiver(marker: TrustMarker) -> Waiver {
    Waiver::new("receipt-schema-version", marker, "proof-review-board", "ticket-42")
}

fn run_audit(
    obligation: &Obligation,
    path: &str,
    content: &str,
    evidences: &[VerifierEvidence],
    waivers: &[Waiver],
    current_revision: &str,
) -> Result<(Vec<AuditResult>, LaneReport), Box<dyn std::error::Error>> {
    let file: ScannedFile = scan_proof_content(path, content);
    let files: &[ScannedFile] = &[file];
    let obligations: &[Obligation] = &[obligation.clone()];
    let inputs: AuditInputs<'_> =
        AuditInputs { obligations, files, evidences, waivers, current_revision };
    let mut report: LaneReport = LaneReport::new();
    let results: Vec<AuditResult> = audit_all(&inputs, &mut report)?;
    Ok((results, report))
}

fn single_verdict(results: &[AuditResult]) -> &BindingVerdict {
    assert_eq!(results.len(), 1, "expected exactly one audit result");
    results[0].verdict()
}

#[test]
fn verified_body_with_matching_evidence_names_binding_and_base() -> TestResult {
    let obligation: Obligation = receipt_obligation("verification/verus/receipt_schema.rs");
    let evidences: &[VerifierEvidence] = &[passing_evidence("rev-abc123")];
    let clean_mirror: &str = r#"use vstd::prelude::*;

#[path = "../../crates/titania-core/src/receipt/schema.rs"]
mod production;

verus! {

fn schema_accepts_current() -> (supported: bool)
    ensures
        supported == true,
{
    production::is_supported_receipt_schema_version(2)
}

}

fn main() {}
"#;
    let (results, report) = run_audit(
        &obligation,
        "verification/verus/receipt_schema.rs",
        clean_mirror,
        evidences,
        &[],
        "rev-abc123",
    )?;
    match single_verdict(&results) {
        BindingVerdict::VerifiedProduction { binding, trusted_base } => {
            assert_eq!(binding, "verification/verus/receipt_schema.rs");
            assert!(trusted_base.contains("verus"), "trusted base names tool");
        }
        other => panic!("expected VerifiedProduction, got {other:?}"),
    }
    assert!(report.is_clean(), "unexpected findings: {}", report.render());
    Ok(())
}

#[test]
fn path_linked_assume_spec_is_source_linked_never_verified() -> TestResult {
    let obligation: Obligation = receipt_obligation("verification/verus/receipt_schema.rs");
    let evidences: &[VerifierEvidence] = &[passing_evidence("rev-abc123")];
    let (results, report) = run_audit(
        &obligation,
        "verification/verus/receipt_schema.rs",
        RECEIPT_MIRROR,
        evidences,
        &[],
        "rev-abc123",
    )?;
    match single_verdict(&results) {
        BindingVerdict::Blocked { reason } => {
            assert_eq!(*reason, BlockReason::UnapprovedTrust);
        }
        other => panic!("expected UnapprovedTrust block, got {other:?}"),
    }
    assert_eq!(report.finding_count(), 1);
    assert_eq!(report.findings()[0].rule().as_str(), "TRUST_UNAPPROVED_MARKER");
    let waivers: &[Waiver] = &[explicit_waiver(TrustMarker::AssumeSpecification)];
    let (results, report) = run_audit(
        &obligation,
        "verification/verus/receipt_schema.rs",
        RECEIPT_MIRROR,
        &[],
        waivers,
        "rev-abc123",
    )?;
    match single_verdict(&results) {
        BindingVerdict::SourceLinked { .. } => {}
        other => panic!("expected SourceLinked, got {other:?}"),
    }
    assert!(report.is_clean(), "unexpected findings: {}", report.render());
    Ok(())
}

#[test]
fn unbound_copied_model_is_missing_binding() -> TestResult {
    let obligation: Obligation = receipt_obligation("verification/verus/copied_model.rs");
    let (results, report) = run_audit(
        &obligation,
        "verification/verus/copied_model.rs",
        COPIED_MODEL,
        &[],
        &[],
        "rev-abc123",
    )?;
    match single_verdict(&results) {
        BindingVerdict::Blocked { reason } => {
            assert_eq!(*reason, BlockReason::MissingBinding);
        }
        other => panic!("expected MissingBinding block, got {other:?}"),
    }
    assert_eq!(report.finding_count(), 1);
    assert_eq!(report.findings()[0].rule().as_str(), "TRUST_MISSING_BINDING");
    Ok(())
}

#[test]
fn changed_body_with_stale_evidence_is_mirror_drift() -> TestResult {
    let obligation: Obligation = receipt_obligation("verification/verus/receipt_schema.rs");
    let clean_mirror: &str = r#"use vstd::prelude::*;

#[path = "../../crates/titania-core/src/receipt/schema.rs"]
mod production;

verus! {

fn schema_accepts_current() -> (supported: bool)
    ensures
        supported == true,
{
    production::is_supported_receipt_schema_version(2)
}

}

fn main() {}
"#;
    let evidences: &[VerifierEvidence] = &[passing_evidence("rev-old999")];
    let (results, report) = run_audit(
        &obligation,
        "verification/verus/receipt_schema.rs",
        clean_mirror,
        evidences,
        &[],
        "rev-abc123",
    )?;
    match single_verdict(&results) {
        BindingVerdict::Blocked { reason } => {
            assert_eq!(*reason, BlockReason::MirrorDrift);
        }
        other => panic!("expected MirrorDrift block, got {other:?}"),
    }
    assert_eq!(report.finding_count(), 1);
    assert_eq!(report.findings()[0].rule().as_str(), "TRUST_MIRROR_DRIFT");
    Ok(())
}

#[test]
fn each_trust_marker_without_waiver_blocks() -> TestResult {
    let cases: &[(&str, &str, TrustMarker, &str)] = &[
        ("assume", bound_assume_file(), TrustMarker::Assume, "assume"),
        ("external_body", bound_external_body_file(), TrustMarker::ExternalBody, "external"),
        ("admit", bound_admit_file(), TrustMarker::Admit, "admit"),
        ("trusted", bound_trusted_file(), TrustMarker::Trusted, "trusted"),
    ];
    for (name, content, marker, fragment) in cases {
        let path: String = ["verification/verus/", name, ".rs"].concat();
        let obligation: Obligation = receipt_obligation(&path);
        let (results, report) = run_audit(&obligation, &path, content, &[], &[], "rev-abc123")?;
        match single_verdict(&results) {
            BindingVerdict::Blocked { reason } => {
                assert_eq!(*reason, BlockReason::UnapprovedTrust, "case {name}");
            }
            other => panic!("case {name}: expected UnapprovedTrust, got {other:?}"),
        }
        assert_eq!(report.finding_count(), 1, "case {name}");
        assert_eq!(report.findings()[0].rule().as_str(), "TRUST_UNAPPROVED_MARKER", "case {name}");
        assert!(
            report.findings()[0].message().contains(fragment),
            "case {name}: message names the marker"
        );
        let waivers: &[Waiver] = &[explicit_waiver(*marker)];
        let (results, report) = run_audit(&obligation, &path, content, &[], waivers, "rev-abc123")?;
        match single_verdict(&results) {
            BindingVerdict::SourceLinked { .. } => {}
            other => panic!("case {name}: expected SourceLinked, got {other:?}"),
        }
        assert!(report.is_clean(), "case {name}: {}", report.render());
    }
    Ok(())
}

#[test]
fn tautology_only_bound_file_is_vacuous() -> TestResult {
    let obligation: Obligation = receipt_obligation("verification/verus/tautology.rs");
    let (results, report) = run_audit(
        &obligation,
        "verification/verus/tautology.rs",
        TAUTOLOGY_ONLY,
        &[],
        &[],
        "rev-abc123",
    )?;
    match single_verdict(&results) {
        BindingVerdict::Blocked { reason } => {
            assert_eq!(*reason, BlockReason::VacuousProof);
        }
        other => panic!("expected VacuousProof block, got {other:?}"),
    }
    assert_eq!(report.finding_count(), 1);
    assert_eq!(report.findings()[0].rule().as_str(), "TRUST_VACUOUS_PROOF");
    Ok(())
}

#[test]
fn smoke_fixture_never_satisfies_obligation() -> TestResult {
    let obligation: Obligation = receipt_obligation("verification/verus/formal_setup_smoke.rs");
    let evidences: &[VerifierEvidence] =
        &[VerifierEvidence::new("receipt-schema-version", "verus", "0.0.1-test", "rev-abc123", 0)];
    let (results, report) = run_audit(
        &obligation,
        "verification/verus/formal_setup_smoke.rs",
        SMOKE_FIXTURE,
        evidences,
        &[],
        "rev-abc123",
    )?;
    match single_verdict(&results) {
        BindingVerdict::Blocked { reason } => {
            assert_eq!(*reason, BlockReason::SmokeOnly);
        }
        other => panic!("expected SmokeOnly block, got {other:?}"),
    }
    assert_eq!(report.finding_count(), 1);
    assert_eq!(report.findings()[0].rule().as_str(), "TRUST_SMOKE_COVERAGE");
    Ok(())
}

#[test]
fn failing_evidence_is_missing_evidence() -> TestResult {
    let obligation: Obligation = receipt_obligation("verification/verus/receipt_schema.rs");
    let clean_mirror: &str = r#"use vstd::prelude::*;

#[path = "../../crates/titania-core/src/receipt/schema.rs"]
mod production;

verus! {

fn schema_accepts_current() -> (supported: bool)
    ensures
        supported == true,
{
    production::is_supported_receipt_schema_version(2)
}

}

fn main() {}
"#;
    let evidences: &[VerifierEvidence] =
        &[VerifierEvidence::new("receipt-schema-version", "verus", "0.0.1-test", "rev-abc123", 1)];
    let (results, report) = run_audit(
        &obligation,
        "verification/verus/receipt_schema.rs",
        clean_mirror,
        evidences,
        &[],
        "rev-abc123",
    )?;
    match single_verdict(&results) {
        BindingVerdict::Blocked { reason } => {
            assert_eq!(*reason, BlockReason::MissingEvidence);
        }
        other => panic!("expected MissingEvidence block, got {other:?}"),
    }
    assert_eq!(report.finding_count(), 1);
    assert_eq!(report.findings()[0].rule().as_str(), "TRUST_MISSING_EVIDENCE");
    Ok(())
}

#[test]
fn non_explicit_waiver_covers_nothing() -> TestResult {
    let obligation: Obligation = receipt_obligation("verification/verus/assume_case.rs");
    let hollow: &[Waiver] = &[Waiver::new("receipt-schema-version", TrustMarker::Assume, "", "")];
    let (results, report) = run_audit(
        &obligation,
        "verification/verus/assume_case.rs",
        bound_assume_file(),
        &[],
        hollow,
        "rev-abc123",
    )?;
    match single_verdict(&results) {
        BindingVerdict::Blocked { reason } => {
            assert_eq!(*reason, BlockReason::UnapprovedTrust);
        }
        other => panic!("expected UnapprovedTrust block, got {other:?}"),
    }
    assert_eq!(report.finding_count(), 1);
    Ok(())
}

#[test]
fn scanner_names_path_target_and_first_marker_lines() -> TestResult {
    let file: ScannedFile =
        scan_proof_content("verification/verus/receipt_schema.rs", RECEIPT_MIRROR);
    assert!(file.signals().has_path_attr());
    assert_eq!(
        file.signals().path_target(),
        Some("../../crates/titania-core/src/receipt/schema.rs")
    );
    assert!(!file.signals().marker_lines(TrustMarker::AssumeSpecification).is_empty());
    assert!(file.signals().production_ref_count() > 0);
    assert!(!file.signals().is_smoke());
    let smoke: ScannedFile =
        scan_proof_content("verification/verus/formal_setup_smoke.rs", SMOKE_FIXTURE);
    assert!(smoke.signals().is_smoke());
    assert!(!smoke.signals().has_path_attr());
    Ok(())
}
