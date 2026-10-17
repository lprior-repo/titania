#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::string_slice)]
#![deny(clippy::get_unwrap)]
#![deny(clippy::arithmetic_side_effects)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::as_conversions)]
#![forbid(unsafe_code)]

mod digest;
mod exceptions;
mod profile;

pub use digest::PolicyDigest;
pub use exceptions::{Exception, ExceptionError, parse_exceptions};
pub use profile::{PolicyArchitecture, PolicyProfile, PolicyValue, ProfileError, parse_profile};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitecturePolicy {
    pub core_dirs: Vec<String>,
    pub infra_crates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDefaults {
    pub schema_version: u32,
    pub profile_name: String,
    pub sources: Vec<String>,
    pub architecture: ArchitecturePolicy,
    pub embedded: bool,
}

impl PolicyDefaults {
    #[must_use]
    pub fn embedded() -> Self {
        Self {
            schema_version: 1,
            profile_name: String::from("strict-ai"),
            sources: vec![String::from("v1-spec.md"), String::from("AGENTS.md")],
            architecture: ArchitecturePolicy {
                core_dirs: vec![
                    String::from("src/core"),
                    String::from("src/domain"),
                    String::from("crates/*-core/src"),
                ],
                infra_crates: vec![
                    String::from("tokio"),
                    String::from("axum"),
                    String::from("sqlx"),
                    String::from("reqwest"),
                ],
            },
            embedded: true,
        }
    }

    #[must_use]
    pub const fn no_fs_access(&self) -> bool {
        self.embedded
    }

    #[must_use]
    pub fn sources(&self) -> &[String] {
        &self.sources
    }
}
