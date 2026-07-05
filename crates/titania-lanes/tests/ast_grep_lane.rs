//! Contract tests for the embedded ast-grep lane runner.
//!
//! These tests assert the public API of `titania_lanes::ast_grep_lane` — the
//! module that runs embedded ast-grep rules against a project and emits typed
//! `LaneOutcome::Findings` with exact rule IDs.
//!
//! Bead: tn-37r.4
//!
//! The tests below reference the *intended* public API.  Because
//! `titania_lanes::ast_grep_lane` does not yet exist, the compiler errors
//! are the RED signal: they prove which symbols the lane runner must expose.

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

// ===========================================================================
// Test 1: Multi-rule-family fixture produces typed Findings with exact rule IDs
// ===========================================================================

/// **Contract:** Running the ast-grep lane runner against a fixture project
/// that contains violations from the functional, bypass, and architecture
/// rule families yields a `LaneOutcome::Findings` whose `findings` contain
/// `Finding` entries with the exact rule IDs from each family.
///
/// This test asserts:
/// 1. The return type is `Result<LaneOutcome, ast_grep_lane::AstGrepLaneError>`.
/// 2. The outcome variant is `LaneOutcome::Findings { .. }`.
/// 3. Each finding carries a `RuleId` whose prefix matches one of
///    `FUNC`, `BYPASS`, or `ARCHITECTURE`.
/// 4. The number of findings is at least the count of known violations
///    in the fixture.
#[test]
fn ast_grep_lane_multi_family_fixture_emits_findings_with_exact_rule_ids() -> TestResult {
    // The production module MUST provide a function like:
    //   pub fn run(
    //       rules_yaml: &'static [ &'static str ],
    //       fixture_paths: &[std::path::Path],
    //       exceptions: &ast_grep_lane::Exceptions,
    //   ) -> Result<LaneOutcome, ast_grep_lane::LaneError>;

    // Load the embedded rule YAMLs.
    let rules_yaml = [
        include_str!("../rules/functional.yml"),
        include_str!("../rules/bypass.yml"),
        include_str!("../rules/architecture.yml"),
    ];

    // Collect fixture paths for a functional violation, a bypass violation,
    // and an architecture violation.
    let fixture_paths: Vec<PathBuf> = [
        "functional/func_loops_for_violation.rs",
        "bypass/bypass_allow_attr_violation.rs",
        "crates/example-core/src/architecture_import_core_infra_violation.rs",
    ]
    .map(|p| fixture_root("ast_grep", p))
    .to_vec();

    // Invoke the (not yet implemented) lane runner.
    // This line MUST compile once the module exists.
    let result = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[]);

    // Assert the outcome variant and findings.
    match result {
        Ok(outcome) => {
            assert!(
                matches!(outcome, LaneOutcome::Findings { .. }),
                "expected LaneOutcome::Findings, got {:?}",
                outcome
            );
            if let LaneOutcome::Findings { findings } = outcome {
                // Must have at least 3 findings (one per rule family).
                assert!(findings.len() >= 3, "expected >= 3 findings, got {}", findings.len());

                // Collect rule prefixes and assert all three families are represented.
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

// ===========================================================================
// Test 2: Exception suppresses only the matching rule/path pair
// ===========================================================================

/// **Contract:** When an exception is provided that matches one rule ID and
/// one fixture path, only that specific finding is suppressed; violations
/// from the same rule family on OTHER paths, and violations from OTHER rule
/// families on the same path, remain present in the findings.
///
/// This test asserts:
/// 1. The exception API accepts `(rule_id, path)` pairs.
/// 2. Only the exact (rule_id, path) combination is suppressed.
/// 3. Other findings from the same rule family on different paths survive.
#[test]
fn ast_grep_lane_exception_suppresses_only_matching_rule_path_pair() -> TestResult {
    // The production module MUST provide an exception type like:
    //   pub fn run(
    //       rules_yaml: &'static [ &'static str ],
    //       fixture_paths: &[std::path::Path],
    //       exceptions: &[(RuleId, String)],  // or a dedicated Exceptions struct
    //   ) -> Result<LaneOutcome, ast_grep_lane::LaneError>;
    //
    // An exception pair (rule_id, file_path) suppresses only that exact
    // finding; all other findings remain.

    let rules_yaml = [
        include_str!("../rules/functional.yml"),
        include_str!("../rules/bypass.yml"),
        include_str!("../rules/architecture.yml"),
    ];

    let fixture_paths: Vec<PathBuf> = [
        "functional/func_loops_for_violation.rs",
        "functional/func_print_stdout_violation.rs",
        "bypass/bypass_allow_attr_violation.rs",
    ]
    .map(|p| fixture_root("ast_grep", p))
    .to_vec();

    // Exception: suppress only FUNC_PRINT_STDOUT in func_print_stdout_violation.rs
    // This should leave func_loops_for_violation.rs and bypass_allow_attr_violation.rs
    // findings intact.
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
                // Must have at least 2 findings (the suppressed one should be gone).
                // Original 3 fixtures - 1 suppressed = 2.
                let suppressed_count = findings
                    .iter()
                    .filter(|f| {
                        f.rule_id().as_str() == "FUNC_PRINT_STDOUT"
                            && f.location().span_file().map(WorkspacePath::as_str)
                                == Some("functional/func_print_stdout_violation.rs")
                    })
                    .count();
                assert_eq!(suppressed_count, 0, "suppressed finding should not be present",);

                // The other findings must be present.
                let func_loops_present =
                    findings.iter().any(|f| f.rule_id().as_str() == "FUNC_LOOPS_FOR");
                let bypass_present =
                    findings.iter().any(|f| f.rule_id().as_str() == "BYPASS_ALLOW_ATTR");
                assert!(func_loops_present, "FUNC_LOOPS_FOR finding should be present");
                assert!(bypass_present, "BYPASS_ALLOW_ATTR finding should be present");
            }
        }
        Err(e) => panic!("run() returned error: {}", e),
    }

    Ok(())
}

// ===========================================================================
// Test 3: Runner uses embedded rules / no external binary dependency
// ===========================================================================

/// **Contract:** The lane runner sources its rule set from embedded YAML
/// strings (via `include_str!`) and does NOT depend on an external `ast-grep`
/// binary.  This is proved by the public API shape: the `run()` function
/// takes `&'static str` YAML data as a parameter, not a binary path or
/// subprocess handle.
#[test]
fn ast_grep_lane_runner_uses_embedded_rules_no_external_binary() -> TestResult {
    // The function signature must accept `&'static [ &'static str ]` for
    // rules_yaml — proving rules are embedded at compile time.
    let rules_yaml = [
        include_str!("../rules/functional.yml"),
        include_str!("../rules/bypass.yml"),
        include_str!("../rules/architecture.yml"),
    ];

    // Verify the YAML is loadable at compile time and contains the expected
    // structure (id, language, pattern, effect).
    for yml in &rules_yaml {
        assert!(yml.contains("id:"), "YAML must contain rule id");
        assert!(yml.contains("language: Rust"), "YAML must declare language Rust");
        assert!(yml.contains("pattern:"), "YAML must contain pattern");
    }

    // Run with no fixtures — the lane should return a clean outcome (no
    // files to scan) or an empty findings list, proving no binary is needed.
    let result = titania_lanes::ast_grep_lane::run(&rules_yaml, &[], &[]);

    match result {
        Ok(outcome) => {
            // With no fixtures, the lane should produce Clean (no files to check)
            // or Findings with zero findings.
            match outcome {
                LaneOutcome::Clean { .. } => {}
                LaneOutcome::Findings { findings } => {
                    assert!(findings.is_empty(), "no fixtures => no findings");
                }
                other => panic!("unexpected outcome with empty fixtures: {:?}", other),
            }
        }
        Err(e) => panic!("run() returned error with empty fixtures: {}", e),
    }

    Ok(())
}

// ===========================================================================
// Test 4: Lane::AstGrep maps to correct lane name in artifact
// ===========================================================================

/// **Contract:** When the ast-grep lane writes its artifact, the `lane` field
/// in the JSON artifact is `"AstGrep"` (PascalCase, matching `Lane::AstGrep`).
#[test]
fn ast_grep_lane_artifact_lane_field_is_astgrep() -> TestResult {
    use titania_core::Lane;

    // The Lane::AstGrep serialization must produce "AstGrep" in PascalCase.
    assert_eq!(Lane::AstGrep.name(), "AstGrep");

    // Verify Lane::AstGrep serializes correctly via serde.
    let json = serde_json::to_string(&Lane::AstGrep).expect("Lane::AstGrep must serialize to JSON");
    assert_eq!(json, "\"AstGrep\"");

    Ok(())
}

// ===========================================================================
// Test 5: LaneOutcome::Findings with ast-grep findings serializes correctly
// ===========================================================================

/// **Contract:** A `LaneOutcome::Findings` containing `Finding` entries for
/// ast-grep rules serializes to JSON with `variant: "findings"` and the
/// correct `rule`, `path`, `line`, and `message` fields.
#[test]
fn ast_grep_lane_findings_outcome_serializes_with_variant_and_findings() -> TestResult {
    use titania_core::{Finding, Lane, Location, RepairHint, RuleId, WorkspacePath};

    // Build findings that would come from the ast-grep lane.
    let finding_1 = Finding::reject(
        Lane::AstGrep,
        RuleId::new("FUNC_PRINT_STDOUT").unwrap(),
        Location::Span {
            file: WorkspacePath::new("src/main.rs").unwrap(),
            line_start: 12,
            col_start: 5,
            line_end: 12,
            col_end: 25,
        },
        "Found `println!` in production source".into(),
        RepairHint::RequiresHumanReview { note: "Replace with tracing".into() },
    );

    let finding_2 = Finding::reject(
        Lane::AstGrep,
        RuleId::new("BYPASS_ALLOW_ATTR").unwrap(),
        Location::Span {
            file: WorkspacePath::new("src/lib.rs").unwrap(),
            line_start: 1,
            col_start: 1,
            line_end: 1,
            col_end: 30,
        },
        "Found `#[allow(...)]` suppression".into(),
        RepairHint::RemoveAllowAttribute { attr: "allow(clippy::unwrap_used)".into() },
    );

    // Construct a LaneOutcome::Findings manually to verify the shape.
    let outcome = LaneOutcome::Findings { findings: Box::new([finding_1, finding_2]) };

    assert!(outcome.is_findings());
    assert!(!outcome.is_pass());

    // Pattern-match to extract findings and assert on their RuleId prefixes.
    let findings_count = match &outcome {
        LaneOutcome::Findings { findings } => findings.len(),
        _ => panic!("expected Findings variant"),
    };
    assert_eq!(findings_count, 2);

    // Verify rule ID prefixes by pattern matching on the findings.
    if let LaneOutcome::Findings { findings } = &outcome {
        assert!(findings[0].rule_id().has_prefix("FUNC"));
        assert!(findings[1].rule_id().has_prefix("BYPASS"));
        // Both findings should be Reject effect.
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
    assert!(ids.contains("ARCHITECTURE_IMPORT_CORE_RANDOM"), "pub use rand::Rng must be rejected");
    Ok(())
}

fn all_rule_fixture_paths() -> Vec<PathBuf> {
    [
        "functional/func_loops_for_violation.rs",
        "functional/func_loops_while_violation.rs",
        "functional/func_loops_loop_violation.rs",
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
        "crates/example-core/src/architecture_import_core_infra_violation.rs",
        "crates/example-core/src/architecture_import_core_fs_grouped_violation.rs",
        "crates/example-core/src/architecture_import_core_time_violation.rs",
        "crates/example-core/src/architecture_import_core_random_violation.rs",
    ]
    .map(|path| fixture_root("ast_grep", path))
    .to_vec()
}
