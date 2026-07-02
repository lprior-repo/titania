use std::{collections::BTreeSet, fs, io, io::Write};

use titania_core::TargetProject;
use titania_lanes::{Finding, LaneReport, RuleId};

use crate::{
    model::{CHECKS, Check, DomainFile, Oracle, TargetRelativePath},
    parser::{collect_qualified_refs, extract_enum_variants, find_function_body},
};

type VariantSet = BTreeSet<String>;
type CheckFindings = Vec<Finding>;
type EnumScan = (Option<VariantSet>, CheckFindings);

/// Run every configured error-exhaustiveness check and append findings.
///
/// # Errors
/// Returns an I/O error when status output cannot be written to stderr.
pub fn run(target: &TargetProject, rule: &RuleId, report: &mut LaneReport) -> io::Result<()> {
    let findings = CHECKS.iter().try_fold(Vec::new(), |mut findings, check| {
        findings.extend(run_check(target, check, rule)?);
        Ok::<_, io::Error>(findings)
    })?;
    report.extend_finding(findings);
    Ok(())
}

/// Run one configured error-exhaustiveness check.
///
/// # Errors
///
/// Returns an I/O error when diagnostic output for not-applicable or passing
/// checks cannot be written.
fn run_check(target: &TargetProject, check: &Check, rule: &RuleId) -> io::Result<CheckFindings> {
    let (variants, mut findings) = enum_variants(target, check, rule)?;
    if let Some(variants) = variants {
        let oracle_findings = oracle_findings(target, check, &variants, rule)?;
        findings.extend(oracle_findings);
    }
    Ok(findings)
}

/// Parse the production error enum for a configured check.
///
/// # Errors
///
/// Returns an I/O error when a not-applicable diagnostic cannot be written.
fn enum_variants(target: &TargetProject, check: &Check, rule: &RuleId) -> io::Result<EnumScan> {
    let text = match read_optional_file(target, check.enum_path) {
        DomainFile::Present(text) => text,
        DomainFile::Absent => return Ok((note_not_applicable(check)?, Vec::new())),
        DomainFile::Unreadable(kind) => {
            return Ok((None, vec![enum_unreadable_finding(check, rule, kind)]));
        }
    };
    let variants = extract_enum_variants(&text, check.type_name);
    if variants.is_empty() {
        Ok((None, vec![empty_variants_finding(check, rule)]))
    } else {
        Ok((Some(variants), Vec::new()))
    }
}

fn enum_unreadable_finding(check: &Check, rule: &RuleId, kind: std::io::ErrorKind) -> Finding {
    Finding::new(
        rule.clone(),
        check.enum_path.as_str(),
        0,
        format!("enum file not readable: {kind:?}"),
    )
}

fn empty_variants_finding(check: &Check, rule: &RuleId) -> Finding {
    Finding::new(
        rule.clone(),
        check.enum_path.as_str(),
        0,
        format!("no variants parsed for {}", check.type_name),
    )
}

/// Emit a not-applicable diagnostic for an absent enum file.
///
/// # Errors
///
/// Returns the stderr write error if diagnostic output fails.
fn note_not_applicable(check: &Check) -> io::Result<Option<VariantSet>> {
    write_stderr_line(&format!(
        "[check-error-exhaustiveness] not applicable: {} absent; skipping {} ({}) exhaustiveness",
        check.enum_path.as_str(),
        check.type_name,
        check.domain_label
    ))?;
    Ok(None)
}

/// Run every oracle for one parsed variant set.
///
/// # Errors
///
/// Returns an I/O error when a passing-oracle diagnostic cannot be written.
fn oracle_findings(
    target: &TargetProject,
    check: &Check,
    variants: &VariantSet,
    rule: &RuleId,
) -> io::Result<CheckFindings> {
    check.oracles.iter().try_fold(Vec::new(), |mut findings, oracle| {
        findings.extend(check_oracle(target, check, oracle, variants, rule)?);
        Ok::<_, io::Error>(findings)
    })
}

/// Check one oracle function body against parsed enum variants.
///
/// # Errors
///
/// Returns an I/O error when a passing-oracle diagnostic cannot be written.
fn check_oracle(
    target: &TargetProject,
    check: &Check,
    oracle: &Oracle,
    variants: &VariantSet,
    rule: &RuleId,
) -> io::Result<CheckFindings> {
    let abs = oracle.path.in_target(target);
    let Ok(text) = fs::read_to_string(&abs) else {
        return Ok(vec![Finding::new(
            rule.clone(),
            oracle.path.as_str(),
            0,
            format!("oracle {} file not readable", oracle.function),
        )]);
    };
    let Some(body) = find_function_body(&text, oracle.function) else {
        return Ok(vec![Finding::new(
            rule.clone(),
            oracle.path.as_str(),
            0,
            format!("function {} not found", oracle.function),
        )]);
    };
    let mentions = collect_qualified_refs(&body, check.type_name);
    let missing = missing_variants(variants, &mentions);
    if missing.is_empty() {
        print_ok(check, oracle, variants.len())?;
        Ok(Vec::new())
    } else {
        Ok(vec![missing_finding(check, oracle, missing, rule)])
    }
}

fn missing_variants(variants: &VariantSet, mentions: &VariantSet) -> Vec<String> {
    variants.iter().filter(|variant| !mentions.contains(*variant)).cloned().collect()
}

fn missing_finding(
    check: &Check,
    oracle: &Oracle,
    mut missing: Vec<String>,
    rule: &RuleId,
) -> Finding {
    missing.sort();
    Finding::new(
        rule.clone(),
        oracle.path.as_str(),
        0,
        format!("{} missing {}: {}", check.type_name, oracle.function, missing.join(",")),
    )
}

/// Write an OK line for an oracle that covers every variant.
///
/// # Errors
///
/// Returns the stderr write error if diagnostic output fails.
fn print_ok(check: &Check, oracle: &Oracle, variant_count: usize) -> io::Result<()> {
    write_stderr_line(&format!(
        "  OK {} in {}::{} ({} variants)",
        check.type_name,
        oracle.path.as_str(),
        oracle.function,
        variant_count
    ))
}

/// Write a line to stderr.
///
/// # Errors
///
/// Returns the stderr write error if text or newline output fails.
fn write_stderr_line(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())?;
    stderr.write_all(b"\n")
}

fn read_optional_file(target: &TargetProject, rel: TargetRelativePath) -> DomainFile {
    match fs::read_to_string(rel.in_target(target)) {
        Ok(text) => DomainFile::Present(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => DomainFile::Absent,
        Err(error) => DomainFile::Unreadable(error.kind()),
    }
}
