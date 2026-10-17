use std::path::Path;

use titania_core::{MutantId, MutantsBaseline};

use super::{
    baseline::load_baseline,
    constants::MUTANTS_OUTPUT_DIR,
    error::MutantsLaneError,
    operators::{operator_for_raw, relative_mutant_path},
    report::find_mutants_artifact_dir,
    state::NewSurvivor,
};

pub(super) fn build_new_survivors(
    survivor_names: &[String],
    workspace_root: &Path,
    now_unix: u64,
) -> Result<Vec<NewSurvivor>, MutantsLaneError> {
    let baseline = load_baseline(&super::baseline::baseline_path(workspace_root)).ok();
    survivor_names
        .iter()
        .map(|mutation_id| {
            classify_survivor_typed(mutation_id, workspace_root, baseline.as_ref(), now_unix)
        })
        .filter_map(Result::transpose)
        .collect()
}

fn classify_survivor_typed(
    mutation_id: &str,
    workspace_root: &Path,
    baseline: Option<&MutantsBaseline>,
    now_unix: u64,
) -> Result<Option<NewSurvivor>, MutantsLaneError> {
    let output_dir = workspace_root.join(MUTANTS_OUTPUT_DIR);
    let artifact_dir = find_mutants_artifact_dir(&output_dir)?;
    let mutants_path = artifact_dir.join("mutants.json");
    let mutants_contents = read_mutants_contents(&mutants_path)?;
    let records = super::report::parse_mutants_records(
        &mutants_contents,
        &mutants_path.display().to_string(),
    )?;
    let raw = records
        .iter()
        .find(|mutant| mutant.name == *mutation_id)
        .ok_or_else(|| MutantsLaneError::SurvivorAbsent(mutation_id.to_owned()))?;
    let survivor = build_survivor_from_record(raw)?;
    let covered = baseline.is_some_and(|b| b.contains(&survivor.typed_id, now_unix));
    Ok(if covered { None } else { Some(survivor) })
}

fn read_mutants_contents(path: &Path) -> Result<String, MutantsLaneError> {
    let label = path.display().to_string();
    std::fs::read_to_string(path).map_err(|error| MutantsLaneError::MutantsParse {
        path: Box::from(label.as_str()),
        reason: Box::from(format!("read failed: {error}").as_str()),
    })
}

fn build_survivor_from_record(
    raw: &titania_core::MutantRecord,
) -> Result<NewSurvivor, MutantsLaneError> {
    let (line, column) = raw
        .start_point()
        .ok_or_else(|| MutantsLaneError::SpanMissing { name: Box::from(raw.name.as_str()) })?;
    let rel_path = relative_mutant_path(&raw.package, &raw.file).map_err(|_path_error| {
        MutantsLaneError::PathOutsidePackage {
            name: Box::from(raw.name.as_str()),
            file: Box::from(raw.file.as_str()),
            package: Box::from(raw.package.as_str()),
        }
    })?;
    let operator = operator_for_raw(
        raw.genre.as_deref().map_or("", |value| value),
        &raw.name,
        raw.replacement.as_deref().map_or("", |value| value),
    )?;
    let typed_id =
        MutantId::new(&raw.package, rel_path, line, column, operator).map_err(|error| {
            MutantsLaneError::MutantIdInvalid {
                name: Box::from(raw.name.as_str()),
                reason: Box::from(error.to_string().as_str()),
            }
        })?;
    let genre: String = raw.genre.as_deref().map_or_else(String::new, str::to_owned);
    let replacement: String = raw.replacement.as_deref().map_or_else(String::new, str::to_owned);
    Ok(NewSurvivor {
        package: raw.package.clone(),
        rel_path: rel_path.to_owned(),
        line,
        column,
        genre,
        replacement,
        raw_name: raw.name.clone(),
        typed_id,
    })
}

#[cfg(test)]
mod tests {
    use super::{super::current_unix, build_new_survivors};

    #[test]
    fn build_new_survivors_skips_when_no_survivor_names() {
        let workspace = tempfile::tempdir().expect("tempdir").keep();
        let survivor_names: Vec<String> = Vec::new();
        let now_unix = current_unix();
        let survivors = build_new_survivors(&survivor_names, &workspace, now_unix);
        assert!(survivors.is_ok(), "empty survivor list must not error");
        assert!(
            survivors.is_ok_and(|v| v.is_empty()),
            "empty survivor list must produce empty vec"
        );
    }

    #[test]
    fn classify_survivor_returns_err_when_artifacts_missing() {
        let workspace = tempfile::tempdir().expect("tempdir").keep();
        let result = super::classify_survivor_typed(
            "src/lib.rs:1:5: replace foo",
            &workspace,
            None,
            current_unix(),
        );
        assert!(
            result.is_err(),
            "missing artifact must surface typed infra failure, never silently coerce"
        );
    }
}
