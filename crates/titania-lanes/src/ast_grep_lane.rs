mod engine;
mod rules;
mod span;

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use rules::{RULES, RuleDef, repair_hint, rule_applies};
use thiserror::Error;
use titania_core::{
    CommandEvidence, Digest, Finding, FindingEffect, Lane, LaneEvidence, LaneOutcome, Location,
    LocationError, OutcomeError, ProcessTermination, RuleId, RuleIdError, WorkspacePath,
    WorkspacePathError,
};

#[derive(Debug, Error)]
pub enum AstGrepLaneError {
    #[error("path is not valid UTF-8: {path}")]
    NonUtf8Path { path: String },
    #[error("absolute path requires a target root or known fixture marker: {path}")]
    AbsolutePathWithoutRoot { path: String },
    #[error("source line index does not fit in u32: {index}")]
    LineNumberOverflow { index: usize },
    #[error("source column does not fit in u32: {column}")]
    ColumnOverflow { column: usize },
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("unknown ast-grep rule id: {id}")]
    UnknownRuleId { id: String },
    #[error(transparent)]
    RuleId(#[from] RuleIdError),
    #[error(transparent)]
    WorkspacePath(#[from] WorkspacePathError),
    #[error(transparent)]
    Location(#[from] LocationError),
    #[error(transparent)]
    Outcome(#[from] OutcomeError),
}

pub fn embedded_rule_ids() -> impl Iterator<Item = &'static str> {
    RULES.iter().map(|rule| rule.id)
}

pub fn run(
    rules_yaml: &[&'static str],
    fixture_paths: &[PathBuf],
    exceptions: &[(RuleId, String)],
) -> Result<LaneOutcome, AstGrepLaneError> {
    let catalog = catalog_text(rules_yaml)?;
    let findings = fixture_paths
        .iter()
        .map(|path| scan_path(path.as_path(), &catalog, exceptions))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .into_boxed_slice();

    if findings.is_empty() { clean_outcome() } else { Ok(LaneOutcome::Findings { findings }) }
}

fn scan_path(
    path: &Path,
    catalog: &str,
    exceptions: &[(RuleId, String)],
) -> Result<Vec<Finding>, AstGrepLaneError> {
    let source = read_source(path)?;
    let workspace_path = workspace_path(path)?;
    let engine = engine::AstEngine::new(&source);
    let offsets = engine.line_offsets();
    RULES
        .iter()
        .copied()
        .filter(|rule| rule_enabled(catalog, rule.id))
        .filter(|rule| rule_applies(rule, &workspace_path))
        .flat_map(|rule| {
            detect_sites(rule, &engine, &source)
                .into_iter()
                .map(move |site| (rule, site))
                .collect::<Vec<_>>()
        })
        .map(|(rule, site)| finding(rule, site, &source, offsets, &workspace_path))
        .collect::<Result<Vec<_>, _>>()
        .map(|findings| suppress_exceptions(findings, exceptions))
}

fn read_source(path: &Path) -> Result<String, AstGrepLaneError> {
    fs::read_to_string(path)
        .map_err(|source| AstGrepLaneError::ReadFile { path: path_display(path), source })
}

fn workspace_path(path: &Path) -> Result<WorkspacePath, AstGrepLaneError> {
    let rendered = path_to_str(path)?;
    let relative = match (path.is_absolute(), fixture_relative_path(&rendered)) {
        (true, Some(path)) => path,
        (true, None) => return Err(AstGrepLaneError::AbsolutePathWithoutRoot { path: rendered }),
        (false, _) => rendered,
    };
    WorkspacePath::new(&relative).map_err(Into::into)
}

fn fixture_relative_path(rendered: &str) -> Option<String> {
    let parts = rendered.split('/').collect::<Vec<_>>();
    parts
        .iter()
        .position(|part| *part == "ast_grep")
        .and_then(|index| index.checked_add(1))
        .map(|start| parts.into_iter().skip(start).collect::<Vec<_>>().join("/"))
}

fn path_to_str(path: &Path) -> Result<String, AstGrepLaneError> {
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| AstGrepLaneError::NonUtf8Path { path: path_display(path) })
}

fn finding(
    rule: RuleDef,
    site: span::MatchSite,
    source: &str,
    offsets: &[usize],
    workspace_path: &WorkspacePath,
) -> Result<Finding, AstGrepLaneError> {
    let rule_id = RuleId::new(rule.id)?;
    let (start_line, start_col) = span::line_col(source, offsets, site.start_byte);
    let (end_line, end_col) = span::line_col(source, offsets, site.end_byte);
    let line_start = checked_line_number(start_line)?;
    let line_end = checked_line_number(end_line)?;
    let col_start = checked_column(start_col)?;
    let col_end = checked_column(end_col)?;
    let location =
        Location::span(workspace_path.clone(), line_start, col_start, line_end, col_end)?;
    let repair = repair_hint(rule.repair);
    Ok(match rule.effect {
        FindingEffect::Reject => {
            Finding::reject(Lane::AstGrep, rule_id, location, rule.message.to_owned(), repair)
        }
        FindingEffect::Informational => Finding::informational(
            Lane::AstGrep,
            rule_id,
            location,
            rule.message.to_owned(),
            repair,
        ),
    })
}

fn detect_sites(rule: RuleDef, engine: &engine::AstEngine, source: &str) -> Vec<span::MatchSite> {
    rule.detect.run(engine, source)
}

fn checked_line_number(index: usize) -> Result<u32, AstGrepLaneError> {
    let Ok(line) = u32::try_from(index) else {
        return Err(AstGrepLaneError::LineNumberOverflow { index });
    };
    line.checked_add(1).ok_or(AstGrepLaneError::LineNumberOverflow { index })
}

fn checked_column(column: usize) -> Result<u32, AstGrepLaneError> {
    let Ok(column) = u32::try_from(column) else {
        return Err(AstGrepLaneError::ColumnOverflow { column });
    };
    Ok(column)
}

fn suppress_exceptions(findings: Vec<Finding>, exceptions: &[(RuleId, String)]) -> Vec<Finding> {
    findings.into_iter().filter(|finding| !is_suppressed(finding, exceptions)).collect()
}

fn is_suppressed(finding: &Finding, exceptions: &[(RuleId, String)]) -> bool {
    finding.location().span_file().is_some_and(|path| exception_matches(finding, path, exceptions))
}

fn exception_matches(
    finding: &Finding,
    path: &WorkspacePath,
    exceptions: &[(RuleId, String)],
) -> bool {
    exceptions.iter().any(|(rule_id, exception_path)| {
        rule_id == finding.rule_id() && exception_path == path.as_str()
    })
}

fn clean_outcome() -> Result<LaneOutcome, AstGrepLaneError> {
    let command = CommandEvidence::embedded(
        "titania-check".to_owned(),
        Box::from(["titania-check".to_owned(), "run-lane".to_owned(), "ast-grep".to_owned()]),
    )?;
    let evidence = LaneEvidence::new(
        command,
        "embedded-rules-v1".to_owned(),
        ProcessTermination::Exited { code: 0 },
        Digest::from_bytes(b"ast-grep-clean"),
    )?;
    Ok(LaneOutcome::Clean { evidence })
}

fn catalog_text(rules_yaml: &[&'static str]) -> Result<String, AstGrepLaneError> {
    let catalog = rules_yaml.join("\n");
    let unknown = catalog_ids(&catalog)
        .into_iter()
        .find(|id| !RULES.iter().any(|rule| rule.id == *id))
        .map(str::to_owned);
    unknown.map_or_else(|| Ok(catalog), |id| Err(AstGrepLaneError::UnknownRuleId { id }))
}

fn catalog_ids(catalog: &str) -> Vec<&str> {
    catalog.lines().filter_map(top_level_id).collect()
}

fn top_level_id(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("id:")?.trim();
    if let Some(quoted) = rest.strip_prefix('"') {
        return quoted.split('"').next();
    }
    if let Some(quoted) = rest.strip_prefix('\'') {
        return quoted.split('\'').next();
    }
    Some(rest.split('#').next().map_or(rest, str::trim))
}

fn rule_enabled(catalog: &str, rule_id: &str) -> bool {
    catalog_ids(catalog).into_iter().any(|id| id == rule_id)
}

fn path_display(path: &Path) -> String {
    path.display().to_string()
}
