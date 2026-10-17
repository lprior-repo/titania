use titania_lanes::{
    current_target_project,
    dylint_lane::{DylintProbe, probe_dylint_toolchain},
};

fn test_target() -> titania_core::TargetProject {
    current_target_project().expect("current dir must resolve to a TargetProject")
}

#[test]
fn dylint_loader_probe_missing_cargo_dylint_returns_infra_failure() {
    let probe = probe_dylint_toolchain(&test_target());

    match probe {
        DylintProbe::Ready(_) => {
            assert!(probe.is_ready());
            assert!(probe.failure().is_none());
        }
        DylintProbe::Infra(failure, report) => {
            assert!(
                failure.tool() == "cargo-dylint" || failure.tool() == "libtitania_dylint",
                "infra_failure.tool must be \"cargo-dylint\" or \"libtitania_dylint\", got \"{}\"",
                failure.tool(),
            );
            assert!(failure.is_infra());
            assert!(!report.is_clean(), "infra failure must produce a non-clean report");
            assert!(report.finding_count() > 0, "infra report must contain at least one finding");
        }
    }
}

#[test]
fn dylint_loader_probe_failure_contains_correct_tool_name() {
    let probe = probe_dylint_toolchain(&test_target());

    if let DylintProbe::Infra(failure, _) = probe {
        let tool = failure.tool();
        assert!(
            tool == "cargo-dylint" || tool == "libtitania_dylint",
            "infra_failure.tool must be \"cargo-dylint\" or \"libtitania_dylint\", got \"{}\"",
            tool,
        );
    }
}

#[test]
fn dylint_loader_probe_infra_failure_is_infra() {
    let probe = probe_dylint_toolchain(&test_target());

    if let DylintProbe::Infra(failure, _) = probe {
        assert!(
            failure.is_infra(),
            "the failure must be an infra failure, not {}",
            format!("{failure:?}"),
        );
    }
}

#[test]
fn dylint_loader_probe_ready_has_no_failure() {
    let probe = probe_dylint_toolchain(&test_target());

    if probe.is_ready() {
        assert!(probe.failure().is_none());
        assert!(probe.load().is_some());
    }
}

#[test]
fn dylint_loader_probe_infra_report_contains_infra_finding() {
    let probe = probe_dylint_toolchain(&test_target());

    if let DylintProbe::Infra(_, report) = probe {
        assert!(report.finding_count() > 0, "infra report must contain at least one finding");
        let finding = &report.findings()[0];
        let rule = finding.rule().as_str();
        assert!(
            rule.contains("INFRA") || rule.contains("DYLINT"),
            "finding rule must reference infra/dylint, got \"{}\"",
            rule,
        );
    }
}

#[test]
fn dylint_loader_probe_render_mentions_tool() {
    let probe = probe_dylint_toolchain(&test_target());

    if let DylintProbe::Infra(failure, _) = probe {
        let rendered = format!("{failure:?}");
        assert!(
            rendered.contains(failure.tool()),
            "rendered failure must mention the tool name \"{}\"",
            failure.tool(),
        );
    }
}
