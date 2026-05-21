use crate::error::XtaskCommandError;
use crate::status::STATUS_FIELDS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandFamilySpec {
    public_name: &'static str,
    status_fields: &'static [&'static str],
}

impl CommandFamilySpec {
    #[must_use]
    pub const fn new(public_name: &'static str, status_fields: &'static [&'static str]) -> Self {
        Self {
            public_name,
            status_fields,
        }
    }

    #[must_use]
    pub const fn public_name(&self) -> &'static str {
        self.public_name
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedCommandRegistry {
    sorted_names: Vec<String>,
}

impl ValidatedCommandRegistry {
    #[must_use]
    pub fn from_sorted_names<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            sorted_names: names.into_iter().map(Into::into).collect(),
        }
    }
}

static REQUIRED_COMMAND_FAMILIES: [CommandFamilySpec; 20] = [
    CommandFamilySpec::new("ai-context", &STATUS_FIELDS),
    CommandFamilySpec::new("ai-plan", &STATUS_FIELDS),
    CommandFamilySpec::new("ai-check", &STATUS_FIELDS),
    CommandFamilySpec::new("ai-evidence", &STATUS_FIELDS),
    CommandFamilySpec::new("invariants", &STATUS_FIELDS),
    CommandFamilySpec::new("scans", &STATUS_FIELDS),
    CommandFamilySpec::new("cert-check", &STATUS_FIELDS),
    CommandFamilySpec::new("perf", &STATUS_FIELDS),
    CommandFamilySpec::new("replay", &STATUS_FIELDS),
    CommandFamilySpec::new("crash", &STATUS_FIELDS),
    CommandFamilySpec::new("diff", &STATUS_FIELDS),
    CommandFamilySpec::new("mutants", &STATUS_FIELDS),
    CommandFamilySpec::new("loom", &STATUS_FIELDS),
    CommandFamilySpec::new("kani", &STATUS_FIELDS),
    CommandFamilySpec::new("fuzz", &STATUS_FIELDS),
    CommandFamilySpec::new("prop", &STATUS_FIELDS),
    CommandFamilySpec::new("repro", &STATUS_FIELDS),
    CommandFamilySpec::new("test-plan", &STATUS_FIELDS),
    CommandFamilySpec::new("review", &STATUS_FIELDS),
    CommandFamilySpec::new("why-failed", &STATUS_FIELDS),
];

#[must_use]
pub fn required_command_families() -> &'static [CommandFamilySpec] {
    &REQUIRED_COMMAND_FAMILIES
}

pub fn validate_command_registry(
    specs: &[CommandFamilySpec],
) -> Result<ValidatedCommandRegistry, XtaskCommandError> {
    let mut names = Vec::new();
    for spec in specs {
        validate_status_schema(spec.status_fields)?;
        if !is_kebab_case(spec.public_name) {
            return Err(XtaskCommandError::InternalInvariantViolation {
                invariant: format!("invalid command family name: {}", spec.public_name),
            });
        }
        if names.contains(&spec.public_name) {
            return Err(XtaskCommandError::InternalInvariantViolation {
                invariant: format!("duplicate command family: {}", spec.public_name),
            });
        }
        names.push(spec.public_name);
    }
    names.sort_unstable();
    Ok(ValidatedCommandRegistry::from_sorted_names(names))
}

fn validate_status_schema(fields: &[&str]) -> Result<(), XtaskCommandError> {
    for required in STATUS_FIELDS {
        if !fields.iter().any(|field| field == &required) {
            return Err(XtaskCommandError::InternalInvariantViolation {
                invariant: format!("structured status schema drift: missing {required}"),
            });
        }
    }
    Ok(())
}

fn is_kebab_case(value: &str) -> bool {
    let mut previous_dash = false;
    let mut saw_char = false;
    for ch in value.chars() {
        if ch == '-' {
            if previous_dash || !saw_char {
                return false;
            }
            previous_dash = true;
        } else if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            saw_char = true;
            previous_dash = false;
        } else {
            return false;
        }
    }
    saw_char && !previous_dash
}
