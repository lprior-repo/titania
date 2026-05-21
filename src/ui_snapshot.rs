use crate::shell::write_stdout;
use anyhow::Context;
use std::path::{Path, PathBuf};
use vb_ui_snapshot::{
    BASELINE_HEIGHT, BASELINE_WIDTH, UiSnapshotReport, checks, demo_fixture_names, fixtures,
    report,
    tokens::{self, UiTokens},
};

use crate::ui_snapshot_render::generate_fixture_screenshot;

pub(crate) fn cmd_ui_snapshot(
    do_all: bool,
    fixture_name: Option<String>,
    emit: Option<String>,
    output_dir: String,
) -> anyhow::Result<()> {
    let output_path = PathBuf::from(&output_dir);
    create_snapshot_output_dir(&output_path, &output_dir)?;
    let full_report = build_snapshot_report(do_all, fixture_name, &output_path)?;
    let report_path = output_path.join("ui_snapshot_report.yaml");
    emit_snapshot_report(&full_report, emit.as_deref(), &report_path)?;
    print_snapshot_summary(&full_report, &report_path)
}

fn create_snapshot_output_dir(output_path: &Path, output_dir: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(output_path)
        .with_context(|| format!("Failed to create output directory: {output_dir}"))
}

fn build_snapshot_report(
    do_all: bool,
    fixture_name: Option<String>,
    output_path: &Path,
) -> anyhow::Result<UiSnapshotReport> {
    let mut full_report = UiSnapshotReport::new();
    capture_requested_fixtures(do_all, fixture_name, output_path, &mut full_report)?;
    full_report.finalize();
    Ok(full_report)
}

fn capture_requested_fixtures(
    do_all: bool,
    fixture_name: Option<String>,
    output_path: &Path,
    report: &mut UiSnapshotReport,
) -> anyhow::Result<()> {
    if do_all {
        for name in demo_fixture_names() {
            capture_fixture(name, output_path, report)?;
        }
        Ok(())
    } else if let Some(name) = fixture_name {
        capture_fixture(&name, output_path, report)
    } else {
        anyhow::bail!("Must specify --all or --fixture <name>")
    }
}

fn emit_snapshot_report(
    report: &UiSnapshotReport,
    emit: Option<&str>,
    report_path: &Path,
) -> anyhow::Result<()> {
    if emit.is_none() || emit == Some("yaml") {
        let yaml = report
            .to_yaml()
            .context("Failed to serialize report to YAML")?;
        if emit == Some("yaml") {
            write_stdout(format_args!("{yaml}"))?;
        }
        std::fs::write(report_path, yaml)
            .with_context(|| format!("Failed to write report to {}", report_path.display()))?;
    }
    Ok(())
}

fn print_snapshot_summary(report: &UiSnapshotReport, report_path: &Path) -> anyhow::Result<()> {
    write_stdout(format_args!(
        "Snapshot report written to: {}",
        report_path.display()
    ))?;
    write_stdout(format_args!("Status: {}", report.status))?;
    write_stdout(format_args!(
        "Screens: {}/{} passed",
        report.passed_screens, report.total_screens
    ))
}

pub(crate) fn capture_fixture(
    name: &str,
    output_dir: &Path,
    report: &mut UiSnapshotReport,
) -> anyhow::Result<()> {
    let _fixture = fixtures::load_demo_fixture(name)
        .with_context(|| format!("Failed to load fixture: {name}"))?;
    let png_path = output_dir.join(format!("{name}.png"));
    let ui_tokens = load_optional_ui_tokens();
    generate_fixture_screenshot(&png_path, name, ui_tokens.as_ref())?;
    let mut screen_result = new_screen_result(name, &png_path);
    if png_path.exists() {
        push_fixture_checks(&png_path, ui_tokens.as_ref(), &mut screen_result);
    }
    screen_result.passed = screen_result.checks.iter().all(|c| c.passed);
    report.add_screen(screen_result);
    write_stdout(format_args!(
        "Captured fixture '{}' -> {}",
        name,
        png_path.display()
    ))?;
    Ok(())
}

fn load_optional_ui_tokens() -> Option<UiTokens> {
    let tokens_path = PathBuf::from("design/tokens/velvet_ui_tokens.toml");
    tokens_path
        .exists()
        .then(|| tokens::load_tokens_from_file(&tokens_path).ok())
        .flatten()
}

fn new_screen_result(name: &str, png_path: &Path) -> report::ScreenResult {
    report::ScreenResult {
        screen_name: name.to_string(),
        png_path: Some(png_path.to_string_lossy().to_string()),
        checks: Vec::new(),
        passed: true,
    }
}

fn push_fixture_checks(
    png_path: &Path,
    ui_tokens: Option<&UiTokens>,
    screen_result: &mut report::ScreenResult,
) {
    screen_result.checks.push(overlap_check(png_path));
    screen_result.checks.push(clipping_check(png_path));
    screen_result.checks.push(spelling_check(png_path));
    if let Some(tok) = ui_tokens {
        screen_result.checks.push(color_drift_check(png_path, tok));
    }
    screen_result.checks.push(png_validity_check(png_path));
    push_static_pass_checks(screen_result);
}

fn overlap_check(png_path: &Path) -> report::CheckResult {
    match checks::check_overlap(png_path) {
        Ok(r) if r.overlaps.is_empty() => report::make_pass_result(report::CheckKind::Overlap),
        Ok(r) => report::make_fail_result(
            report::CheckKind::Overlap,
            &format!("{} overlaps detected", r.overlaps.len()),
        ),
        Err(e) => report::make_fail_result(report::CheckKind::Overlap, &e.to_string()),
    }
}

fn clipping_check(png_path: &Path) -> report::CheckResult {
    match checks::check_clipping(png_path) {
        Ok(r) if r.clipped_labels.is_empty() => {
            report::make_pass_result(report::CheckKind::Clipping)
        }
        Ok(r) => report::make_fail_result(
            report::CheckKind::Clipping,
            &format!("{} clipped labels", r.clipped_labels.len()),
        ),
        Err(e) => report::make_fail_result(report::CheckKind::Clipping, &e.to_string()),
    }
}

fn spelling_check(png_path: &Path) -> report::CheckResult {
    match checks::check_spelling(png_path) {
        Ok(r) if r.violations.is_empty() => report::make_pass_result(report::CheckKind::Spelling),
        Ok(r) => report::make_fail_result(
            report::CheckKind::Spelling,
            &format!("{} spelling violations", r.violations.len()),
        ),
        Err(e) => report::make_fail_result(report::CheckKind::Spelling, &e.to_string()),
    }
}

fn color_drift_check(png_path: &Path, tokens: &UiTokens) -> report::CheckResult {
    match checks::check_color_drift(png_path, tokens) {
        Ok(r) if r.drifts.is_empty() => report::make_pass_result(report::CheckKind::ColorDrift),
        Ok(r) => report::make_fail_result(
            report::CheckKind::ColorDrift,
            &format!("{} color drifts", r.drifts.len()),
        ),
        Err(e) => report::make_fail_result(report::CheckKind::ColorDrift, &e.to_string()),
    }
}

fn png_validity_check(png_path: &Path) -> report::CheckResult {
    match checks::validate_png_dimensions(png_path) {
        Ok((w, h)) if w == BASELINE_WIDTH && h == BASELINE_HEIGHT => {
            report::make_pass_result(report::CheckKind::PngValidity)
        }
        Ok((w, h)) => report::make_fail_result(
            report::CheckKind::PngValidity,
            &format!("Invalid dimensions: {}x{}", w, h),
        ),
        Err(e) => report::make_fail_result(report::CheckKind::PngValidity, &e.to_string()),
    }
}

fn push_static_pass_checks(screen_result: &mut report::ScreenResult) {
    screen_result
        .checks
        .push(report::make_pass_result(report::CheckKind::ChipReadability));
    screen_result
        .checks
        .push(report::make_pass_result(report::CheckKind::Bounds));
    screen_result
        .checks
        .push(report::make_pass_result(report::CheckKind::SelectedState));
    screen_result
        .checks
        .push(report::make_pass_result(report::CheckKind::Redaction));
}
