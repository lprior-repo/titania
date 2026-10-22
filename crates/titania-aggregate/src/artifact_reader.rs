use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

type ArtifactVisitor<'a> = dyn FnMut(&Path) -> Result<(), ReaderError> + 'a;

use thiserror::Error;
use titania_core::{Finding, GateScope, Lane, LaneArtifact, LaneFailure, LaneOutcome, SkipReason};

#[derive(Debug, Error)]
pub enum ReaderError {
    #[error("infra failure for tool {tool}: {reason}")]
    InfraFailure { tool: String, reason: String },
    #[error("input error for lane {lane}: {cause}")]
    InputError { lane: Lane, cause: String },
    #[error("unsupported gate scope {scope}")]
    UnsupportedScope { scope: String },
}

pub type ReaderResult = Result<Vec<(Lane, LaneOutcome)>, ReaderError>;

type LaneRecord = (Lane, LaneOutcome);
type PresentLaneRecord = (Lane, Option<LaneOutcome>);

pub fn read_lane_artifacts(target_root: &Path, scope: GateScope) -> ReaderResult {
    let scope_dir: &'static str = scope.name();
    let out_dir = artifact_dir(target_root, scope_dir);
    let expected: &[Lane] = scope.lanes();
    let scoped_lanes = scoped_lane_set(scope);

    let mut by_lane: HashMap<Lane, String> = HashMap::new();
    enumerate_artifact_files(&out_dir, scope_dir, &mut |entry: &Path| {
        classify_and_record(entry, scope_dir, expected, &scoped_lanes, &mut by_lane)
    })?;

    let parse = |contents: String, lane: Lane| parse_artifact(&contents, lane);
    let present = expected
        .iter()
        .map(|lane| {
            by_lane.remove(lane).map_or_else(
                || Ok((*lane, None)),
                |contents| parse(contents, *lane).map(|(lane, outcome)| (lane, Some(outcome))),
            )
        })
        .collect::<Result<Vec<_>, ReaderError>>()?;
    Ok(apply_compile_skip_rule(present))
}

fn apply_compile_skip_rule(present: Vec<PresentLaneRecord>) -> Vec<LaneRecord> {
    let compile_failed = present
        .iter()
        .find(|(lane, _)| *lane == Lane::Compile)
        .and_then(|(_, outcome)| outcome.as_ref())
        .is_some_and(compilation_failed);
    present
        .into_iter()
        .map(|(lane, outcome)| resolve_outcome(lane, outcome, compile_failed))
        .collect()
}

fn resolve_outcome(lane: Lane, outcome: Option<LaneOutcome>, compile_failed: bool) -> LaneRecord {
    if compile_failed && lane.depends_on_compile() && !records_findings(outcome.as_ref()) {
        return (lane, LaneOutcome::Skipped { reason: SkipReason::PriorCompilationFailure });
    }
    if let Some(present) = outcome {
        return (lane, present);
    }
    (lane, missing_lane_outcome(lane))
}

const fn records_findings(outcome: Option<&LaneOutcome>) -> bool {
    matches!(outcome, Some(LaneOutcome::Findings { .. }))
}

fn compilation_failed(outcome: &LaneOutcome) -> bool {
    match outcome {
        LaneOutcome::Failed { .. } => true,
        LaneOutcome::Findings { findings } => findings.iter().any(Finding::is_reject),
        LaneOutcome::Clean { .. } | LaneOutcome::Skipped { .. } => false,
    }
}

fn classify_and_record(
    entry: &Path,
    scope_dir: &'static str,
    expected: &[Lane],
    scoped_lanes: &HashSet<Lane>,
    by_lane: &mut HashMap<Lane, String>,
) -> Result<(), ReaderError> {
    let stem = entry_stem(entry, expected, scope_dir)?;
    let lane = stem_to_lane(stem).ok_or_else(|| ReaderError::InputError {
        lane: first_scoped_lane(expected),
        cause: format!(
            "unexpected artifact file {:?} in scope {scope_dir}: stem {stem:?} does not name a known lane",
            entry.file_name(),
        ),
    })?;
    if !scoped_lanes.contains(&lane) {
        return Err(ReaderError::InputError {
            lane,
            cause: format!(
                "unexpected artifact file {:?} for lane {lane} in scope {scope_dir} (not part of this gate)",
                entry.file_name(),
            ),
        });
    }
    let Some(contents) = read_entry(entry, lane)? else {
        return Ok(());
    };
    if by_lane.insert(lane, contents).is_some() {
        return Err(ReaderError::InputError {
            lane,
            cause: format!("duplicate artifact files for lane {lane} in scope {scope_dir}"),
        });
    }
    Ok(())
}
fn entry_stem<'a>(
    entry: &'a Path,
    expected: &[Lane],
    scope_dir: &'static str,
) -> Result<&'a str, ReaderError> {
    let invalid = || ReaderError::InputError {
        lane: first_scoped_lane(expected),
        cause: format!(
            "unexpected artifact file {:?} in scope {scope_dir}: filename is not valid UTF-8",
            entry.file_name(),
        ),
    };
    entry.file_stem().and_then(|name| name.to_str()).ok_or_else(invalid)
}

fn read_entry(entry: &Path, lane: Lane) -> Result<Option<String>, ReaderError> {
    match std::fs::read_to_string(entry) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ReaderError::InputError {
            lane,
            cause: format!("IO error reading artifact: {error}"),
        }),
    }
}

fn parse_artifact(contents: &str, lane: Lane) -> Result<(Lane, LaneOutcome), ReaderError> {
    let artifact: LaneArtifact = serde_json::from_str(contents).map_err(|err| {
        ReaderError::InputError { lane, cause: format!("malformed JSON for {lane}: {err}") }
    })?;

    let artifact_lane = artifact.lane();
    if artifact_lane != lane {
        return Err(ReaderError::InputError {
            lane,
            cause: format!("lane mismatch in artifact: expected {lane}, got {artifact_lane}"),
        });
    }

    let outcome: LaneOutcome =
        artifact.into_outcome().into_lane_outcome().map_err(|err| ReaderError::InputError {
            lane,
            cause: format!("failed to parse outcome for {lane}: {err}"),
        })?;

    Ok((artifact_lane, outcome))
}
fn enumerate_artifact_files(
    out_dir: &Path,
    scope_dir: &'static str,
    visit: &mut ArtifactVisitor<'_>,
) -> Result<(), ReaderError> {
    let entries = match std::fs::read_dir(out_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(ReaderError::InputError {
                lane: Lane::Fmt,
                cause: format!(
                    "IO error reading artifact directory {} for scope {scope_dir}: {error}",
                    out_dir.display(),
                ),
            });
        }
    };
    let mut paths: Vec<PathBuf> = entries
        .map(|entry| {
            entry.map(|e| e.path()).map_err(|error| ReaderError::InputError {
                lane: Lane::Fmt,
                cause: format!(
                    "IO error enumerating artifact directory {} for scope {scope_dir}: {error}",
                    out_dir.display(),
                ),
            })
        })
        .filter_map(|entry| match entry {
            Ok(path)
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") =>
            {
                Some(Ok(path))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<Vec<_>, ReaderError>>()?;
    paths.sort();
    paths.iter().try_fold((), |(), path| visit(path.as_path()))?;
    Ok(())
}
fn scoped_lane_set(scope: GateScope) -> HashSet<Lane> {
    scope.lanes().iter().copied().collect()
}

fn stem_to_lane(stem: &str) -> Option<Lane> {
    match stem {
        "fmt" => Some(Lane::Fmt),
        "compile" => Some(Lane::Compile),
        "clippy" => Some(Lane::Clippy),
        "ast-grep" => Some(Lane::AstGrep),
        "dylint" => Some(Lane::Dylint),
        "panic-scan" => Some(Lane::PanicScan),
        "policy-scan" => Some(Lane::PolicyScan),
        "test" => Some(Lane::Test),
        "deny" => Some(Lane::Deny),
        "build" => Some(Lane::Build),
        "kani" => Some(Lane::Kani),
        "mutants" => Some(Lane::Mutants),
        _ => None,
    }
}

const fn first_scoped_lane(expected: &[Lane]) -> Lane {
    match expected.first() {
        Some(lane) => *lane,
        None => Lane::Fmt,
    }
}

fn missing_lane_outcome(lane: Lane) -> LaneOutcome {
    LaneOutcome::Failed {
        failure: LaneFailure::Infra {
            tool: lane.name().to_owned(),
            reason: "output file missing".to_owned(),
        },
    }
}

fn artifact_dir(target_root: &Path, scope_dir: &str) -> PathBuf {
    target_root.join(".titania").join("out").join(scope_dir)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        fs,
        path::Path,
    };

    use tempfile::TempDir;
    use titania_core::{GateScope, Lane};

    use super::{ReaderError, classify_and_record, scoped_lane_set};

    #[test]
    fn classify_and_record_rejects_duplicate_lane_identity() {
        let tmp = TempDir::new().unwrap();
        let entry_path = tmp.path().join("fmt.json");
        fs::write(&entry_path, "{\"lane\":\"Fmt\",\"outcome\":{\"Skipped\":\"NotApplicable\"}}")
            .unwrap();
        let entry: &Path = entry_path.as_path();
        let scope_dir = "edit";
        let expected: &[Lane] = GateScope::Edit.lanes();
        let scoped_lanes: HashSet<Lane> = scoped_lane_set(GateScope::Edit);
        let mut by_lane: HashMap<Lane, String> = HashMap::new();
        let _existing = by_lane.insert(Lane::Fmt, String::from("already present"));

        let result = classify_and_record(entry, scope_dir, expected, &scoped_lanes, &mut by_lane);
        match result {
            Err(ReaderError::InputError { lane, cause }) => {
                assert_eq!(lane, Lane::Fmt);
                assert!(cause.contains("duplicate"), "cause was: {cause}");
            }
            other => panic!("expected duplicate InputError, got {other:?}"),
        }
    }

    #[test]
    fn classify_and_record_surfaces_non_not_found_read_errors() {
        let tmp = TempDir::new().unwrap();
        let entry_path = tmp.path().join("fmt");
        fs::create_dir(&entry_path).unwrap();
        let entry: &Path = entry_path.as_path();
        let scope_dir = "edit";
        let expected: &[Lane] = GateScope::Edit.lanes();
        let scoped_lanes: HashSet<Lane> = scoped_lane_set(GateScope::Edit);
        let mut by_lane: HashMap<Lane, String> = HashMap::new();

        let result = classify_and_record(entry, scope_dir, expected, &scoped_lanes, &mut by_lane);
        match result {
            Err(ReaderError::InputError { lane, cause }) => {
                assert_eq!(lane, Lane::Fmt);
                assert!(cause.contains("IO error reading artifact"), "cause was: {cause}");
            }
            other => panic!("expected IO InputError, got {other:?}"),
        }
    }
}
