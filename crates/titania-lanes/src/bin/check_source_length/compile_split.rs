use std::path::Path;

use titania_lanes::{Finding, LaneReport, RuleId, RuleIdError, helpers::relative_path};

const SPLIT_MODULES: &[&str] = &[
    "mod_compile_core.rs",
    "mod_compile_errors.rs",
    "mod_compile_validation.rs",
    "mod_compile_lowering.rs",
];
const COMPILE_SPLIT_RULE: &str = "COMPILE_SPLIT";

/// Check legacy compile split source layout.
///
/// # Errors
///
/// Returns a rule-id construction error when the compile-split rule id is
/// invalid.
pub(super) fn check_compile_split_sources(
    root: &Path,
    report: &mut LaneReport,
) -> Result<(), RuleIdError> {
    let rule = RuleId::new(COMPILE_SPLIT_RULE)?;
    let compile_dir = root.join("crates/vb_compile/src");
    if !compile_dir.is_dir() {
        let _emitted = crate::write_stderr_line(format_args!(
            "NotApplicable: legacy compile split directory absent"
        ))
        .is_ok();
        return Ok(());
    }
    check_impl_body(root, &compile_dir, &rule, report);
    SPLIT_MODULES
        .iter()
        .fold((), |(), name| check_split_module(root, &compile_dir, name, &rule, report));
    Ok(())
}

fn check_impl_body(root: &Path, compile_dir: &Path, rule: &RuleId, report: &mut LaneReport) {
    let impl_body = compile_dir.join("compile_core_impl.rs");
    if impl_body.is_file() {
        report.push(Finding::new(
            rule.clone(),
            relative_path(root, &impl_body),
            0,
            "hidden production include body must not remain",
        ));
    }
}

fn check_split_module(
    root: &Path,
    compile_dir: &Path,
    name: &str,
    rule: &RuleId,
    report: &mut LaneReport,
) {
    let path = compile_dir.join(name);
    if !path.is_file() {
        push_missing_module(name, rule, report);
        return;
    }
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    check_module_text(root, &path, &text, rule, report);
}

fn check_module_text(root: &Path, path: &Path, text: &str, rule: &RuleId, report: &mut LaneReport) {
    if text.contains("include!(") {
        push_compile_split(root, path, "contains monolithic include body", rule, report);
    }
    if is_doc_only_shell(text) {
        push_compile_split(
            root,
            path,
            "doc-only shell, not an owned implementation module",
            rule,
            report,
        );
    }
}

fn is_doc_only_shell(text: &str) -> bool {
    let line_count = text.lines().count();
    let has_mod = text.lines().any(|line| line.trim_start().starts_with("mod "));
    line_count < 50 && !has_mod
}

fn push_compile_split(
    root: &Path,
    path: &Path,
    message: &str,
    rule: &RuleId,
    report: &mut LaneReport,
) {
    report.push(Finding::new(rule.clone(), relative_path(root, path), 0, message));
}

fn push_missing_module(name: &str, rule: &RuleId, report: &mut LaneReport) {
    report.push(Finding::new(
        rule.clone(),
        format!("crates/vb_compile/src/{name}"),
        0,
        "missing from compile split",
    ));
}
