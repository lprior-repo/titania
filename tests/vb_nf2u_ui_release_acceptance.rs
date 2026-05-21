#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;
use xtask::evidence::{
    CoreParityClaim, DiagnosticCode, FixtureBackedState, FixtureGate, FixtureStatus,
    ParsedAiReleaseDocument, ParsedNegativeFixtureDocument, ParsedOverlapExpectedFailure,
    ParsedOverlapFixtureEvidence, ParsedSecretExpectedFailure, ParsedSecretFixtureEvidence,
    ParsedSnapshotDocument, XtaskCommandDiagnostic, parse_ai_release_document,
    parse_negative_fixture_document, parse_snapshot_document,
};

const BEAD_ID: &str = "vb-nf2u";
const AI_RELEASE_YAML: &str = ".evidence/vb-nf2u/ai-release.yaml";
const UI_SNAPSHOT_REPORT_YAML: &str = ".evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml";
const NEGATIVE_FIXTURES_TXT: &str = ".evidence/vb-nf2u/negative-fixtures.txt";
const EVIDENCE_DIR: &str = ".evidence/vb-nf2u";
const NEGATIVE_FIXTURE_DIR: &str = "target/vb-nf2u-negative-fixtures";
const TOOLING_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const TOOLING_MANIFEST: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");

const RAW_SENTINEL: &str = "vb_nf2u_secret_sentinel";
const RAW_API_KEY: &str = "sk_test_vb_nf2u_raw_secret";
const RAW_TOKEN: &str = "Bearer vb_nf2u_token";
const RAW_PASSWORD: &str = "password=hunter2";
const RAW_IDEMPOTENCY_KEY: &str = "Idempotency-Key: idem_vb_nf2u_secret";
const RAW_TAINT: &str = "tainted_fixture_value_vb_nf2u";

#[test]
fn all_eight_screens_pass_reachability_and_overlap_gates() {
    // Given
    let workspace = isolated_workspace("all_eight_screens");
    let output = must(run_ai_release_for_vb_nf2u(&workspace), "run ai-release");

    // When
    assert_command_succeeded(&output);
    let ai_release = read_required_artifact(&workspace, AI_RELEASE_YAML);
    let snapshot_report = read_required_artifact(&workspace, UI_SNAPSHOT_REPORT_YAML);

    // Then
    assert_ui_subgates_are_exact(&ai_release);
    assert_snapshot_inventory_is_exact(&snapshot_report);
    assert_screen_has_required_checks(&snapshot_report, "execution_overview");
    assert_screen_has_required_checks(&snapshot_report, "workflow_graph_authoring");
    assert_screen_has_required_checks(&snapshot_report, "execution_details");
    assert_screen_has_required_checks(&snapshot_report, "verification_certificate");
    assert_screen_has_required_checks(&snapshot_report, "replay_theater");
    assert_screen_has_required_checks(&snapshot_report, "incident_failure");
    assert_screen_has_required_checks(&snapshot_report, "action_registry");
    assert_screen_has_required_checks(&snapshot_report, "storage_doctor_ai_context");
    assert_fixture_evidence_disclaims_core_parity(&ai_release, &snapshot_report);
}

#[test]
fn secret_values_are_redacted_in_every_screen() {
    // Given
    let workspace = isolated_workspace("redacted_every_screen");
    let output = must(run_ai_release_for_vb_nf2u(&workspace), "run ai-release");

    // When
    assert_command_succeeded(&output);
    let ai_release = read_required_artifact(&workspace, AI_RELEASE_YAML);
    let snapshot_report = read_required_artifact(&workspace, UI_SNAPSHOT_REPORT_YAML);
    let negative_evidence = read_required_artifact(&workspace, NEGATIVE_FIXTURES_TXT);
    let evidence_text = format!("{ai_release}\n{snapshot_report}\n{negative_evidence}");

    // Then
    assert_no_raw_denied_values_are_emitted(&evidence_text);
    assert_redaction_coverage_for_screen(&ai_release, "execution_overview");
    assert_redaction_coverage_for_screen(&ai_release, "workflow_graph_authoring");
    assert_redaction_coverage_for_screen(&ai_release, "execution_details");
    assert_redaction_coverage_for_screen(&ai_release, "verification_certificate");
    assert_redaction_coverage_for_screen(&ai_release, "replay_theater");
    assert_redaction_coverage_for_screen(&ai_release, "incident_failure");
    assert_redaction_coverage_for_screen(&ai_release, "action_registry");
    assert_redaction_coverage_for_screen(&ai_release, "storage_doctor_ai_context");
}

#[test]
fn intentional_overlap_fixture_fails_gate() {
    // Given
    let workspace = isolated_workspace("overlap_fails_gate");
    must(
        prepare_intentional_overlap_fixture(
            &workspace,
            "screen_id=execution_overview\nfirst_control_id=run_button\nsecond_control_id=stop_button\nrun_button={x:10,y:10,width:100,height:60}\nstop_button={x:80,y:40,width:50,height:50}\nexpected_gate=layout\nexpected_code=layout_violation\noverlap_area_px=600\nbounds={ x: 10, y: 10, width: 100, height: 60 }\nactual_status=failed\n",
        ),
        "prepare overlap fixture",
    );

    // When
    let output = must(run_ai_release_for_vb_nf2u(&workspace), "run ai-release");

    // Then
    assert_command_succeeded(&output);
    let negative_evidence = read_required_artifact(&workspace, NEGATIVE_FIXTURES_TXT);
    let entry = must(
        parse_negative_evidence(&negative_evidence),
        "parse negative evidence",
    )
    .overlap;
    assert_default_overlap_entry(&entry);
}

#[test]
fn intentional_secret_fixture_fails_redaction_gate() {
    // Given
    let workspace = isolated_workspace("secret_fails_gate");
    must(
        prepare_intentional_secret_fixture(
            &workspace,
            "fixture_id=intentional_secret_fixture\nraw_sentinel=vb_nf2u_secret_sentinel\napi_key=sk_test_vb_nf2u_raw_secret\ntoken=Bearer vb_nf2u_token\npassword=password=hunter2\nidempotency=Idempotency-Key: idem_vb_nf2u_secret\ntaint=tainted_fixture_value_vb_nf2u\nexpected_gate=redaction\nexpected_code=redaction_violation\nactual_status=failed\n",
        ),
        "prepare secret fixture",
    );

    // When
    let output = must(run_ai_release_for_vb_nf2u(&workspace), "run ai-release");

    // Then
    assert_command_succeeded(&output);
    let negative_evidence = read_required_artifact(&workspace, NEGATIVE_FIXTURES_TXT);
    let entry = must(
        parse_negative_evidence(&negative_evidence),
        "parse negative evidence",
    )
    .secret;
    let expected = must(require_secret_expected(&entry), "require secret expected");
    assert_eq!(expected.status, FixtureStatus::ExpectedFailed);
    assert_eq!(expected.diagnostic_code, DiagnosticCode::Redaction);
    assert_eq!(expected.secret_class.as_str(), "api_key");
    assert_eq!(expected.redacted_sample.as_str(), "[REDACTED:api_key]");
    assert_no_raw_denied_values_are_emitted(&negative_evidence);
}

#[test]
fn overlap_negative_fixture_is_consumed_by_command_boundary() {
    // Given
    let workspace = isolated_workspace("overlap_boundary");
    must(
        prepare_intentional_overlap_fixture(
            &workspace,
            "screen_id=execution_overview\nfirst_control_id=changed_run_button\nsecond_control_id=changed_stop_button\nchanged_run_button={x:1,y:1,width:10,height:10}\nchanged_stop_button={x:5,y:5,width:20,height:20}\nexpected_gate=layout\nexpected_code=layout_violation\noverlap_area_px=25\nbounds={ x: 1, y: 1, width: 10, height: 10 }\nactual_status=failed\nfixture_nonce=overlap_fixture_must_be_read\n",
        ),
        "prepare overlap fixture",
    );

    // When
    let output = must(run_ai_release_for_vb_nf2u(&workspace), "run ai-release");

    // Then
    assert_command_succeeded(&output);
    let negative_evidence = read_required_artifact(&workspace, NEGATIVE_FIXTURES_TXT);
    let entry = must(
        parse_negative_evidence(&negative_evidence),
        "parse negative evidence",
    )
    .overlap;
    assert_changed_overlap_entry(&entry);
}

#[test]
fn secret_negative_fixture_is_consumed_by_command_boundary() {
    // Given
    let workspace = isolated_workspace("secret_boundary");
    must(
        prepare_intentional_secret_fixture(
            &workspace,
            "fixture_id=intentional_secret_fixture\nraw_sentinel=vb_nf2u_secret_sentinel\napi_key=sk_test_vb_nf2u_raw_secret_CHANGED\nexpected_gate=redaction\nexpected_code=redaction_violation\nactual_status=failed\nfixture_nonce=secret_fixture_must_be_read\n",
        ),
        "prepare secret fixture",
    );

    // When
    let output = must(run_ai_release_for_vb_nf2u(&workspace), "run ai-release");

    // Then
    assert_command_succeeded(&output);
    let negative_evidence = read_required_artifact(&workspace, NEGATIVE_FIXTURES_TXT);
    let entry = must(
        parse_negative_evidence(&negative_evidence),
        "parse negative evidence",
    )
    .secret;
    let expected = must(require_secret_expected(&entry), "require secret expected");
    assert_eq!(
        expected.fixture_nonce.as_ref().map(|nonce| nonce.as_str()),
        Some("secret_fixture_must_be_read")
    );
    assert_no_raw_value(&negative_evidence, "sk_test_vb_nf2u_raw_secret_CHANGED");
}

#[test]
fn overlap_false_pass_fixture_is_rejected() {
    // Given
    let workspace = isolated_workspace("overlap_false_pass");
    must(
        prepare_intentional_overlap_fixture(
            &workspace,
            "fixture_id=intentional_overlap_fixture\nfirst_control_id=run_button\nsecond_control_id=stop_button\nexpected_gate=layout\nexpected_code=layout_violation\noverlap_area_px=600\nbounds={ x: 10, y: 10, width: 100, height: 60 }\nactual_status=passed\nfixture_nonce=overlap_false_pass_detector\n",
        ),
        "prepare overlap fixture",
    );

    // When
    let output = must(run_ai_release_for_vb_nf2u(&workspace), "run ai-release");

    // Then
    assert_false_pass_diagnostic(&output, "intentional_overlap_fixture", FixtureGate::Layout);
}

#[test]
fn secret_false_pass_fixture_is_rejected() {
    // Given
    let workspace = isolated_workspace("secret_false_pass");
    must(
        prepare_intentional_secret_fixture(
            &workspace,
            "fixture_id=intentional_secret_fixture\nexpected_gate=redaction\nexpected_code=redaction_violation\nactual_status=passed\nfixture_nonce=secret_false_pass_detector\n",
        ),
        "prepare secret fixture",
    );

    // When
    let output = must(run_ai_release_for_vb_nf2u(&workspace), "run ai-release");

    // Then
    assert_false_pass_diagnostic(
        &output,
        "intentional_secret_fixture",
        FixtureGate::Redaction,
    );
}

fn run_ai_release_for_vb_nf2u(workspace: &IsolatedWorkspace) -> Result<Output, Box<dyn Error>> {
    Command::new("cargo")
        .current_dir(workspace.root.path())
        .args([
            "run",
            "--locked",
            "--manifest-path",
            TOOLING_MANIFEST,
            "--",
            "ai-release",
            "--bead",
            BEAD_ID,
        ])
        .output()
        .map_err(Into::into)
}

fn assert_command_succeeded(output: &Output) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected `cargo xtask ai-release --bead vb-nf2u` to succeed and emit UI release evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_false_pass_diagnostic(
    output: &Output,
    expected_fixture: &str,
    expected_gate: FixtureGate,
) {
    assert_ne!(
        output.status.code(),
        Some(0),
        "expected `cargo xtask ai-release --bead vb-nf2u` to fail closed for false-pass negative fixture\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_exact_false_pass_diagnostic(&combined, expected_fixture, expected_gate);
}

fn read_required_artifact(workspace: &IsolatedWorkspace, path: &str) -> String {
    let content = must(
        fs::read_to_string(workspace.root.path().join(path)).map_err(Box::<dyn Error>::from),
        "read artifact",
    );
    assert_ne!(content, "", "required artifact was empty: {path}");
    content
}

fn assert_ui_subgates_are_exact(ai_release: &str) {
    assert_eq!(
        subgate_names(&must(parse_ai_release(ai_release), "parse ai-release")),
        canonical_subgates()
    );
}

fn assert_snapshot_inventory_is_exact(snapshot_report: &str) {
    let report = must(
        parse_snapshot_report(snapshot_report),
        "parse snapshot report",
    );
    assert_eq!(report.total_screens, 8);
    assert_eq!(report.passed_screens, 8);
    assert_eq!(report.failed_screens, 0);
    assert_eq!(snapshot_screen_names(&report), canonical_screens());
}

fn assert_screen_has_required_checks(snapshot_report: &str, screen: &str) {
    let report = must(
        parse_snapshot_report(snapshot_report),
        "parse snapshot report",
    );
    let checks = snapshot_checks_for(&report, screen);
    assert_eq!(checks, required_checks());
}

fn assert_redaction_coverage_for_screen(ai_release: &str, screen: &str) {
    let ai = must(parse_ai_release(ai_release), "parse ai-release");
    let classes = redaction_classes_for(&ai, screen);
    assert_eq!(classes, redaction_classes());
}

fn snapshot_screen_names(report: &SnapshotReport) -> Vec<String> {
    report
        .screens
        .iter()
        .map(|screen| screen.screen_name.as_str().to_string())
        .collect()
}

fn snapshot_checks_for(report: &SnapshotReport, screen: &str) -> Vec<String> {
    report
        .screens
        .iter()
        .find_map(|entry| {
            (entry.screen_name.as_str() == screen).then(|| {
                entry
                    .checks
                    .iter()
                    .map(|check| check.as_str().to_string())
                    .collect()
            })
        })
        .unwrap_or_default()
}

fn redaction_classes_for(doc: &AiReleaseDoc, screen: &str) -> Vec<String> {
    doc.redaction
        .iter()
        .find_map(|entry| {
            (entry.screen_id.as_str() == screen).then(|| {
                entry
                    .classes
                    .iter()
                    .map(|class| class.as_str().to_string())
                    .collect()
            })
        })
        .unwrap_or_default()
}

fn subgate_names(doc: &AiReleaseDoc) -> Vec<String> {
    doc.subgates
        .iter()
        .map(|subgate| subgate.as_str().to_string())
        .collect()
}

fn assert_no_raw_denied_values_are_emitted(evidence_text: &str) {
    assert_no_raw_value(evidence_text, RAW_SENTINEL);
    assert_no_raw_value(evidence_text, RAW_API_KEY);
    assert_no_raw_value(evidence_text, RAW_TOKEN);
    assert_no_raw_value(evidence_text, RAW_PASSWORD);
    assert_no_raw_value(evidence_text, RAW_IDEMPOTENCY_KEY);
    assert_no_raw_value(evidence_text, RAW_TAINT);
}

fn assert_no_raw_value(evidence_text: &str, raw: &str) {
    assert_eq!(
        raw_value_count(evidence_text, raw),
        0,
        "raw value leaked: {raw}"
    );
}

fn raw_value_count(evidence_text: &str, raw: &str) -> usize {
    evidence_text.match_indices(raw).count()
}

fn assert_exact_false_pass_diagnostic(
    text: &str,
    expected_fixture: &str,
    expected_gate: FixtureGate,
) {
    let diag = must(
        XtaskCommandDiagnostic::parse_output(text).map_err(Box::<dyn Error>::from),
        "parse false-pass diagnostic",
    );
    assert_eq!(diag.error_code.as_str(), "false_pass_fixture_violation");
    assert_eq!(diag.fixture_id.as_str(), expected_fixture);
    assert_eq!(diag.expected_gate, expected_gate);
    assert_eq!(diag.actual_status, FixtureStatus::Passed);
}

fn assert_fixture_evidence_disclaims_core_parity(ai_release: &str, snapshot_report: &str) {
    assert_eq!(
        must(parse_ai_release(ai_release), "parse ai-release").fixture_backed,
        FixtureBackedState::FixtureBacked
    );
    assert_eq!(
        must(parse_ai_release(ai_release), "parse ai-release").core_runtime_parity_claim,
        CoreParityClaim::Unsupported
    );
    assert_eq!(
        must(
            parse_snapshot_report(snapshot_report),
            "parse snapshot report"
        )
        .fixture_backed,
        FixtureBackedState::FixtureBacked
    );
    assert_eq!(
        must(
            parse_snapshot_report(snapshot_report),
            "parse snapshot report"
        )
        .core_runtime_parity_claim,
        CoreParityClaim::Unsupported
    );
}

type SnapshotReport = ParsedSnapshotDocument;
type AiReleaseDoc = ParsedAiReleaseDocument;
type NegativeEvidenceDoc = ParsedNegativeFixtureDocument;
type NegativeEntry = ParsedOverlapFixtureEvidence;

fn assert_default_overlap_entry(entry: &NegativeEntry) {
    let expected = must(require_overlap_expected(entry), "require overlap expected");
    assert_eq!(expected.status, FixtureStatus::ExpectedFailed);
    assert_eq!(expected.diagnostic_code, DiagnosticCode::Layout);
    assert_eq!(expected.screen_id.as_str(), "execution_overview");
    assert_eq!(expected.control_id.as_str(), "run_button");
    assert_eq!(expected.second_control_id.as_str(), "stop_button");
    assert_eq!(expected.predicate.as_str(), "Overlap");
    assert_eq!(expected.overlap_area_px.as_u32(), 600);
    assert_default_overlap_bounds(expected);
}

fn assert_default_overlap_bounds(entry: &ParsedOverlapExpectedFailure) {
    assert_eq!(
        entry.bounds.as_str(),
        "{ x: 10, y: 10, width: 100, height: 60 }"
    );
}

fn assert_changed_overlap_entry(entry: &NegativeEntry) {
    let expected = must(require_overlap_expected(entry), "require overlap expected");
    assert_eq!(
        expected.fixture_nonce.as_ref().map(|nonce| nonce.as_str()),
        Some("overlap_fixture_must_be_read")
    );
    assert_eq!(expected.overlap_area_px.as_u32(), 25);
    assert_eq!(expected.control_id.as_str(), "changed_run_button");
    assert_eq!(expected.second_control_id.as_str(), "changed_stop_button");
    assert_changed_overlap_bounds(expected);
}

fn assert_changed_overlap_bounds(entry: &ParsedOverlapExpectedFailure) {
    assert_eq!(
        entry.bounds.as_str(),
        "{ x: 1, y: 1, width: 10, height: 10 }"
    );
}

fn require_overlap_expected(
    entry: &ParsedOverlapFixtureEvidence,
) -> Result<&ParsedOverlapExpectedFailure, Box<dyn Error>> {
    match entry {
        ParsedOverlapFixtureEvidence::ExpectedFailed(value) => Ok(value),
        ParsedOverlapFixtureEvidence::Rejected(_) => Err("overlap fixture was rejected".into()),
    }
}

fn require_secret_expected(
    entry: &ParsedSecretFixtureEvidence,
) -> Result<&ParsedSecretExpectedFailure, Box<dyn Error>> {
    match entry {
        ParsedSecretFixtureEvidence::ExpectedFailed(value) => Ok(value),
        ParsedSecretFixtureEvidence::Rejected(_) => Err("secret fixture was rejected".into()),
    }
}

fn parse_snapshot_report(text: &str) -> Result<SnapshotReport, Box<dyn Error>> {
    parse_snapshot_document(text).map_err(Into::into)
}

fn parse_ai_release(text: &str) -> Result<AiReleaseDoc, Box<dyn Error>> {
    parse_ai_release_document(text).map_err(Into::into)
}

fn parse_negative_evidence(text: &str) -> Result<NegativeEvidenceDoc, Box<dyn Error>> {
    parse_negative_fixture_document(text).map_err(Into::into)
}

fn must<T>(result: Result<T, Box<dyn Error>>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => {
            assert_eq!(format!("{context}: {error}"), "");
            std::process::abort();
        }
    }
}

fn canonical_screens() -> Vec<String> {
    [
        "execution_overview",
        "workflow_graph_authoring",
        "execution_details",
        "verification_certificate",
        "replay_theater",
        "incident_failure",
        "action_registry",
        "storage_doctor_ai_context",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}

fn canonical_subgates() -> Vec<String> {
    [
        "ui_snapshot",
        "layout_readability",
        "redaction",
        "negative_fixture",
        "deterministic_capture",
        "evidence_shape",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}

fn required_checks() -> Vec<String> {
    [
        "Overlap",
        "Clipping",
        "Bounds",
        "ChipReadability",
        "SelectedState",
        "FixtureArtifactProvenance",
        "Redaction",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}

fn redaction_classes() -> Vec<String> {
    [
        "sentinel",
        "api_key",
        "token",
        "password",
        "idempotency_key",
        "tainted_fixture_value",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}

fn prepare_intentional_overlap_fixture(
    workspace: &IsolatedWorkspace,
    content: &str,
) -> Result<(), Box<dyn Error>> {
    let path = workspace
        .root
        .path()
        .join(NEGATIVE_FIXTURE_DIR)
        .join("intentional_overlap_fixture.txt");
    write_fixture(path, content)
}

fn prepare_intentional_secret_fixture(
    workspace: &IsolatedWorkspace,
    content: &str,
) -> Result<(), Box<dyn Error>> {
    let path = workspace
        .root
        .path()
        .join(NEGATIVE_FIXTURE_DIR)
        .join("intentional_secret_fixture.txt");
    write_fixture(path, content)
}

fn write_fixture(path: PathBuf, content: &str) -> Result<(), Box<dyn Error>> {
    let parent = path.parent().ok_or("fixture path must have parent")?;
    fs::create_dir_all(parent)?;
    fs::write(path, content)?;
    Ok(())
}

struct IsolatedWorkspace {
    root: TempDir,
}

fn isolated_workspace(test_name: &str) -> IsolatedWorkspace {
    let root = must(
        tempfile::Builder::new()
            .prefix(&format!("vb-nf2u-{test_name}-"))
            .tempdir()
            .map_err(Box::<dyn Error>::from),
        "create isolated workspace",
    );
    let workspace = IsolatedWorkspace { root };
    must(
        seed_snapshot_fixture_artifacts(&workspace),
        "seed snapshot fixtures",
    );
    workspace
}

fn seed_snapshot_fixture_artifacts(workspace: &IsolatedWorkspace) -> Result<(), Box<dyn Error>> {
    let snapshot_dir = workspace
        .root
        .path()
        .join(EVIDENCE_DIR)
        .join("ui_snapshots");
    fs::create_dir_all(&snapshot_dir)?;
    for screen in canonical_screens() {
        let fixture_name = format!("{screen}.fixture.txt");
        let source = Path::new(TOOLING_ROOT)
            .join("fixtures")
            .join("vb-nf2u-ui")
            .join(&fixture_name);
        let destination = snapshot_dir.join(fixture_name);
        fs::copy(source, destination)?;
    }
    Ok(())
}
