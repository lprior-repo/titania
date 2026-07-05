fn usage_requested(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

/// Emits command usage text to stderr.
///
/// # Errors
///
/// Returns an I/O error when writing to stderr fails.
fn emit_usage() -> io::Result<()> {
    write_stderr_line(
        "usage: check-public-api-diff\n\
         Discovers vb_* workspace packages and runs\n\
         `cargo public-api diff origin/main..HEAD` through CommandIn.",
    )
}

/// Resolves the target project for this lane.
///
/// # Errors
///
/// Returns a lane exit code when target discovery fails or the rendered failure
/// report cannot be written to stderr.
fn resolve_target(report: &mut LaneReport, rules: &PubApiRules) -> Result<TargetProject, LaneExit> {
    match current_target_project() {
        Ok(target) => Ok(target),
        Err(error) => {
            report.push(Finding::new(
                rules.target.clone(),
                "Cargo.toml",
                0,
                format!("target discovery failed: {error}"),
            ));
            Err(render_report_or(report, LaneExit::Usage))
        }
    }
}

/// Resolves the package list checked by this lane.
///
/// # Errors
///
/// Returns a lane exit code when package discovery fails or the rendered failure
/// report cannot be written to stderr.
fn resolve_package_list(
    target: &TargetProject,
    report: &mut LaneReport,
    rules: &PubApiRules,
) -> Result<Vec<String>, LaneExit> {
    match discover_packages(target) {
        Ok(packages) => Ok(packages),
        Err(error) => Err(record_package_list_error(&error, report, rules)),
    }
}

fn record_package_list_error(
    error: &PackageDiscoveryError,
    report: &mut LaneReport,
    rules: &PubApiRules,
) -> LaneExit {
    let is_missing = error.is_missing_command();
    let (rule, code) = package_failure_rule(is_missing, rules);
    report.push(Finding::new(rule.clone(), "Cargo.toml", 0, error.to_string()));
    render_report_or(report, code)
}

const fn package_failure_rule(is_missing: bool, rules: &PubApiRules) -> (&RuleId, LaneExit) {
    if is_missing {
        (&rules.cargo_missing, LaneExit::Failure)
    } else {
        (&rules.metadata, LaneExit::Violations)
    }
}

fn render_report_or(report: &LaneReport, code: LaneExit) -> LaneExit {
    if write_stderr(&report.render()).is_err() { LaneExit::Failure } else { code }
}

fn report_no_packages() -> LaneExit {
    match write_stderr_line(
        "NotApplicable: no applicable vb_* Cargo packages discovered in workspace metadata",
    ) {
        Ok(()) => LaneExit::NotApplicable,
        Err(_) => LaneExit::Failure,
    }
}

fn run_diffs_and_emit(
    target: &TargetProject,
    packages: &[String],
    report: &mut LaneReport,
    rules: &PubApiRules,
) -> LaneExit {
    let exit_code = run_package_diffs(target, packages, rules, report);
    if exit_code != LaneExit::Clean && write_stderr(&report.render()).is_err() {
        return LaneExit::Failure;
    }
    exit_code
}

fn usage_exit() -> std::process::ExitCode {
    match emit_usage() {
        Ok(()) => exit(LaneExit::Usage),
        Err(_) => exit(LaneExit::Failure),
    }
}

/// Writes raw text to stderr.
///
/// # Errors
///
/// Returns an I/O error when stderr cannot accept the complete text.
fn write_stderr(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())
}

/// Writes one newline-terminated line to stderr.
///
/// # Errors
///
/// Returns an I/O error when writing either the text or trailing newline fails.
fn write_stderr_line(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())?;
    stderr.write_all(b"\n")
}

fn exit_after_stderr_line(text: &str, code: LaneExit) -> std::process::ExitCode {
    match write_stderr_line(text) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}
