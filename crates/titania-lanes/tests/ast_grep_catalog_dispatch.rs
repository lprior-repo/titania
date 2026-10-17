use std::{error::Error, path::PathBuf};

use titania_core::LaneOutcome;

type TestResult = Result<(), Box<dyn Error>>;

fn fixture_root(name: &str) -> PathBuf {
    let base = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(base).join("tests").join("fixtures").join("ast_grep").join(name)
}

#[test]
fn exact_id_enables_func_loops_for() -> TestResult {
    let catalog = "\
id: BOUNDED_EXECUTION_UNBOUNDED_LOOP
language: Rust
";
    let fixture = fixture_root("functional/bounded_execution_unbounded_loop_violation.rs");
    let outcome = titania_lanes::ast_grep_lane::run(&[catalog], &[fixture], &[])?;
    match outcome {
        LaneOutcome::Findings { findings } => {
            assert!(
                findings.iter().any(|f| f.rule_id().as_str() == "BOUNDED_EXECUTION_UNBOUNDED_LOOP"),
                "exact `id: BOUNDED_EXECUTION_UNBOUNDED_LOOP` must enable the rule, got findings: {:?}",
                findings.iter().map(|f| f.rule_id().as_str()).collect::<Vec<_>>()
            );
            Ok(())
        }
        other => Err(format!("expected Findings for exact id, got {other:?}").into()),
    }
}

#[test]
fn quoted_id_with_inline_comment_enables_func_loops_for() -> TestResult {
    let catalog = "id: \"BOUNDED_EXECUTION_UNBOUNDED_LOOP\" # documented rule\nlanguage: Rust\n";
    let fixture = fixture_root("functional/bounded_execution_unbounded_loop_violation.rs");
    let outcome = titania_lanes::ast_grep_lane::run(&[catalog], &[fixture], &[])?;
    let findings = match outcome {
        LaneOutcome::Findings { findings } => findings,
        other => return Err(format!("expected Findings, got {other:?}").into()),
    };
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule_id().as_str() == "BOUNDED_EXECUTION_UNBOUNDED_LOOP"),
        "quoted exact id must enable the rule: {findings:?}"
    );
    Ok(())
}

#[test]
fn prefix_id_does_not_enable_func_loops_for() -> TestResult {
    let catalog = "id: BOUNDED_EXECUTION_UNBOUNDED_LOOP_EXTRA\n";
    let fixture = fixture_root("functional/bounded_execution_unbounded_loop_violation.rs");
    let err = titania_lanes::ast_grep_lane::run(&[catalog], &[fixture], &[]).expect_err(
        "prefix id must not enable BOUNDED_EXECUTION_UNBOUNDED_LOOP and must be rejected",
    );
    match err {
        titania_lanes::ast_grep_lane::AstGrepLaneError::UnknownRuleId { id } => {
            assert_eq!(id, "BOUNDED_EXECUTION_UNBOUNDED_LOOP_EXTRA");
            Ok(())
        }
        other => Err(format!("expected UnknownRuleId, got {other:?}").into()),
    }
}

#[test]
fn comment_id_does_not_enable_func_loops_for() -> TestResult {
    let catalog = "\
# Catalog note: this comment mentions id: BOUNDED_EXECUTION_UNBOUNDED_LOOP
# but no top-level id is declared.
# The lane must not enable BOUNDED_EXECUTION_UNBOUNDED_LOOP based on comment text.
";
    let fixture = fixture_root("functional/bounded_execution_unbounded_loop_violation.rs");
    let outcome = titania_lanes::ast_grep_lane::run(&[catalog], &[fixture], &[])?;
    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "comment-only id must not enable rule, got {outcome:?}"
    );
    Ok(())
}

#[test]
fn message_text_substring_does_not_enable_func_loops_for() -> TestResult {
    let catalog = "\
id: BYPASS_GENERATED_INCLUDE
language: Rust
severity: error
message: \"BOUNDED_EXECUTION_UNBOUNDED_LOOP: a note referencing id: BOUNDED_EXECUTION_UNBOUNDED_LOOP inside message text\"
rule:
  regex: 'include!\\s*\\(\\s*concat!\\s*\\(\\s*env!\\s*\\(\\s*\"?OUT_DIR\"?\\s*\\)\\s*,\\s*\"[^\"]*\"\\s*\\)\\s*\\)\\s*;?'
metadata:
  effect: Reject
";
    let fixture = fixture_root("functional/bounded_execution_unbounded_loop_violation.rs");
    let outcome = titania_lanes::ast_grep_lane::run(&[catalog], &[fixture], &[])?;
    let findings: Vec<String> = match outcome {
        LaneOutcome::Clean { .. } => return Ok(()),
        LaneOutcome::Findings { findings } => {
            findings.iter().map(|f| f.rule_id().as_str().to_owned()).collect()
        }
        other => return Err(format!("unexpected LaneOutcome variant: {other:?}").into()),
    };
    assert!(
        !findings.iter().any(|id| id == "BOUNDED_EXECUTION_UNBOUNDED_LOOP"),
        "BOUNDED_EXECUTION_UNBOUNDED_LOOP must NOT be enabled by message substring: {findings:?}"
    );
    assert!(
        findings.is_empty(),
        "no rule should fire on the unbounded-loop fixture in this catalog: {findings:?}"
    );
    Ok(())
}

#[test]
fn empty_id_is_rejected_as_unknown() -> TestResult {
    let catalog = "id: \n";
    let fixture = fixture_root("functional/bounded_execution_unbounded_loop_violation.rs");
    let err = titania_lanes::ast_grep_lane::run(&[catalog], &[fixture], &[])
        .expect_err("empty id must be rejected");
    match err {
        titania_lanes::ast_grep_lane::AstGrepLaneError::UnknownRuleId { id } => {
            assert_eq!(id, "");
            Ok(())
        }
        other => Err(format!("expected UnknownRuleId for empty id, got {other:?}").into()),
    }
}

#[test]
fn ast_grep_bypass_generated_include_real_in_code_still_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/bypass.yml")];
    let fixture_paths =
        ["bypass/real_generated_include_in_code.rs"].map(|p| fixture_root(p)).to_vec();
    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    let ids: std::collections::BTreeSet<_> = match outcome {
        LaneOutcome::Findings { findings } => {
            findings.iter().map(|f| f.rule_id().as_str().to_owned()).collect()
        }
        other => panic!("expected findings for real generated include, got {other:?}"),
    };
    assert!(
        ids.contains("BYPASS_GENERATED_INCLUDE"),
        "real include!(concat!(env!(\"OUT_DIR\"), ...)) must emit BYPASS_GENERATED_INCLUDE: {ids:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_bypass_generated_include_in_line_comment_not_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/bypass.yml")];
    let fixture_paths =
        ["bypass/allowed_generated_include_in_line_comment.rs"].map(|p| fixture_root(p)).to_vec();
    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "line-comment generated include must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_bypass_generated_include_in_block_comment_not_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/bypass.yml")];
    let fixture_paths =
        ["bypass/allowed_generated_include_in_block_comment.rs"].map(|p| fixture_root(p)).to_vec();
    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "block-comment generated include must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_bypass_generated_include_in_string_literal_not_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/bypass.yml")];
    let fixture_paths =
        ["bypass/allowed_generated_include_in_string_literal.rs"].map(|p| fixture_root(p)).to_vec();
    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "string-literal generated include must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_bypass_generated_include_in_raw_string_literal_not_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/bypass.yml")];
    let fixture_paths = ["bypass/allowed_generated_include_in_raw_string_literal.rs"]
        .map(|p| fixture_root(p))
        .to_vec();
    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "raw-string generated include must stay clean: {outcome:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_bypass_generated_include_raw_string_in_code_still_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/bypass.yml")];
    let fixture_paths = ["bypass/bypass_generated_include_raw_string_violation.rs"]
        .map(|p| fixture_root(p))
        .to_vec();
    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    let ids: std::collections::BTreeSet<_> = match outcome {
        LaneOutcome::Findings { findings } => {
            findings.iter().map(|f| f.rule_id().as_str().to_owned()).collect()
        }
        other => panic!("expected findings for raw-string generated include, got {other:?}"),
    };
    assert!(
        ids.contains("BYPASS_GENERATED_INCLUDE"),
        "raw-string include!(concat!(env!(r#\"OUT_DIR\"#), r#\"/gen.rs\"#)) must emit BYPASS_GENERATED_INCLUDE: {ids:?}"
    );
    Ok(())
}

#[test]
fn ast_grep_bypass_generated_include_multi_part_concat_detected() -> TestResult {
    let rules_yaml = [include_str!("../rules/bypass.yml")];
    let fixture_paths =
        ["bypass/bypass_generated_include_multi_part.rs"].map(|p| fixture_root(p)).to_vec();
    let outcome = titania_lanes::ast_grep_lane::run(&rules_yaml, &fixture_paths, &[])?;
    let ids: std::collections::BTreeSet<_> = match outcome {
        LaneOutcome::Findings { findings } => {
            findings.iter().map(|finding| finding.rule_id().as_str().to_owned()).collect()
        }
        other => panic!("expected findings for multi-part generated include, got {other:?}"),
    };
    assert!(ids.contains("BYPASS_GENERATED_INCLUDE"));
    Ok(())
}
