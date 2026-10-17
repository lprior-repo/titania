use std::path::{Path, PathBuf};

use titania_core::MutantsBaseline;

use super::error::MutantsLaneError;

#[must_use]
pub(super) fn baseline_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".titania").join("profiles").join("strict-ai").join("mutants.baseline.json")
}

pub(super) fn load_baseline(path: &Path) -> Result<MutantsBaseline, MutantsLaneError> {
    let label = path.display().to_string();
    if !path.exists() {
        return Err(MutantsLaneError::BaselineMissing(label));
    }
    let contents =
        std::fs::read_to_string(path).map_err(|error| MutantsLaneError::BaselineRead {
            path: Box::from(label.as_str()),
            reason: Box::from(error.to_string().as_str()),
        })?;
    MutantsBaseline::parse_str(&contents, &label).map_err(|error| map_parse_error(error, &label))
}

fn map_parse_error(
    error: titania_core::MutantsBaselineError,
    _path_label: &str,
) -> MutantsLaneError {
    let reason_text = error.to_string();
    let (path, reason) = match error {
        titania_core::MutantsBaselineError::JsonParse { path, reason } => {
            (path.into_string(), reason.into_string())
        }
        titania_core::MutantsBaselineError::UnsupportedSchemaVersion { path, .. } => {
            (path.into_string(), reason_text)
        }
        titania_core::MutantsBaselineError::InvalidAcceptedByRule {
            path,
            accepted_by_rule,
            reason,
        } => (
            path.into_string(),
            format!(
                "invalid accepted_by_rule {:?}: {}",
                accepted_by_rule.into_string(),
                reason.into_string()
            ),
        ),
        titania_core::MutantsBaselineError::InvalidReason { path, reason } => {
            (path.into_string(), format!("invalid reason {:?}", reason.into_string()))
        }
    };
    MutantsLaneError::BaselineMalformed {
        path: Box::from(path.as_str()),
        reason: Box::from(reason.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::baseline_path;

    #[test]
    fn baseline_path_resolves_to_strict_ai_profile() {
        let workspace = Path::new("/tmp/repo");
        let path = baseline_path(workspace);
        assert_eq!(path, Path::new("/tmp/repo/.titania/profiles/strict-ai/mutants.baseline.json"));
    }
}
