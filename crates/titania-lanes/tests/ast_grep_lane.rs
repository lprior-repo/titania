use std::{
    error::Error,
    path::{Path, PathBuf},
};

use titania_core::{LaneOutcome, RuleId, WorkspacePath};

type TestResult = Result<(), Box<dyn Error>>;

fn fixture_root(dir: &str, name: &str) -> PathBuf {
    let base = env!("CARGO_MANIFEST_DIR");
    Path::new(base).join("tests").join("fixtures").join(dir).join(name)
}

#[test]
fn ast_grep_lane_multi_family_fixture_emits_findings_with_exact_rule_ids() -> TestResult {
    let rules_yaml = [
        include_str!("../rules/functional.yml"),
        include_str!("../rules/bypass.yml"),
        include_str!("../rules/architecture.yml"),
    ];

    let fixture_paths: Vec<PathBuf> = [
        "functional/func_nesting_depth_violation.rs",
        "bypass/bypass_allow_attr_violation.rs",
        "crates/example-core/src/architecture_import_core_infra_violation.rs",
    ]
    .map(|p| fixture_root("ast_grep", p))
    .to_vec();

    let result = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[]);

    match result {
        Ok(outcome) => {
            assert!(
                matches!(outcome, LaneOutcome::Findings { .. }),
                "expected LaneOutcome::Findings, got {:?}",
                outcome
            );
            if let LaneOutcome::Findings { findings } = outcome {
                assert!(findings.len() >= 3, "expected >= 3 findings, got {}", findings.len());

                let prefixes: std::collections::BTreeSet<&str> =
                    findings.iter().map(|f| f.rule_id().prefix()).collect();
                assert!(prefixes.contains("FUNC"), "must contain FUNC prefix rule",);
                assert!(prefixes.contains("BYPASS"), "must contain BYPASS prefix rule",);
                assert!(prefixes.contains("ARCHITECTURE"), "must contain ARCHITECTURE prefix rule",);
            }
        }
        Err(e) => panic!("run() returned error: {}", e),
    }

    Ok(())
}

#[test]
fn ast_grep_lane_exception_suppresses_only_matching_rule_path_pair() -> TestResult {
    let rules_yaml = [
        include_str!("../rules/functional.yml"),
        include_str!("../rules/bypass.yml"),
        include_str!("../rules/architecture.yml"),
    ];

    let fixture_paths: Vec<PathBuf> = [
        "functional/func_nesting_depth_violation.rs",
        "functional/func_print_stdout_violation.rs",
        "bypass/bypass_allow_attr_violation.rs",
    ]
    .map(|p| fixture_root("ast_grep", p))
    .to_vec();

    let exceptions: [(RuleId, String); 1] = [(
        RuleId::new("FUNC_PRINT_STDOUT").unwrap(),
        "functional/func_print_stdout_violation.rs".to_owned(),
    )];

    let result = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &exceptions);

    match result {
        Ok(outcome) => {
            assert!(
                matches!(outcome, LaneOutcome::Findings { .. }),
                "expected LaneOutcome::Findings, got {:?}",
                outcome
            );
            if let LaneOutcome::Findings { findings } = outcome {
                let suppressed_count = findings
                    .iter()
                    .filter(|f| {
                        f.rule_id().as_str() == "FUNC_PRINT_STDOUT"
                            && f.location().span_file().map(WorkspacePath::as_str)
                                == Some("functional/func_print_stdout_violation.rs")
                    })
                    .count();
                assert_eq!(suppressed_count, 0, "suppressed finding should not be present",);

                let nesting_present =
                    findings.iter().any(|f| f.rule_id().as_str() == "FUNC_NESTING_DEPTH");
                let bypass_present =
                    findings.iter().any(|f| f.rule_id().as_str() == "BYPASS_ALLOW_ATTR");
                assert!(nesting_present, "FUNC_NESTING_DEPTH finding should be present");
                assert!(bypass_present, "BYPASS_ALLOW_ATTR finding should be present");
            }
        }
        Err(e) => panic!("run() returned error: {}", e),
    }

    Ok(())
}

#[test]
fn ast_grep_lane_runner_uses_embedded_rules_no_external_binary() -> TestResult {
    let rules_yaml = [
        include_str!("../rules/functional.yml"),
        include_str!("../rules/bypass.yml"),
        include_str!("../rules/architecture.yml"),
    ];

    for yml in &rules_yaml {
        assert!(yml.contains("id:"), "YAML must contain rule id");
        assert!(yml.contains("language: Rust"), "YAML must declare language Rust");
        assert!(yml.contains("pattern:"), "YAML must contain pattern");
    }

    let result = titania_lanes::ast_grep_lane::run(&rules_yaml, &[], &[]);

    match result {
        Ok(outcome) => match outcome {
            LaneOutcome::Clean { .. } => {}
            LaneOutcome::Findings { findings } => {
                assert!(findings.is_empty(), "no fixtures => no findings");
            }
            other => panic!("unexpected outcome with empty fixtures: {:?}", other),
        },
        Err(e) => panic!("run() returned error with empty fixtures: {}", e),
    }

    Ok(())
}

#[test]
fn ast_grep_lane_artifact_lane_field_is_astgrep() -> TestResult {
    use titania_core::Lane;

    assert_eq!(Lane::AstGrep.name(), "AstGrep");

    let json = serde_json::to_string(&Lane::AstGrep).expect("Lane::AstGrep must serialize to JSON");
    assert_eq!(json, "\"AstGrep\"");

    Ok(())
}

#[test]
fn ast_grep_lane_findings_outcome_serializes_with_variant_and_findings() -> TestResult {
    use titania_core::{Finding, Lane, Location, RepairHint, RuleId, WorkspacePath};

    let finding_1 = Finding::reject(
        Lane::AstGrep,
        RuleId::new("FUNC_PRINT_STDOUT").unwrap(),
        Location::span(WorkspacePath::new("src/main.rs").unwrap(), 12, 5, 12, 25).unwrap(),
        "Found `println!` in production source".into(),
        RepairHint::requires_human_review("Replace with tracing".into()),
    );

    let finding_2 = Finding::reject(
        Lane::AstGrep,
        RuleId::new("BYPASS_ALLOW_ATTR").unwrap(),
        Location::span(WorkspacePath::new("src/lib.rs").unwrap(), 1, 1, 1, 30).unwrap(),
        "Found `#[allow(...)]` suppression".into(),
        RepairHint::remove_allow_attribute("allow(clippy::unwrap_used)".into()),
    );

    let outcome = LaneOutcome::Findings { findings: Box::new([finding_1, finding_2]) };

    assert!(outcome.is_findings());
    assert!(!outcome.is_pass());

    let findings_count = match &outcome {
        LaneOutcome::Findings { findings } => findings.len(),
        _ => panic!("expected Findings variant"),
    };
    assert_eq!(findings_count, 2);

    if let LaneOutcome::Findings { findings } = &outcome {
        assert!(findings[0].rule_id().has_prefix("FUNC"));
        assert!(findings[1].rule_id().has_prefix("BYPASS"));
        assert!(findings[0].is_reject());
        assert!(findings[1].is_reject());
    }

    Ok(())
}

#[test]
fn ast_grep_lane_runtime_dispatch_matches_embedded_yaml_ids() {
    let rules_yaml = [
        include_str!("../rules/functional.yml"),
        include_str!("../rules/bypass.yml"),
        include_str!("../rules/architecture.yml"),
    ];

    let yaml_ids = rules_yaml
        .iter()
        .flat_map(|yaml| yaml.lines())
        .filter_map(|line| line.strip_prefix("id: "))
        .collect::<std::collections::BTreeSet<_>>();
    let runtime_ids = titania_lanes::ast_grep_lane::embedded_rule_ids()
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(runtime_ids, yaml_ids, "runtime RULES table must cover every embedded YAML id");
}

#[test]
fn ast_grep_lane_runtime_emits_every_embedded_rule_id() -> TestResult {
    let rules_yaml = [
        include_str!("../rules/functional.yml"),
        include_str!("../rules/bypass.yml"),
        include_str!("../rules/architecture.yml"),
    ];
    let fixture_paths = all_rule_fixture_paths();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    let emitted: std::collections::BTreeSet<_> = match outcome {
        LaneOutcome::Findings { findings } => {
            findings.iter().map(|finding| finding.rule_id().as_str().to_owned()).collect()
        }
        other => panic!("expected findings for full rule fixture matrix, got {other:?}"),
    };
    let expected: std::collections::BTreeSet<_> =
        titania_lanes::ast_grep_lane::embedded_rule_ids().map(|s| s.to_owned()).collect();

    assert_eq!(emitted, expected, "each embedded YAML rule id must be reachable at runtime");
    Ok(())
}

#[test]
fn ast_grep_lane_honors_yaml_ignore_filters() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths =
        ["tests/ignored_loop.rs", "build.rs"].map(|path| fixture_root("ast_grep", path)).to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "ignored paths must not emit findings: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_lane_maps_stderr_macros_to_stderr_rule() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths = ["functional/func_print_stderr_violation.rs"]
        .map(|path| fixture_root("ast_grep", path))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    let ids: std::collections::BTreeSet<_> = match outcome {
        LaneOutcome::Findings { findings } => {
            findings.iter().map(|finding| finding.rule_id().as_str().to_owned()).collect()
        }
        other => panic!("expected stderr finding, got {other:?}"),
    };

    assert!(ids.contains("FUNC_PRINT_STDERR"), "stderr macro must emit FUNC_PRINT_STDERR");
    assert!(!ids.contains("FUNC_PRINT_STDOUT"), "stderr macro must not emit FUNC_PRINT_STDOUT");
    Ok(())
}

#[test]
fn ast_grep_lane_does_not_reject_string_value_types_as_result_error() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths = ["functional/allowed_result_string_value.rs"]
        .map(|path| fixture_root("ast_grep", path))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "String as a map value with a non-String Result error must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_lane_does_not_reject_rngcore_as_rng_import() -> TestResult {
    let rules_yaml = [include_str!("../rules/architecture.yml")];
    let fixture_paths = ["crates/example-core/src/allowed_core_rngcore_import.rs"]
        .map(|path| fixture_root("ast_grep", path))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "RngCore must not match the ARCHITECTURE_IMPORT_CORE_RANDOM Rng boundary: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_lane_does_not_reject_std_boundary_prefix_names() -> TestResult {
    let rules_yaml = [include_str!("../rules/architecture.yml")];
    let fixture_paths = ["crates/example-core/src/allowed_core_std_boundary_imports.rs"]
        .map(|path| fixture_root("ast_grep", path))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "std::fs_extra and std::time::Instantaneous must not match fs/time boundary imports: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_lane_rejects_multiline_result_string_error() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths = [
        "functional/func_result_string_multiline_violation.rs",
        "functional/func_result_string_spaced_violation.rs",
    ]
    .map(|path| fixture_root("ast_grep", path))
    .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    let ids: std::collections::BTreeSet<_> = match outcome {
        LaneOutcome::Findings { findings } => {
            findings.iter().map(|finding| finding.rule_id().as_str().to_owned()).collect()
        }
        other => panic!("expected multiline Result<String> finding, got {other:?}"),
    };

    assert!(
        ids.contains("FUNC_RESULT_STRING"),
        "multiline Result<_, String> must emit FUNC_RESULT_STRING: {ids:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_lane_rejects_fully_qualified_result_string_error() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths = ["functional/func_result_string_fq_violation.rs"]
        .map(|path| fixture_root("ast_grep", path))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    let count = match outcome {
        LaneOutcome::Findings { findings } => findings
            .iter()
            .filter(|finding| finding.rule_id().as_str() == "FUNC_RESULT_STRING")
            .count(),
        other => panic!("expected fully-qualified Result<String> findings, got {other:?}"),
    };

    assert_eq!(count, 2, "both fully-qualified Result<String> spellings must be rejected");
    Ok(())
}

#[test]
fn ast_grep_lane_rejects_method_and_self_type_recursion() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths = ["functional/func_recursion_method_violation.rs"]
        .map(|path| fixture_root("ast_grep", path))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    let count = match outcome {
        LaneOutcome::Findings { findings } => findings
            .iter()
            .filter(|finding| finding.rule_id().as_str() == "FUNC_RECURSION_DIRECT")
            .count(),
        other => panic!("expected method recursion findings, got {other:?}"),
    };

    assert_eq!(count, 2, "self.walk() and Self::descend() must each be rejected");
    Ok(())
}

#[test]
fn ast_grep_lane_emits_one_finding_per_occurrence() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths =
        ["functional/func_print_stdout_violation.rs", "functional/func_unwrap_or_violation.rs"]
            .map(|path| fixture_root("ast_grep", path))
            .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    let (stdout_count, unwrap_or_count) = match outcome {
        LaneOutcome::Findings { findings } => (
            findings.iter().filter(|f| f.rule_id().as_str() == "FUNC_PRINT_STDOUT").count(),
            findings.iter().filter(|f| f.rule_id().as_str() == "FUNC_UNWRAP_OR").count(),
        ),
        other => panic!("expected per-occurrence findings, got {other:?}"),
    };

    assert_eq!(stdout_count, 3, "three print macros must emit three findings");
    assert_eq!(unwrap_or_count, 3, "three unwrap_or forms must emit three findings");
    Ok(())
}

#[test]
fn ast_grep_lane_inline_suppression_ignores_string_literals() -> TestResult {
    let rules_yaml = [include_str!("../rules/bypass.yml")];
    let allowed_paths = ["bypass/allowed_inline_suppression_in_string_literal.rs"]
        .map(|path| fixture_root("ast_grep", path))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &allowed_paths, &[])?;
    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "marker text inside string literals must not fire BYPASS_INLINE_SUPPRESSION: {outcome:?}"
    );

    let violation_paths = ["bypass/bypass_inline_suppression_violation.rs"]
        .map(|path| fixture_root("ast_grep", path))
        .to_vec();
    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &violation_paths, &[])?;
    let count = match outcome {
        LaneOutcome::Findings { findings } => findings
            .iter()
            .filter(|finding| finding.rule_id().as_str() == "BYPASS_INLINE_SUPPRESSION")
            .count(),
        other => panic!("expected inline suppression findings, got {other:?}"),
    };

    assert_eq!(count, 3, "three real marker comments must emit three findings");
    Ok(())
}

#[test]
fn ast_grep_lane_allows_injected_rng_trait() -> TestResult {
    let rules_yaml = [include_str!("../rules/architecture.yml")];
    let fixture_paths = ["crates/example-core/src/allowed_core_rand_rng_injection.rs"]
        .map(|path| fixture_root("ast_grep", path))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "an injected rand::Rng trait parameter must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_lane_rejects_pub_and_multiline_architecture_imports() -> TestResult {
    let rules_yaml = [include_str!("../rules/architecture.yml")];
    let fixture_paths = [
        "crates/example-core/src/architecture_import_core_fs_pub_violation.rs",
        "crates/example-core/src/architecture_import_core_fs_pub_spaced_violation.rs",
        "crates/example-core/src/architecture_import_core_time_multiline_violation.rs",
        "crates/example-core/src/architecture_import_core_random_pub_violation.rs",
    ]
    .map(|path| fixture_root("ast_grep", path))
    .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    let ids: std::collections::BTreeSet<_> = match outcome {
        LaneOutcome::Findings { findings } => {
            findings.iter().map(|finding| finding.rule_id().as_str().to_owned()).collect()
        }
        other => panic!("expected architecture import findings, got {other:?}"),
    };

    assert!(ids.contains("ARCHITECTURE_IMPORT_CORE_FS"), "pub use std::fs must be rejected");
    assert!(
        ids.contains("ARCHITECTURE_IMPORT_CORE_TIME"),
        "multiline std::time::Instant grouped import must be rejected"
    );
    assert!(
        ids.contains("ARCHITECTURE_IMPORT_CORE_RANDOM"),
        "pub use rand::thread_rng must be rejected"
    );
    Ok(())
}

#[test]
fn ast_grep_lane_rejects_fq_effect_paths_without_imports() -> TestResult {
    let rules_yaml = [include_str!("../rules/architecture.yml")];
    let fixture_paths = [
        "crates/example-core/src/architecture_import_core_fs_fq_violation.rs",
        "crates/example-core/src/architecture_import_core_process_fq_violation.rs",
    ]
    .map(|path| fixture_root("ast_grep", path))
    .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    let ids: std::collections::BTreeSet<_> = match outcome {
        LaneOutcome::Findings { findings } => {
            findings.iter().map(|finding| finding.rule_id().as_str().to_owned()).collect()
        }
        other => panic!("expected fully-qualified effect findings, got {other:?}"),
    };

    assert!(
        ids.contains("ARCHITECTURE_IMPORT_CORE_FS"),
        "std::fs::/std::env::/std::net:: with no import must be rejected: {ids:?}"
    );
    assert!(
        ids.contains("ARCHITECTURE_IMPORT_CORE_PROCESS"),
        "std::process::Command with no import must be rejected: {ids:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_lane_rejects_new_core_effect_boundaries() -> TestResult {
    let rules_yaml = [include_str!("../rules/architecture.yml")];
    let fixture_paths = [
        "crates/example-core/src/architecture_import_core_process_violation.rs",
        "crates/example-core/src/architecture_effect_core_async_violation.rs",
        "crates/example-core/src/architecture_effect_core_state_violation.rs",
    ]
    .map(|path| fixture_root("ast_grep", path))
    .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    let ids: std::collections::BTreeSet<_> = match outcome {
        LaneOutcome::Findings { findings } => {
            findings.iter().map(|finding| finding.rule_id().as_str().to_owned()).collect()
        }
        other => panic!("expected new effect boundary findings, got {other:?}"),
    };

    assert!(
        ids.contains("ARCHITECTURE_IMPORT_CORE_PROCESS"),
        "use std::process::Command must be rejected: {ids:?}"
    );
    assert!(
        ids.contains("ARCHITECTURE_EFFECT_CORE_ASYNC"),
        "async fn / .await / tokio::spawn must be rejected: {ids:?}"
    );
    assert!(
        ids.contains("ARCHITECTURE_EFFECT_CORE_STATE"),
        "static mut / Mutex must be rejected: {ids:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_lane_accepts_clean_core_effect_controls() -> TestResult {
    let rules_yaml = [include_str!("../rules/architecture.yml")];
    let fixture_paths = [
        "crates/example-core/src/allowed_no_core_process_import.rs",
        "crates/example-core/src/allowed_no_core_async_effect.rs",
        "crates/example-core/src/allowed_no_core_state_effect.rs",
    ]
    .map(|path| fixture_root("ast_grep", path))
    .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "clean process/async/state controls must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_functional_for_loop_in_line_comment_not_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths = ["functional/allowed_for_loop_in_line_comment.rs"]
        .map(|p| fixture_root("ast_grep", p))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "line-comment for-loop must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_functional_for_loop_in_block_comment_not_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths = ["functional/allowed_for_loop_in_block_comment.rs"]
        .map(|p| fixture_root("ast_grep", p))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "block-comment-body loop keywords must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_functional_loop_keywords_in_nested_block_comment_not_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths = ["functional/allowed_nested_block_comment_loops.rs"]
        .map(|p| fixture_root("ast_grep", p))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "nested block-comment loop keywords must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_functional_while_in_trailing_comment_not_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths = ["functional/allowed_while_in_trailing_comment.rs"]
        .map(|p| fixture_root("ast_grep", p))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "trailing-comment while/loop keywords must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_functional_loop_keywords_in_line_comments_only_are_clean() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths = ["functional/allowed_loop_keywords_in_strings.rs"]
        .map(|p| fixture_root("ast_grep", p))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "line-comment loop keywords must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_functional_loop_in_string_literal_not_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths = ["functional/allowed_loop_in_string_literal.rs"]
        .map(|p| fixture_root("ast_grep", p))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "string-literal loop keywords must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_functional_loop_in_raw_string_literal_not_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths = ["functional/allowed_raw_string_loop_keywords.rs"]
        .map(|p| fixture_root("ast_grep", p))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "raw-string loop keywords must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_functional_for_after_line_comment_delimiter_string_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths = ["functional/real_for_after_line_comment_delimiter_string.rs"]
        .map(|p| fixture_root("ast_grep", p))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "bounded for-loop after string containing // must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_functional_while_after_block_comment_delimiter_string_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths = ["functional/real_while_after_block_comment_delimiter_string.rs"]
        .map(|p| fixture_root("ast_grep", p))
        .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    let ids: std::collections::BTreeSet<_> = match outcome {
        LaneOutcome::Findings { findings } => {
            findings.iter().map(|f| f.rule_id().as_str().to_owned()).collect()
        }
        other => panic!("expected finding after block delimiter string, got {other:?}"),
    };

    assert!(
        ids.contains("BOUNDED_EXECUTION_UNBOUNDED_LOOP"),
        "real while-loop after string containing /* must emit BOUNDED_EXECUTION_UNBOUNDED_LOOP: {ids:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_functional_real_for_loop_still_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths =
        ["functional/real_for_loop_in_code.rs"].map(|p| fixture_root("ast_grep", p)).to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "bounded for-loops in real code must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_functional_real_unbounded_violations_still_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/functional.yml")];
    let fixture_paths = [
        "functional/bounded_execution_unbounded_loop_violation.rs",
        "functional/bounded_execution_unbounded_iterator_violation.rs",
    ]
    .map(|p| fixture_root("ast_grep", p))
    .to_vec();

    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;

    let ids: std::collections::BTreeSet<_> = match outcome {
        LaneOutcome::Findings { findings } => {
            findings.iter().map(|f| f.rule_id().as_str().to_owned()).collect()
        }
        other => panic!("expected findings for unbounded violations, got {other:?}"),
    };

    assert!(
        ids.contains("BOUNDED_EXECUTION_UNBOUNDED_LOOP"),
        "unbounded loop fixture must emit BOUNDED_EXECUTION_UNBOUNDED_LOOP: {ids:?}"
    );
    assert!(
        ids.contains("BOUNDED_EXECUTION_UNBOUNDED_ITERATOR"),
        "unbounded iterator fixture must emit BOUNDED_EXECUTION_UNBOUNDED_ITERATOR: {ids:?}"
    );
    Ok(())
}

fn all_rule_fixture_paths() -> Vec<PathBuf> {
    [
        "functional/bounded_execution_unbounded_loop_violation.rs",
        "functional/bounded_execution_unbounded_iterator_violation.rs",
        "functional/func_nesting_depth_violation.rs",
        "functional/func_recursion_direct_violation.rs",
        "functional/func_print_stdout_violation.rs",
        "functional/func_print_stderr_violation.rs",
        "functional/func_wildcard_import_violation.rs",
        "functional/func_unwrap_or_violation.rs",
        "functional/func_result_string_violation.rs",
        "bypass/bypass_allow_attr_violation.rs",
        "bypass/bypass_expect_attr_violation.rs",
        "bypass/bypass_cfg_attr_allow_violation.rs",
        "bypass/bypass_crate_allow_violation.rs",
        "bypass/bypass_crate_expect_violation.rs",
        "bypass/bypass_inline_suppression_violation.rs",
        "bypass/bypass_generated_include_violation.rs",
        "crates/example-core/src/architecture_import_core_infra_violation.rs",
        "crates/example-core/src/architecture_import_core_fs_grouped_violation.rs",
        "crates/example-core/src/architecture_import_core_time_violation.rs",
        "crates/example-core/src/architecture_import_core_random_violation.rs",
        "crates/example-core/src/architecture_import_core_process_violation.rs",
        "crates/example-core/src/architecture_effect_core_async_violation.rs",
        "crates/example-core/src/architecture_effect_core_state_violation.rs",
    ]
    .map(|path| fixture_root("ast_grep", path))
    .to_vec()
}
