#![forbid(unsafe_code)]

pub mod assure;
pub mod contracts;
pub mod evidence;
pub mod evidence_gate;

mod command_family;
mod dependency_boundary;
mod error;
mod parser;
mod registry;
mod routing;
mod status;

pub use command_family::CommandFamily;
pub use dependency_boundary::{WorkspaceManifest, assert_runtime_dependency_boundary};
pub use error::XtaskCommandError;
pub use parser::{XtaskCommand, parse_xtask_command};
pub use registry::validate_command_registry;
pub use registry::{CommandFamilySpec, ValidatedCommandRegistry, required_command_families};
pub use routing::{XtaskEnvironment, placeholder_status, route_command};
pub use status::{DeferredReason, OutputFormat, StructuredStatus, render_structured_status};

// New modules for proof/test orchestrator (vb-i7xn)
pub mod discovery;
pub mod lanes;
pub mod logger;
pub mod profiles;
pub mod proof_orchestrator;
pub mod scheduler;
pub mod summary;
