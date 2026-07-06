//! Library runner for the v1 panic-scan lane.

#[path = "panic_scan_lane/paths.rs"]
mod paths;
#[path = "panic_scan_lane/scan.rs"]
mod scan;

use std::path::Path;

use titania_core::TargetProject;

use crate::{Finding, LaneReport, RuleId, RuleIdError, SourceLine};

const PANIC_SURFACE_RULE: &str = "PANIC_SURFACE_001";
const RG_TOOL: &str = "rg";

const EXCLUDED_SEGMENTS: &[&str] = &[
    "/workspace_tests/",
    "/test_loop_inventory/",
    "/tests/",
    "/lifecycle_tests/",
    "/benches/",
    "/examples/",
    "/proofs/",
    "/models/loom/",
    "/target/",
    "/.beads/",
    "/.titania/",
    "/fixtures/",
    "/fuzz/",
    "/titania-lanes/src/bin/",
];

#[derive(Clone, Copy)]
struct PanicMacroRule {
    macro_name: &'static str,
    rule_id: &'static str,
}

impl PanicMacroRule {
    #[must_use]
    const fn macro_name(self) -> &'static str {
        self.macro_name
    }

    #[must_use]
    const fn rule_id(self) -> &'static str {
        self.rule_id
    }
}

const PANIC_MACROS: &[PanicMacroRule] = &[
    PanicMacroRule { macro_name: "assert!", rule_id: "HOLZMAN_PANIC_ASSERT" },
    PanicMacroRule { macro_name: "assert_eq!", rule_id: "HOLZMAN_PANIC_ASSERT_EQ" },
    PanicMacroRule { macro_name: "assert_ne!", rule_id: "HOLZMAN_PANIC_ASSERT_NE" },
    PanicMacroRule { macro_name: "unreachable!", rule_id: "HOLZMAN_PANIC_UNREACHABLE" },
];

pub(super) enum PanicRun {
    Report(LaneReport),
    Infra(String),
    RuleId(RuleIdError),
}

pub(super) fn run(target: &TargetProject) -> PanicRun {
    if !rg_available(target) {
        return PanicRun::Infra(String::from("tool rg unavailable for panic-scan"));
    }
    if let Err(error) = panic_surface_rules() {
        return PanicRun::RuleId(error);
    }
    PanicRun::Report(scan_target(target.as_std_path()))
}

/// Validate panic-surface rule identifiers at startup.
///
/// # Errors
/// Returns [`RuleIdError`] when an embedded rule id is invalid.
fn panic_surface_rules() -> Result<(), RuleIdError> {
    PANIC_MACROS.iter().try_for_each(|rule| crate::RuleId::new(rule.rule_id()).map(|_| ()))
}

fn scan_target(root: &Path) -> LaneReport {
    paths::collect_source_files(root).iter().fold(LaneReport::new(), |mut report, file| {
        scan::scan_file(root, file, &mut report);
        report
    })
}
fn rg_available(target: &TargetProject) -> bool {
    crate::command::CommandIn::new(target, RG_TOOL)
        .and_then(|mut cmd| cmd.arg("--version").run_capture_raw())
        .is_ok_and(|output| output.success())
}
