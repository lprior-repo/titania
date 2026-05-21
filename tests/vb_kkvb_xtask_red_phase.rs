#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::let_underscore_must_use,
    clippy::indexing_slicing,
    clippy::string_slice
)]

use std::ffi::OsString;
use std::path::PathBuf;

use xtask::{
    CommandFamily, CommandFamilySpec, DeferredReason, OutputFormat, StructuredStatus,
    WorkspaceManifest, XtaskCommand, XtaskCommandError, XtaskEnvironment,
    assert_runtime_dependency_boundary, parse_xtask_command, placeholder_status,
    render_structured_status, required_command_families, route_command, validate_command_registry,
};

const CONTRACT_NAMES: [&str; 20] = [
    "ai-context",
    "ai-plan",
    "ai-check",
    "ai-evidence",
    "invariants",
    "scans",
    "cert-check",
    "perf",
    "replay",
    "crash",
    "diff",
    "mutants",
    "loom",
    "kani",
    "fuzz",
    "prop",
    "repro",
    "test-plan",
    "review",
    "why-failed",
];

fn argv(tokens: &[&str]) -> Vec<OsString> {
    tokens.iter().map(OsString::from).collect()
}

fn deterministic_environment(unavailable_families: Vec<CommandFamily>) -> XtaskEnvironment {
    XtaskEnvironment {
        workspace_root: PathBuf::from("."),
        bead_id: Some("vb-kkvb".to_string()),
        output_format: OutputFormat::JsonLines,
        unavailable_families,
    }
}

fn required_family_from_name(name: &str) -> CommandFamily {
    match name {
        "ai-context" => CommandFamily::AiContext,
        "ai-plan" => CommandFamily::AiPlan,
        "ai-check" => CommandFamily::AiCheck,
        "ai-evidence" => CommandFamily::AiEvidence,
        "invariants" => CommandFamily::Invariants,
        "scans" => CommandFamily::Scans,
        "cert-check" => CommandFamily::CertCheck,
        "perf" => CommandFamily::Perf,
        "replay" => CommandFamily::Replay,
        "crash" => CommandFamily::Crash,
        "diff" => CommandFamily::Diff,
        "mutants" => CommandFamily::Mutants,
        "loom" => CommandFamily::Loom,
        "kani" => CommandFamily::Kani,
        "fuzz" => CommandFamily::Fuzz,
        "prop" => CommandFamily::Prop,
        "repro" => CommandFamily::Repro,
        "test-plan" => CommandFamily::TestPlan,
        "review" => CommandFamily::Review,
        _ => CommandFamily::WhyFailed,
    }
}

#[test]
fn required_registry_contains_each_contract_command_once_and_sorted() {
    let names: Vec<_> = required_command_families()
        .iter()
        .map(CommandFamilySpec::public_name)
        .collect();
    assert_eq!(names.len(), 20);
    for expected in CONTRACT_NAMES {
        assert_eq!(names.iter().filter(|name| **name == expected).count(), 1);
    }
}

#[test]
fn registry_validation_rejects_duplicate_and_schema_drift() {
    assert!((validate_command_registry(required_command_families()).is_ok()));
    assert_eq!(
        validate_command_registry(&[
            CommandFamilySpec::new(
                "ai-context",
                &["command", "status", "message", "next_steps"]
            ),
            CommandFamilySpec::new(
                "ai-context",
                &["command", "status", "message", "next_steps"]
            ),
        ]),
        Err(XtaskCommandError::InternalInvariantViolation {
            invariant: "duplicate command family: ai-context".to_string(),
        })
    );
    assert_eq!(
        validate_command_registry(&[CommandFamilySpec::new(
            "ai-context",
            &["command", "status", "message"]
        )]),
        Err(XtaskCommandError::InternalInvariantViolation {
            invariant: "structured status schema drift: missing next_steps".to_string(),
        })
    );
}

#[test]
fn parser_rejects_unknown_and_invalid_required_inputs() {
    for command in ["unknown", "AiContext", "ai--context"] {
        assert_eq!(
            parse_xtask_command(argv(&["xtask", command])),
            Err(XtaskCommandError::UnknownCommand {
                command: command.to_string()
            })
        );
    }
    assert_eq!(
        parse_xtask_command(argv(&["xtask", "ai-context", "--bead"])),
        Err(XtaskCommandError::MissingRequiredInput {
            command: "ai-context".to_string(),
            input: "bead".to_string()
        })
    );
    assert_eq!(
        parse_xtask_command(argv(&["xtask", "ai-context", "--bead", ""])),
        Err(XtaskCommandError::InvalidInput {
            command: "ai-context".to_string(),
            input: "bead".to_string(),
            reason: "bead id must not be empty".to_string()
        })
    );
    assert_eq!(
        parse_xtask_command(argv(&["xtask", "test-plan", "--format", "xml"])),
        Err(XtaskCommandError::InvalidInput {
            command: "test-plan".to_string(),
            input: "format".to_string(),
            reason: "unsupported output format: xml".to_string()
        })
    );
}

#[test]
fn placeholder_route_and_render_are_deterministic() {
    for name in ["perf", "fuzz", "ai-context"] {
        let family = required_family_from_name(name);
        let expected = StructuredStatus {
            command: name.to_string(),
            status: "deferred".to_string(),
            message: format!("{name} automation deferred: implementation is outside bead vb-kkvb"),
            next_steps: vec![format!("open follow-up bead for {name} engine integration")],
        };
        assert_eq!(
            placeholder_status(family, DeferredReason::NotImplementedInThisBead),
            Ok(expected.clone())
        );
        assert_eq!(
            route_command(
                XtaskCommand::Required(family),
                &deterministic_environment(Vec::new())
            ),
            Ok(expected)
        );
    }
    assert_eq!(
        route_command(
            XtaskCommand::Required(CommandFamily::Perf),
            &deterministic_environment(vec![CommandFamily::Perf])
        ),
        Err(XtaskCommandError::Unavailable {
            command: "perf".to_string(),
            reason: "perf automation is not implemented in bead vb-kkvb".to_string(),
        })
    );
}

#[test]
fn renderer_returns_json_or_exact_failures() {
    let status = StructuredStatus {
        command: "fuzz".to_string(),
        status: "deferred".to_string(),
        message: "fuzz automation deferred: implementation is outside bead vb-kkvb".to_string(),
        next_steps: vec!["open follow-up bead for fuzz engine integration".to_string()],
    };
    assert_eq!(
        render_structured_status(&status, OutputFormat::JsonLines)
            .unwrap_or_default()
            .contains("\"command\":\"fuzz\""),
        true
    );
    assert_eq!(
        render_structured_status(
            &StructuredStatus::with_renderer_failure_for_test(
                "fuzz",
                "deferred",
                "m",
                ["n"],
                "boom"
            ),
            OutputFormat::JsonLines
        ),
        Err(XtaskCommandError::OutputRenderFailed {
            command: "fuzz".to_string(),
            reason: "boom".to_string()
        })
    );
}

#[test]
fn dependency_boundary_rejects_runtime_shell_dependencies() {
    for (crate_name, dependency) in [("vb_core", "clap"), ("vb_runtime", "xtask")] {
        assert_eq!(
            assert_runtime_dependency_boundary(&WorkspaceManifest::from_edges([(
                crate_name, dependency
            )])),
            Err(XtaskCommandError::DependencyBoundaryViolation {
                crate_name: crate_name.to_string(),
                dependency: dependency.to_string()
            })
        );
    }
}
