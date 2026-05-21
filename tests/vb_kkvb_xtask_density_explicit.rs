#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::ffi::OsString;
use std::path::PathBuf;

use xtask::{
    CommandFamily, DeferredReason, OutputFormat, StructuredStatus, WorkspaceManifest, XtaskCommand,
    XtaskCommandError, XtaskEnvironment, assert_runtime_dependency_boundary, parse_xtask_command,
    placeholder_status, render_structured_status, route_command,
};

const FAMILIES: [(&str, CommandFamily); 20] = [
    ("ai-context", CommandFamily::AiContext),
    ("ai-plan", CommandFamily::AiPlan),
    ("ai-check", CommandFamily::AiCheck),
    ("ai-evidence", CommandFamily::AiEvidence),
    ("invariants", CommandFamily::Invariants),
    ("scans", CommandFamily::Scans),
    ("cert-check", CommandFamily::CertCheck),
    ("perf", CommandFamily::Perf),
    ("replay", CommandFamily::Replay),
    ("crash", CommandFamily::Crash),
    ("diff", CommandFamily::Diff),
    ("mutants", CommandFamily::Mutants),
    ("loom", CommandFamily::Loom),
    ("kani", CommandFamily::Kani),
    ("fuzz", CommandFamily::Fuzz),
    ("prop", CommandFamily::Prop),
    ("repro", CommandFamily::Repro),
    ("test-plan", CommandFamily::TestPlan),
    ("review", CommandFamily::Review),
    ("why-failed", CommandFamily::WhyFailed),
];

fn argv(tokens: &[&str]) -> Vec<OsString> {
    tokens.iter().map(OsString::from).collect()
}

fn env_available() -> XtaskEnvironment {
    XtaskEnvironment {
        workspace_root: PathBuf::from("."),
        bead_id: Some("vb-kkvb".to_string()),
        output_format: OutputFormat::JsonLines,
        unavailable_families: Vec::new(),
    }
}

fn env_with_disabled(family: CommandFamily) -> XtaskEnvironment {
    XtaskEnvironment {
        unavailable_families: vec![family],
        ..env_available()
    }
}

#[test]
fn all_command_families_have_exact_public_names_and_parse_forms() {
    for (name, family) in FAMILIES {
        assert_eq!(family.public_name(), name);
        if name == "loom" {
            // loom is classified as Legacy, not Required
            assert_eq!(
                parse_xtask_command(argv(&["xtask", name])),
                Ok(XtaskCommand::Legacy(name))
            );
        } else {
            assert_eq!(
                parse_xtask_command(argv(&["xtask", name])),
                Ok(XtaskCommand::Required(family))
            );
            assert_eq!(
                parse_xtask_command(argv(&["xtask", name, "--bead", "vb-kkvb"])),
                Ok(XtaskCommand::Required(family))
            );
            assert_eq!(
                parse_xtask_command(argv(&["xtask", name, "--format", "jsonl"])),
                Ok(XtaskCommand::Required(family))
            );
        }
    }
}

#[test]
fn all_command_families_reject_invalid_required_options() {
    for (name, _) in FAMILIES {
        if name == "loom" {
            // loom is Legacy, not Required - it doesn't validate --bead/--format options
            continue;
        }
        assert_eq!(
            parse_xtask_command(argv(&["xtask", name, "--bead"])),
            Err(XtaskCommandError::MissingRequiredInput {
                command: name.to_string(),
                input: "bead".to_string(),
            })
        );
        assert_eq!(
            parse_xtask_command(argv(&["xtask", name, "--bead", ""])),
            Err(XtaskCommandError::InvalidInput {
                command: name.to_string(),
                input: "bead".to_string(),
                reason: "bead id must not be empty".to_string(),
            })
        );
        assert_eq!(
            parse_xtask_command(argv(&["xtask", name, "--format"])),
            Err(XtaskCommandError::MissingRequiredInput {
                command: name.to_string(),
                input: "format".to_string(),
            })
        );
        assert_eq!(
            parse_xtask_command(argv(&["xtask", name, "--format", "xml"])),
            Err(XtaskCommandError::InvalidInput {
                command: name.to_string(),
                input: "format".to_string(),
                reason: "unsupported output format: xml".to_string(),
            })
        );
    }
}

#[test]
fn all_command_families_route_and_render_deferred_json() {
    for (name, family) in FAMILIES {
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
            route_command(XtaskCommand::Required(family), &env_available()),
            Ok(expected.clone())
        );
        assert_eq!(
            route_command(XtaskCommand::Required(family), &env_with_disabled(family)),
            Err(XtaskCommandError::Unavailable {
                command: name.to_string(),
                reason: format!("{name} automation is not implemented in bead vb-kkvb"),
            })
        );
        assert_eq!(
            render_structured_status(&expected, OutputFormat::JsonLines),
            Ok(format!(
                "{{\"command\":\"{name}\",\"status\":\"deferred\",\"message\":\"{name} automation deferred: implementation is outside bead vb-kkvb\",\"next_steps\":[\"open follow-up bead for {name} engine integration\"]}}\n"
            ))
        );
    }
}

#[test]
fn top_level_and_legacy_commands_classify_exactly() {
    assert_eq!(
        parse_xtask_command(argv(&["xtask", "--help"])),
        Ok(XtaskCommand::Help)
    );
    assert_eq!(
        parse_xtask_command(argv(&["xtask", "-h"])),
        Ok(XtaskCommand::Help)
    );
    assert_eq!(
        parse_xtask_command(argv(&["xtask", "--version"])),
        Ok(XtaskCommand::Version)
    );
    assert_eq!(
        parse_xtask_command(argv(&["xtask", "-V"])),
        Ok(XtaskCommand::Version)
    );
    for name in [
        "ui-snapshot",
        "ui-tokens",
        "ui-overlap-check",
        "ai-fast",
        "ai-deep",
        "ai-release",
    ] {
        assert_eq!(
            parse_xtask_command(argv(&["xtask", name])),
            Ok(XtaskCommand::Legacy(name))
        );
    }
}

#[test]
fn parser_rejects_malformed_commands_without_normalization_shortcuts() {
    for command in [
        "", " ", "AI-plan", "-ai-plan", "ai-plan-", "ai--plan", "ai_plan", "åi-plan",
    ] {
        assert_eq!(
            parse_xtask_command(argv(&["xtask", command])),
            Err(XtaskCommandError::UnknownCommand {
                command: command.to_string()
            })
        );
    }
    assert_eq!(
        parse_xtask_command(argv(&["xtask"])),
        Err(XtaskCommandError::MissingRequiredInput {
            command: "xtask".to_string(),
            input: "command".to_string(),
        })
    );
}

#[test]
fn renderer_preserves_json_escaping_and_rejects_incomplete_status() {
    let quoted = StructuredStatus {
        command: "ai-context\"quoted".to_string(),
        status: "deferred".to_string(),
        message: "line\nnext".to_string(),
        next_steps: vec!["next".to_string()],
    };
    let rendered = render_structured_status(&quoted, OutputFormat::JsonLines).unwrap_or_default();
    assert!(rendered.contains("ai-context\\\"quoted"));
    assert!(rendered.contains("line\\nnext"));
    assert_eq!(
        render_structured_status(
            &StructuredStatus {
                command: "ai-context".into(),
                status: "deferred".into(),
                message: String::new(),
                next_steps: vec!["next".into()]
            },
            OutputFormat::JsonLines
        ),
        Err(XtaskCommandError::OutputRenderFailed {
            command: "ai-context".to_string(),
            reason: "structured status fields must be non-empty".to_string(),
        })
    );
}

#[test]
fn runtime_dependency_boundary_accepts_and_rejects_declared_edges() {
    for (crate_name, dep) in [
        ("vb_core", "xtask"),
        ("vb_storage", "toml"),
        ("vb_ipc", "reqwest"),
        ("vb_runtime", "serde_yaml"),
    ] {
        assert_eq!(
            assert_runtime_dependency_boundary(&WorkspaceManifest::from_edges([(crate_name, dep)])),
            Err(XtaskCommandError::DependencyBoundaryViolation {
                crate_name: crate_name.to_string(),
                dependency: dep.to_string(),
            })
        );
    }
    for (crate_name, dep) in [
        ("vb_ui", "clap"),
        ("vb_codegen", "toml"),
        ("vb_runtime", "fjall"),
        ("vb_storage", "bytes"),
    ] {
        assert_eq!(
            assert_runtime_dependency_boundary(&WorkspaceManifest::from_edges([(crate_name, dep)])),
            Ok(())
        );
    }
}
