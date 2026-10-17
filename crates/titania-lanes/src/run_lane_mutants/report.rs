use std::path::{Path, PathBuf};

use titania_core::{
    MutantRecord, MutantsOutcomes, MutantsRecords, OutcomeScenario, OutcomeSummary,
};

use super::{constants::MUTANTS_OUTPUT_DIR, error::MutantsLaneError, state::MutantsReport};

pub(super) fn read_workspace_report(
    workspace_root: &Path,
    exit_code: i32,
) -> Result<MutantsReport, MutantsLaneError> {
    let output_dir = workspace_root.join(MUTANTS_OUTPUT_DIR);
    let artifact_dir = find_mutants_artifact_dir(&output_dir)?;
    let outcomes_path = artifact_dir.join("outcomes.json");
    let mutants_path = artifact_dir.join("mutants.json");
    let outcomes_label = outcomes_path.display().to_string();
    let mutants_label = mutants_path.display().to_string();
    let outcomes_contents = std::fs::read_to_string(&outcomes_path).map_err(|error| {
        MutantsLaneError::OutcomesParse {
            path: Box::from(outcomes_label.as_str()),
            reason: Box::from(format!("read failed: {error}").as_str()),
        }
    })?;
    let mutants_contents =
        std::fs::read_to_string(&mutants_path).map_err(|error| MutantsLaneError::MutantsParse {
            path: Box::from(mutants_label.as_str()),
            reason: Box::from(format!("read failed: {error}").as_str()),
        })?;
    let survivor_names = parse_survivor_names(&outcomes_contents, &outcomes_label)?;
    drop(parse_mutants_records(&mutants_contents, &mutants_label)?);
    Ok(MutantsReport { survivor_names, exit_code })
}

pub(super) fn find_mutants_artifact_dir(output_dir: &Path) -> Result<PathBuf, MutantsLaneError> {
    let direct_outcomes = output_dir.join("outcomes.json");
    if direct_outcomes.is_file() {
        return Ok(output_dir.to_owned());
    }
    let nested_dir = output_dir.join(MUTANTS_OUTPUT_DIR);
    if nested_dir.join("outcomes.json").is_file() {
        return Ok(nested_dir);
    }
    Err(MutantsLaneError::ArtifactDir(format!(
        "neither {} nor {} contain an outcomes.json",
        direct_outcomes.display(),
        nested_dir.join("outcomes.json").display()
    )))
}

pub(super) fn parse_survivor_names(
    outcomes_json: &str,
    label: &str,
) -> Result<Vec<String>, MutantsLaneError> {
    let outcomes = MutantsOutcomes::parse_str(outcomes_json, label).map_err(|error| {
        MutantsLaneError::OutcomesParse {
            path: Box::from(label),
            reason: Box::from(error.to_string().as_str()),
        }
    })?;
    let survivor_names: Vec<String> = outcomes
        .outcomes
        .iter()
        .filter(|entry| matches!(entry.summary, OutcomeSummary::MissedMutant))
        .filter_map(survivor_name_from_entry)
        .collect();
    validate_reported_missed_count(&outcomes, survivor_names.len(), label)?;
    Ok(survivor_names)
}

pub(super) fn survivor_name_from_entry(entry: &titania_core::MutantOutcomeEntry) -> Option<String> {
    match &entry.scenario {
        OutcomeScenario::Mutant { mutant } => Some(mutant.name.clone()),
        OutcomeScenario::Baseline(_) => None,
    }
}

fn validate_reported_missed_count(
    outcomes: &MutantsOutcomes,
    actual: usize,
    label: &str,
) -> Result<(), MutantsLaneError> {
    let Some(reported) = outcomes.missed else {
        return Ok(());
    };
    let reported = usize::try_from(reported).map_err(|error| {
        MutantsLaneError::MissedCountMismatch(format!(
            "reported missed-mutant count exceeds platform limits: {error}"
        ))
    })?;
    if reported != actual {
        return Err(MutantsLaneError::MissedCountMismatch(format!(
            "{label} reports {reported} missed mutants but carries {actual}"
        )));
    }
    Ok(())
}

pub(super) fn parse_mutants_records(
    mutants_json: &str,
    label: &str,
) -> Result<Vec<MutantRecord>, MutantsLaneError> {
    let records = MutantsRecords::parse_str(mutants_json, label).map_err(|error| {
        MutantsLaneError::MutantsParse {
            path: Box::from(label),
            reason: Box::from(error.to_string().as_str()),
        }
    })?;
    Ok(records.into_inner())
}

pub(super) fn validate_report_exit(report: &MutantsReport) -> Result<(), MutantsLaneError> {
    if report.exit_code == 0 {
        return Ok(());
    }
    if report.exit_code == 2 && !report.survivor_names.is_empty() {
        return Ok(());
    }
    Err(MutantsLaneError::CargoMutantsExit(report.exit_code))
}

#[cfg(test)]
mod tests {
    use titania_core::{MutantOutcomeEntry, MutantScenarioData, OutcomeScenario, OutcomeSummary};

    use super::{parse_survivor_names, survivor_name_from_entry, validate_reported_missed_count};

    fn entry_with_scenario(name: &str, summary: OutcomeSummary) -> MutantOutcomeEntry {
        MutantOutcomeEntry {
            scenario: OutcomeScenario::Mutant {
                mutant: MutantScenarioData {
                    name: name.to_owned(),
                    package: String::from("titania-core"),
                    file: String::from("src/lib.rs"),
                    span: None,
                    genre: None,
                    replacement: None,
                },
            },
            summary,
            log_path: None,
            diff_path: None,
        }
    }

    fn payload_with_missed(missed: Option<u64>) -> String {
        let payload = serde_json::json!({
            "outcomes": [{
                "scenario": {"Mutant": {
                    "name": "src/lib.rs:1:5: replace foo",
                    "package": "titania-core",
                    "file": "src/lib.rs"
                }},
                "summary": "MissedMutant"
            }],
            "missed": missed
        });
        serde_json::to_string(&payload)
            .unwrap_or_else(|error| panic!("payload must serialise: {error}"))
    }

    #[test]
    fn survivor_name_picks_mutant_name_under_scenario_object() {
        let entry =
            entry_with_scenario("src/lib.rs:1:5: replace foo", OutcomeSummary::MissedMutant);
        let name = survivor_name_from_entry(&entry);
        assert_eq!(name.as_deref(), Some("src/lib.rs:1:5: replace foo"));
    }

    #[test]
    fn survivor_name_returns_none_for_baseline_payload() {
        let entry = MutantOutcomeEntry {
            scenario: OutcomeScenario::Baseline(String::from("Baseline")),
            summary: OutcomeSummary::Success,
            log_path: None,
            diff_path: None,
        };
        assert_eq!(survivor_name_from_entry(&entry), None);
    }

    #[test]
    fn parse_survivor_names_returns_empty_for_clean_run() {
        let payload = serde_json::json!({
            "outcomes": [{
                "scenario": {"Mutant": {
                    "name": "src/lib.rs:1:5: replace foo",
                    "package": "titania-core",
                    "file": "src/lib.rs"
                }},
                "summary": "Success"
            }],
            "missed": 0
        });
        let payload_text = serde_json::to_string(&payload)
            .unwrap_or_else(|error| panic!("payload must serialise: {error}"));
        let names = parse_survivor_names(&payload_text, "fake/outcomes.json")
            .unwrap_or_else(|error| panic!("parse must succeed: {error}"));
        assert!(names.is_empty(), "clean run produces no survivors");
    }

    #[test]
    fn validate_reported_missed_count_accepts_matching_aggregate() {
        let outcomes = titania_core::MutantsOutcomes::parse_str(
            &payload_with_missed(Some(1)),
            "fake/outcomes.json",
        )
        .unwrap_or_else(|error| panic!("parse must succeed: {error}"));
        validate_reported_missed_count(&outcomes, 1, "fake/outcomes.json")
            .unwrap_or_else(|error| panic!("matching aggregate must validate: {error}"));
    }

    #[test]
    fn validate_reported_missed_count_rejects_mismatch() {
        let outcomes = titania_core::MutantsOutcomes::parse_str(
            &payload_with_missed(Some(2)),
            "fake/outcomes.json",
        )
        .unwrap_or_else(|error| panic!("parse must succeed: {error}"));
        let result = validate_reported_missed_count(&outcomes, 1, "fake/outcomes.json");
        assert!(result.is_err(), "aggregate mismatch must surface typed error");
    }
}
