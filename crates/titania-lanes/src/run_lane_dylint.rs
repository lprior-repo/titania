use std::path::{Path, PathBuf};

use titania_core::{Lane, LaneFailure, LaneOutcome, TargetProject};

use crate::{
    CommandIn, CommandOutput, Finding, LaneError, LaneReport, RuleId,
    dylint_lane::{
        DylintLibStaging, DylintLoad, DylintProbe, path_needs_staging, probe_dylint_toolchain,
    },
};

const CARGO_DYLINT_TOOL: &str = "cargo-dylint";
const DYLINT_LOAD_FAILURE_MARKER: &str = "could not find `dylint_version` in `";
const DYLINT_LOCATION_LOOKAHEAD: usize = 6;
const DYLINT_BASE_ARGS: &[&str] =
    &["dylint", "--workspace", "--all", "--", "--lib", "--bins", "--examples"];
const DYLINT_RULE_IDS: &[&str] = &[
    "FUNC_UNWRAP_USED",
    "FUNC_EXPECT_USED",
    "FUNC_UNWRAP_OR",
    "FUNC_RESULT_OK",
    "BOUNDED_EXECUTION_UNBOUNDED_LOOP",
    "BOUNDED_EXECUTION_UNBOUNDED_ITERATOR",
    "HOLZMAN_PANIC_PANIC",
    "HOLZMAN_PANIC_ASSERT",
    "HOLZMAN_PANIC_ASSERT_EQ",
    "HOLZMAN_PANIC_ASSERT_NE",
    "HOLZMAN_PANIC_TODO",
    "HOLZMAN_PANIC_UNIMPLEMENTED",
    "HOLZMAN_PANIC_UNREACHABLE",
    "HOLZMAN_PANIC_DBG",
    "BYPASS_PUB_ALLOW",
    "BYPASS_ATTR_CONTEXT",
    "BYPASS_REQUIRED_LINT_WEAKENING",
    "BYPASS_INTERNAL_UNSTABLE",
    "BYPASS_INTERNAL_UNSAFE",
];

pub(super) fn outcome(target: &TargetProject) -> LaneOutcome {
    let load = match probe_dylint_toolchain(target) {
        DylintProbe::Infra(failure, _) => {
            return LaneOutcome::Failed { failure };
        }
        DylintProbe::Ready(load) => load,
    };

    let (effective_path, _staging) = match resolve_effective_lib_path(&load, target) {
        Ok(pair) => pair,
        Err(failure) => return LaneOutcome::Failed { failure },
    };

    let args = match cargo_dylint_args(effective_path.as_deref()) {
        Ok(args) => args,
        Err(failure) => return LaneOutcome::Failed { failure },
    };

    let output = match dylint_output(target, &args) {
        Ok(output) => output,
        Err(error) => return failure_outcome(&error),
    };

    if output.success() {
        return super::run_lane_outcome::clean_outcome_unchecked(
            Lane::Dylint,
            CARGO_DYLINT_TOOL,
            output.stdout(),
        );
    }
    nonzero_outcome(target, &output, effective_path.as_deref())
}

type StagedLibPath = (Option<PathBuf>, Option<DylintLibStaging>);

fn resolve_effective_lib_path(
    load: &DylintLoad,
    target: &TargetProject,
) -> Result<StagedLibPath, LaneFailure> {
    match load {
        DylintLoad::Metadata => Ok((None, None)),
        DylintLoad::LibraryPath { path, .. } => Ok(stage_if_needed(path, target)?),
    }
}

fn stage_if_needed(path: &Path, target: &TargetProject) -> Result<StagedLibPath, LaneFailure> {
    let staging =
        if path_needs_staging(path) { Some(DylintLibStaging::stage(path, target)?) } else { None };
    let effective_path =
        staging.as_ref().map_or_else(|| path.to_owned(), |s| s.lib_path().to_owned());
    Ok((Some(effective_path), staging))
}

fn cargo_dylint_args(effective_path: Option<&Path>) -> Result<Vec<String>, LaneFailure> {
    let args = base_dylint_args();
    match effective_path {
        None => Ok(args),
        Some(path) => args_with_library_path(args, path),
    }
}

fn base_dylint_args() -> Vec<String> {
    DYLINT_BASE_ARGS.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>()
}

fn args_with_library_path(mut args: Vec<String>, path: &Path) -> Result<Vec<String>, LaneFailure> {
    let path_text = library_path_text(path)?;
    let separator_index = args.iter().position(|arg| arg == "--").map_or(args.len(), |index| index);
    args.insert(separator_index, path_text.to_owned());
    args.insert(separator_index, String::from("--lib-path"));
    Ok(args)
}

fn library_path_text(path: &Path) -> Result<&str, LaneFailure> {
    path.to_str().ok_or_else(|| LaneFailure::Infra {
        tool: String::from(CARGO_DYLINT_TOOL),
        reason: format!("Dylint library path is not valid UTF-8: {}", path.display()),
    })
}

fn dylint_output(target: &TargetProject, args: &[String]) -> Result<CommandOutput, LaneError> {
    let mut command = CommandIn::new(target, "cargo")?;
    let _ = command.inherit_env().args_strings(args);
    command.run_capture_raw()
}

fn nonzero_outcome(
    target: &TargetProject,
    output: &CommandOutput,
    expected_lib_path: Option<&Path>,
) -> LaneOutcome {
    let report = match dylint_report(target, output) {
        Ok(report) => report,
        Err(failure) => return LaneOutcome::Failed { failure },
    };
    if !report.is_clean() {
        return super::run_lane_outcome::findings_outcome(Lane::Dylint, &report);
    }
    if let Some(failure) = load_failure(output, expected_lib_path) {
        return LaneOutcome::Failed { failure };
    }
    LaneOutcome::Failed {
        failure: LaneFailure::Suspicious {
            tool: String::from(CARGO_DYLINT_TOOL),
            evidence: failure_evidence(output),
        },
    }
}

fn load_failure(output: &CommandOutput, expected_lib_path: Option<&Path>) -> Option<LaneFailure> {
    let text = match combined_output(output) {
        Ok(text) => text,
        Err(_output_error) => return None,
    };
    let line = text.lines().find(|line| is_driver_load_failure(line, expected_lib_path))?;
    Some(LaneFailure::Infra {
        tool: String::from(CARGO_DYLINT_TOOL),
        reason: format!("cargo-dylint could not load the dylint library: {}", line.trim()),
    })
}

fn is_driver_load_failure(line: &str, expected_lib_path: Option<&Path>) -> bool {
    let Some(expected) = expected_lib_path else {
        return false;
    };
    let Some((_, rest)) = line.split_once(DYLINT_LOAD_FAILURE_MARKER) else {
        return false;
    };
    let Some((quoted_path, _)) = rest.split_once('`') else {
        return false;
    };
    quoted_path_names_expected_lib(quoted_path, expected)
}

fn quoted_path_names_expected_lib(quoted_path: &str, expected: &Path) -> bool {
    expected.to_str().is_some_and(|text| text == quoted_path)
        || canonical_path_text(expected).is_some_and(|text| text == quoted_path)
}

fn canonical_path_text(path: &Path) -> Option<String> {
    let canonical = match std::fs::canonicalize(path) {
        Ok(canonical) => canonical,
        Err(_) => return None,
    };
    canonical.to_str().map(str::to_owned)
}

fn dylint_report(
    target: &TargetProject,
    output: &CommandOutput,
) -> Result<LaneReport, LaneFailure> {
    let text = combined_output(output)?;
    let lines = text.lines().collect::<Vec<_>>();
    lines.iter().enumerate().try_fold(LaneReport::new(), |report, (position, line)| {
        add_dylint_line_finding(target, &lines, position, line, report)
    })
}

fn combined_output(output: &CommandOutput) -> Result<String, LaneFailure> {
    let stdout = output.stdout_str().map_err(|error| lane_error_failure(&error))?;
    let stderr = output.stderr_str().map_err(|error| lane_error_failure(&error))?;
    Ok(format!("{stdout}\n{stderr}"))
}

fn add_dylint_line_finding(
    target: &TargetProject,
    lines: &[&str],
    position: usize,
    line: &str,
    mut report: LaneReport,
) -> Result<LaneReport, LaneFailure> {
    let Some(rule_id) = dylint_rule_id(line) else {
        return Ok(report);
    };
    report.push(dylint_finding(target, lines, position, line, rule_id)?);
    Ok(report)
}

fn dylint_rule_id(line: &str) -> Option<&'static str> {
    DYLINT_RULE_IDS.iter().copied().find(|rule_id| line_has_rule_id(line, rule_id))
}

fn line_has_rule_id(line: &str, rule_id: &str) -> bool {
    line.split(|ch: char| !is_rule_id_char(ch)).any(|token| token == rule_id)
}

const fn is_rule_id_char(ch: char) -> bool {
    matches!(ch, 'A'..='Z' | '0'..='9' | '_')
}

fn dylint_finding(
    target: &TargetProject,
    lines: &[&str],
    position: usize,
    line: &str,
    rule_id: &'static str,
) -> Result<Finding, LaneFailure> {
    let rule = RuleId::new(rule_id).map_err(|error| LaneFailure::Infra {
        tool: String::from(CARGO_DYLINT_TOOL),
        reason: format!("invalid Dylint rule id {rule_id}: {error}"),
    })?;
    let location = nearest_location(target, lines, position);
    Ok(Finding::new(rule, location.path, location.line, dylint_message(rule_id, line)))
}

fn dylint_message(rule_id: &str, line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        format!("{rule_id} emitted by cargo-dylint")
    } else {
        trimmed.to_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DiagnosticLocation {
    path: String,
    line: u32,
}

impl DiagnosticLocation {
    const fn workspace() -> Self {
        Self { path: String::new(), line: 0 }
    }
}

enum LocationSearch {
    Found(DiagnosticLocation),
    Missing,
}

impl LocationSearch {
    fn or_else(self, fallback: impl FnOnce() -> Self) -> Self {
        match self {
            found @ Self::Found(_) => found,
            Self::Missing => fallback(),
        }
    }

    fn into_location(self) -> DiagnosticLocation {
        match self {
            Self::Found(location) => location,
            Self::Missing => DiagnosticLocation::workspace(),
        }
    }
}

fn nearest_location(target: &TargetProject, lines: &[&str], position: usize) -> DiagnosticLocation {
    next_location(target, lines, position)
        .or_else(|| previous_location(target, lines, position))
        .into_location()
}

fn next_location(target: &TargetProject, lines: &[&str], position: usize) -> LocationSearch {
    location_search(
        lines
            .iter()
            .skip(position)
            .take(DYLINT_LOCATION_LOOKAHEAD)
            .find_map(|line| parse_diagnostic_location(target, line)),
    )
}

fn previous_location(target: &TargetProject, lines: &[&str], position: usize) -> LocationSearch {
    location_search(
        lines.iter().take(position).rev().find_map(|line| parse_diagnostic_location(target, line)),
    )
}

fn location_search(location: Option<DiagnosticLocation>) -> LocationSearch {
    location.map_or(LocationSearch::Missing, LocationSearch::Found)
}

fn parse_diagnostic_location(target: &TargetProject, line: &str) -> Option<DiagnosticLocation> {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix("-->") {
        return parse_colon_location(target, rest.trim_start());
    }
    parse_colon_location(target, trimmed)
}

fn parse_colon_location(target: &TargetProject, text: &str) -> Option<DiagnosticLocation> {
    let (path_and_line, _column) = text.rsplit_once(':')?;
    let (path, line_text) = path_and_line.rsplit_once(':')?;
    let Ok(line) = line_text.parse::<u32>() else {
        return None;
    };
    Some(DiagnosticLocation { path: relative_output_path(target.as_std_path(), path), line })
}

fn relative_output_path(root: &Path, path: &str) -> String {
    let candidate = Path::new(path);
    if !candidate.is_absolute() {
        return path.to_owned();
    }
    let Ok(stripped) = candidate.strip_prefix(root) else {
        return path.to_owned();
    };
    stripped.to_str().map_or_else(|| path.to_owned(), ToOwned::to_owned)
}

fn failure_evidence(output: &CommandOutput) -> String {
    output
        .stderr_str()
        .map_or_else(|_| String::from("<non-UTF-8>"), |stderr| stderr_or_status(stderr, output))
}

fn stderr_or_status(stderr: &str, output: &CommandOutput) -> String {
    if stderr.is_empty() {
        format!("cargo dylint exited with code {:?}", output.status().code())
    } else {
        stderr.to_owned()
    }
}

fn failure_outcome(error: &LaneError) -> LaneOutcome {
    LaneOutcome::Failed { failure: lane_error_failure(error) }
}

fn lane_error_failure(error: &LaneError) -> LaneFailure {
    LaneFailure::Infra { tool: String::from(CARGO_DYLINT_TOOL), reason: error.to_string() }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use titania_core::LaneOutcome;

    use super::{CARGO_DYLINT_TOOL, cargo_dylint_args, nonzero_outcome};
    use crate::{command::mock_output, current_target_project};

    fn staged_lib_path() -> PathBuf {
        PathBuf::from(
            "/tmp/titania-dylint-4242/libtitania_dylint@nightly-2026-04-27-x86_64-unknown-linux-gnu.so",
        )
    }

    #[test]
    fn env_library_load_uses_lib_path_args() {
        let path = PathBuf::from("/tmp/libtitania_dylint.so");
        let args =
            cargo_dylint_args(Some(path.as_path())).expect("valid UTF-8 path must produce args");

        assert_eq!(
            args,
            vec![
                String::from("dylint"),
                String::from("--workspace"),
                String::from("--all"),
                String::from("--lib-path"),
                path.to_string_lossy().into_owned(),
                String::from("--"),
                String::from("--lib"),
                String::from("--bins"),
                String::from("--examples"),
            ]
        );
    }

    #[test]
    fn metadata_load_does_not_use_lib_path_args() {
        let args = cargo_dylint_args(None).expect("metadata load must produce base args");

        assert_eq!(
            args,
            vec![
                String::from("dylint"),
                String::from("--workspace"),
                String::from("--all"),
                String::from("--"),
                String::from("--lib"),
                String::from("--bins"),
                String::from("--examples"),
            ]
        );
    }

    #[test]
    fn nonzero_bypass_output_becomes_findings() {
        let target = current_target_project().expect("test target project must resolve");
        let output = mock_output(
            1,
            b"",
            b"warning: BYPASS_PUB_ALLOW: public API item weakens lint policy\n  --> src/lib.rs:7:1\n",
        )
        .expect("mock output must build");

        let outcome = nonzero_outcome(&target, &output, None);

        let LaneOutcome::Findings { findings } = outcome else {
            panic!("BYPASS lint output must become findings");
        };
        assert_eq!(findings.len(), 1);
        let first = findings.first().expect("one finding must exist");
        assert_eq!(first.rule_id().as_str(), "BYPASS_PUB_ALLOW");
        assert!(first.location().is_span(), "Dylint arrow location must become a span");
    }

    #[test]
    fn typed_bypass_rule_ids_survive() {
        let target = current_target_project().expect("test target project must resolve");
        let output = mock_output(
            1,
            b"warning: BYPASS_INTERNAL_UNSAFE: macro uses #[allow_internal_unsafe]\n  --> src/macros.rs:3:1\n",
            b"warning: BYPASS_REQUIRED_LINT_WEAKENING: crate-level allow weakens required lint\n  --> src/lib.rs:1:1\n",
        )
        .expect("mock output must build");

        let outcome = nonzero_outcome(&target, &output, None);

        let LaneOutcome::Findings { findings } = outcome else {
            panic!("BYPASS lint output must become findings");
        };
        let rule_ids =
            findings.iter().map(|finding| finding.rule_id().as_str()).collect::<Vec<_>>();
        assert_eq!(rule_ids, vec!["BYPASS_INTERNAL_UNSAFE", "BYPASS_REQUIRED_LINT_WEAKENING"]);
    }

    #[test]
    fn result_ok_output_becomes_typed_finding() {
        let target = current_target_project().expect("test target project must resolve");
        let output = mock_output(
            1,
            b"",
            b"warning: FUNC_RESULT_OK: replace ok() erasure with explicit typed error handling\n  --> src/lib.rs:9:1\n",
        )
        .expect("mock output must build");

        let outcome = nonzero_outcome(&target, &output, None);

        let LaneOutcome::Findings { findings } = outcome else {
            panic!("FUNC_RESULT_OK output must become findings, not a suspicious failure");
        };
        assert_eq!(findings.len(), 1);
        let first = findings.first().expect("one finding must exist");
        assert_eq!(first.rule_id().as_str(), "FUNC_RESULT_OK");
        assert!(first.is_reject(), "FUNC_RESULT_OK must carry the Reject effect");
        assert_eq!(
            first.repair().class(),
            titania_core::RepairHintClass::RequiresHumanReview,
            "FUNC_RESULT_OK must resolve to its catalog repair hint, not an unmapped fallback"
        );
        assert!(first.location().is_span(), "Dylint arrow location must become a span");
    }

    #[test]
    fn non_bypass_nonzero_remains_suspicious_failure() {
        let target = current_target_project().expect("test target project must resolve");
        let output = mock_output(1, b"", b"error: could not compile fixture\n")
            .expect("mock output must build");

        let outcome = nonzero_outcome(&target, &output, Some(staged_lib_path().as_path()));

        let LaneOutcome::Failed { failure } = outcome else {
            panic!("non-BYPASS output must remain a failure");
        };
        assert_eq!(failure.tool(), CARGO_DYLINT_TOOL);
        assert!(!failure.is_infra(), "compile errors must not become infra failures");
    }

    #[test]
    fn driver_load_failure_becomes_infra_failure() {
        let target = current_target_project().expect("test target project must resolve");
        let output = mock_output(
            1,
            b"",
            b"error: could not find `dylint_version` in `/tmp/libtitania_dylint@nightly.so`\n\nerror: aborting due to 1 previous error\n",
        )
        .expect("mock output must build");

        let outcome =
            nonzero_outcome(&target, &output, Some(Path::new("/tmp/libtitania_dylint@nightly.so")));

        let LaneOutcome::Failed { failure } = outcome else {
            panic!("driver load failure must become a lane failure");
        };
        assert_eq!(failure.tool(), CARGO_DYLINT_TOOL);
        assert!(failure.is_infra(), "driver load failure must be an infra failure: {failure:?}");
    }

    #[test]
    fn forged_marker_with_fake_path_remains_suspicious_failure() {
        let target = current_target_project().expect("test target project must resolve");
        let output = mock_output(
            1,
            b"",
            b"   Compiling forged v0.1.0 (/tmp/forged)\nerror: failed to run custom build command for `forged v0.1.0 (/tmp/forged)`\n\nCaused by:\n  process didn't exit successfully: `/tmp/forged/build-script-build` (exit status: 1)\n  --- stderr\n  error: could not find `dylint_version` in `/tmp/fake.so`\n",
        )
        .expect("mock output must build");

        let outcome =
            nonzero_outcome(&target, &output, Some(Path::new("/tmp/libtitania_dylint@nightly.so")));

        let LaneOutcome::Failed { failure } = outcome else {
            panic!("forged marker output must become a lane failure");
        };
        assert_eq!(failure.tool(), CARGO_DYLINT_TOOL);
        assert!(
            !failure.is_infra(),
            "subject-printed marker naming a fake path must not become an infra failure: {failure:?}"
        );
    }

    #[test]
    fn marker_naming_unversioned_library_remains_suspicious_failure() {
        let target = current_target_project().expect("test target project must resolve");
        let output = mock_output(
            1,
            b"",
            b"error: could not find `dylint_version` in `/tmp/libtitania_dylint.so`\n\nerror: aborting due to 1 previous error\n",
        )
        .expect("mock output must build");

        let outcome =
            nonzero_outcome(&target, &output, Some(Path::new("/tmp/libtitania_dylint@nightly.so")));

        let LaneOutcome::Failed { failure } = outcome else {
            panic!("unversioned marker output must become a lane failure");
        };
        assert_eq!(failure.tool(), CARGO_DYLINT_TOOL);
        assert!(
            !failure.is_infra(),
            "marker without the toolchain-versioned artifact name must not become an infra failure: {failure:?}"
        );
    }

    #[test]
    fn driver_load_failure_naming_staged_path_becomes_infra_failure() {
        let target = current_target_project().expect("test target project must resolve");
        let expected = staged_lib_path();
        let stderr = format!(
            "error: could not find `dylint_version` in `{}`\n\nerror: aborting due to 1 previous error\n",
            expected.display()
        );
        let output = mock_output(1, b"", stderr.as_bytes()).expect("mock output must build");

        let outcome = nonzero_outcome(&target, &output, Some(expected.as_path()));

        let LaneOutcome::Failed { failure } = outcome else {
            panic!("driver load failure naming the staged path must become a lane failure");
        };
        assert_eq!(failure.tool(), CARGO_DYLINT_TOOL);
        assert!(
            failure.is_infra(),
            "driver load failure naming the staged path must be an infra failure: {failure:?}"
        );
    }

    #[test]
    fn forged_marker_naming_public_artifact_path_remains_suspicious_failure() {
        let target = current_target_project().expect("test target project must resolve");
        let expected = staged_lib_path();
        let output = mock_output(
            1,
            b"",
            b"   Compiling forge-named v0.1.0 (/tmp/forge-named)\nerror: failed to run custom build command for `forge-named v0.1.0 (/tmp/forge-named)`\n\nCaused by:\n  process didn't exit successfully: `/tmp/forge-named/build-script-build` (exit status: 1)\n  --- stderr\n  error: could not find `dylint_version` in `/tmp/forged/libtitania_dylint@nightly-2026-04-27.so`\n",
        )
        .expect("mock output must build");

        let outcome = nonzero_outcome(&target, &output, Some(expected.as_path()));

        let LaneOutcome::Failed { failure } = outcome else {
            panic!("forged marker output must become a lane failure");
        };
        assert_eq!(failure.tool(), CARGO_DYLINT_TOOL);
        assert!(
            !failure.is_infra(),
            "subject-printed marker naming a public forged path must not become an infra failure: {failure:?}"
        );
    }

    #[test]
    fn compile_error_marker_naming_forged_path_remains_suspicious_failure() {
        let target = current_target_project().expect("test target project must resolve");
        let expected = staged_lib_path();
        let output = mock_output(
            1,
            b"",
            b"error: could not find `dylint_version` in `/tmp/forged/libtitania_dylint@nightly-2026-04-27.so`\n --> src/lib.rs:1:1\n  |\n1 | compile_error!(\"could not find `dylint_version` in `/tmp/forged/libtitania_dylint@nightly-2026-04-27.so`\");\n  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^\n\nerror: aborting due to 1 previous error\n",
        )
        .expect("mock output must build");

        let outcome = nonzero_outcome(&target, &output, Some(expected.as_path()));

        let LaneOutcome::Failed { failure } = outcome else {
            panic!("compile_error marker output must become a lane failure");
        };
        assert_eq!(failure.tool(), CARGO_DYLINT_TOOL);
        assert!(
            !failure.is_infra(),
            "compile_error! carrying the marker text must not become an infra failure: {failure:?}"
        );
    }

    #[test]
    fn marker_without_known_lib_path_remains_suspicious_failure() {
        let target = current_target_project().expect("test target project must resolve");
        let expected = staged_lib_path();
        let stderr = format!(
            "error: could not find `dylint_version` in `{}`\n\nerror: aborting due to 1 previous error\n",
            expected.display()
        );
        let output = mock_output(1, b"", stderr.as_bytes()).expect("mock output must build");

        let outcome = nonzero_outcome(&target, &output, None);

        let LaneOutcome::Failed { failure } = outcome else {
            panic!("marker output without a known lib path must become a lane failure");
        };
        assert_eq!(failure.tool(), CARGO_DYLINT_TOOL);
        assert!(
            !failure.is_infra(),
            "marker line must not become an infra failure when the lane staged no library path: {failure:?}"
        );
    }

    #[test]
    fn marker_naming_other_toolchain_path_remains_suspicious_failure() {
        let target = current_target_project().expect("test target project must resolve");
        let expected = staged_lib_path();
        let output = mock_output(
            1,
            b"",
            b"error: could not find `dylint_version` in `/tmp/titania-dylint-4242/libtitania_dylint@nightly-1999-01-01-x86_64-unknown-linux-gnu.so`\n\nerror: aborting due to 1 previous error\n",
        )
        .expect("mock output must build");

        let outcome = nonzero_outcome(&target, &output, Some(expected.as_path()));

        let LaneOutcome::Failed { failure } = outcome else {
            panic!("other-toolchain marker output must become a lane failure");
        };
        assert_eq!(failure.tool(), CARGO_DYLINT_TOOL);
        assert!(
            !failure.is_infra(),
            "marker naming a different artifact path in the same directory must not become an infra failure: {failure:?}"
        );
    }
}
