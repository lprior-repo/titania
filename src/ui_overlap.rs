use anyhow::Context;
use std::path::Path;
use std::path::PathBuf;
use vb_ui_snapshot::{checks, demo_fixture_names};

use crate::shell::write_stdout;

pub(crate) fn cmd_ui_overlap_check(
    do_all: bool,
    screen_name: Option<String>,
    input_dir: &str,
) -> anyhow::Result<()> {
    let input_path = PathBuf::from(input_dir);
    let has_failure = overlap_check_has_failure(do_all, screen_name, &input_path)?;
    if has_failure {
        anyhow::bail!("UI overlap check failed");
    }
    Ok(())
}

fn overlap_check_has_failure(
    do_all: bool,
    screen_name: Option<String>,
    input_path: &Path,
) -> anyhow::Result<bool> {
    if do_all {
        any_demo_overlap_failure(input_path)
    } else if let Some(name) = screen_name {
        check_overlap_for_screen(input_path, &name).map(|passed| !passed)
    } else {
        anyhow::bail!("Must specify --all or --screen <name>")
    }
}

fn any_demo_overlap_failure(input_path: &Path) -> anyhow::Result<bool> {
    let mut has_failure = false;
    for name in demo_fixture_names() {
        if !check_overlap_for_screen(input_path, name)? {
            has_failure = true;
        }
    }
    Ok(has_failure)
}

// ============================================================================
// Section 77 Command-Center Gate Profiles
// ============================================================================
pub(crate) fn check_overlap_for_screen(base_dir: &Path, name: &str) -> anyhow::Result<bool> {
    let png_path = base_dir.join(format!("{name}.png"));
    if !png_path.exists() {
        write_stdout(format_args!("FAIL: {} does not exist", png_path.display()))?;
        return Ok(false);
    }
    let result = checks::check_overlap(&png_path)
        .with_context(|| format!("Overlap check failed for: {name}"))?;
    print_overlap_result(name, &result)
}

fn print_overlap_result(name: &str, result: &checks::OverlapResult) -> anyhow::Result<bool> {
    if result.overlaps.is_empty() {
        return write_stdout(format_args!("PASS: {name} — no overlaps detected")).map(|()| true);
    }
    write_stdout(format_args!(
        "FAIL: {name} — {} overlaps detected:",
        result.overlaps.len()
    ))?;
    for ov in &result.overlaps {
        write_stdout(format_args!(
            "  {} overlaps {} ({}px)",
            ov.panel_a, ov.panel_b, ov.overlap_area_px
        ))?;
    }
    Ok(false)
}
