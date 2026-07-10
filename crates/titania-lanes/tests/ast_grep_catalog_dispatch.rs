//! Focused contract tests for `titania_lanes::ast_grep_lane`.
//!
//! Two related concerns are exercised here, neither of which overlaps the
//! shared `tests/ast_grep_lane.rs` matrix or the generated-include fixture work
//! owned by the AST-grep detector specialist.
//!
//! 1. **Exact catalog dispatch.** The lane must enable rules only from
//!    document-level YAML `id:` fields. A prefix id such as
//!    `FUNC_LOOPS_FOR_EXTRA`, a comment that mentions `id: FUNC_LOOPS_FOR`,
//!    and a `message:` field containing the substring must NOT enable
//!    `FUNC_LOOPS_FOR`. Unknown ids are rejected explicitly via
//!    [`AstGrepLaneError::UnknownRuleId`] instead of being silently accepted.
//!
//! 2. **`BYPASS_GENERATED_INCLUDE` detector parity.** Real `include!(
//!    concat!(env!("OUT_DIR"), "..."))` in code must trigger the rule, and
//!    the same surface pattern inside line comments, block comments, regular
//!    string literals, and raw string literals must NOT trigger it. These
//!    prove the AST-structural detector is not regressed by the surrounding
//!    catalog-dispatch refactor.

use std::{error::Error, path::PathBuf};

use titania_core::LaneOutcome;

type TestResult = Result<(), Box<dyn Error>>;

fn fixture_root(name: &str) -> PathBuf {
    let base = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(base).join("tests").join("fixtures").join("ast_grep").join(name)
}

// ===========================================================================
// Exact catalog dispatch — comments, message text, and prefix ids must not
// enable `FUNC_LOOPS_FOR`.
// ===========================================================================

/// **Contract:** A catalog document whose top-level `id:` is exactly
/// `FUNC_LOOPS_FOR` enables that rule and produces a finding on a fixture
/// that contains a real imperative for-loop.
#[test]
fn exact_id_enables_func_loops_for() -> TestResult {
    let catalog = "\
id: FUNC_LOOPS_FOR
language: Rust
";
    let fixture = fixture_root("functional/func_loops_for_violation.rs");
    let outcome = titania_lanes::ast_grep_lane::run(&[catalog], &[fixture], &[])?;
    match outcome {
        LaneOutcome::Findings { findings } => {
            assert!(
                findings.iter().any(|f| f.rule_id().as_str() == "FUNC_LOOPS_FOR"),
                "exact `id: FUNC_LOOPS_FOR` must enable the rule, got findings: {:?}",
                findings.iter().map(|f| f.rule_id().as_str()).collect::<Vec<_>>()
            );
            Ok(())
        }
        other => Err(format!("expected Findings for exact id, got {other:?}").into()),
    }
}

/// **Contract:** Quoted document ids with an inline YAML comment normalize to
/// the same exact rule id as an unquoted id.
#[test]
fn quoted_id_with_inline_comment_enables_func_loops_for() -> TestResult {
    let catalog = "id: \"FUNC_LOOPS_FOR\" # documented rule\nlanguage: Rust\n";
    let fixture = fixture_root("functional/func_loops_for_violation.rs");
    let outcome = titania_lanes::ast_grep_lane::run(&[catalog], &[fixture], &[])?;
    let findings = match outcome {
        LaneOutcome::Findings { findings } => findings,
        other => return Err(format!("expected Findings, got {other:?}").into()),
    };
    assert!(
        findings.iter().any(|finding| finding.rule_id().as_str() == "FUNC_LOOPS_FOR"),
        "quoted exact id must enable the rule: {findings:?}"
    );
    Ok(())
}

/// **Contract:** A catalog document whose top-level `id:` is a prefix
/// string (`FUNC_LOOPS_FOR_EXTRA`) must NOT enable `FUNC_LOOPS_FOR`. The
/// old substring dispatch would have enabled it; the new dispatch rejects
/// the unknown id explicitly via [`AstGrepLaneError::UnknownRuleId`].
#[test]
fn prefix_id_does_not_enable_func_loops_for() -> TestResult {
    let catalog = "id: FUNC_LOOPS_FOR_EXTRA\n";
    let fixture = fixture_root("functional/func_loops_for_violation.rs");
    let err = titania_lanes::ast_grep_lane::run(&[catalog], &[fixture], &[])
        .expect_err("prefix id must not enable FUNC_LOOPS_FOR and must be rejected");
    match err {
        titania_lanes::ast_grep_lane::AstGrepLaneError::UnknownRuleId { id } => {
            assert_eq!(id, "FUNC_LOOPS_FOR_EXTRA");
            Ok(())
        }
        other => Err(format!("expected UnknownRuleId, got {other:?}").into()),
    }
}

/// **Contract:** A catalog that only contains a comment mentioning
/// `id: FUNC_LOOPS_FOR` must NOT enable `FUNC_LOOPS_FOR`. The lane returns
/// `Clean` because no document-level id is present.
#[test]
fn comment_id_does_not_enable_func_loops_for() -> TestResult {
    let catalog = "\
# Catalog note: this comment mentions id: FUNC_LOOPS_FOR
# but no top-level id is declared.
# The lane must not enable FUNC_LOOPS_FOR based on comment text.
";
    let fixture = fixture_root("functional/func_loops_for_violation.rs");
    let outcome = titania_lanes::ast_grep_lane::run(&[catalog], &[fixture], &[])?;
    assert!(
        matches!(outcome, LaneOutcome::Clean { .. }),
        "comment-only id must not enable rule, got {outcome:?}"
    );
    Ok(())
}

/// **Contract:** A catalog whose `message:` field contains the substring
/// `id: FUNC_LOOPS_FOR` must NOT enable `FUNC_LOOPS_FOR`. Only the document-
/// level `id:` is authoritative. The other id in the catalog
/// (`BYPASS_GENERATED_INCLUDE`) is a known rule that does not apply to the
/// `for`-loop fixture, so the outcome is `Clean`.
#[test]
fn message_text_substring_does_not_enable_func_loops_for() -> TestResult {
    let catalog = "\
id: BYPASS_GENERATED_INCLUDE
language: Rust
severity: error
message: \"FUNC_LOOPS_FOR: a note referencing id: FUNC_LOOPS_FOR inside message text\"
rule:
  regex: 'include!\\s*\\(\\s*concat!\\s*\\(\\s*env!\\s*\\(\\s*\"?OUT_DIR\"?\\s*\\)\\s*,\\s*\"[^\"]*\"\\s*\\)\\s*\\)\\s*;?'
metadata:
  effect: Reject
";
    let fixture = fixture_root("functional/func_loops_for_violation.rs");
    let outcome = titania_lanes::ast_grep_lane::run(&[catalog], &[fixture], &[])?;
    let findings: Vec<String> = match outcome {
        LaneOutcome::Clean { .. } => return Ok(()),
        LaneOutcome::Findings { findings } => {
            findings.iter().map(|f| f.rule_id().as_str().to_owned()).collect()
        }
        other => return Err(format!("unexpected LaneOutcome variant: {other:?}").into()),
    };
    assert!(
        !findings.iter().any(|id| id == "FUNC_LOOPS_FOR"),
        "FUNC_LOOPS_FOR must NOT be enabled by message substring: {findings:?}"
    );
    assert!(
        findings.is_empty(),
        "no rule should fire on the for-loop fixture in this catalog: {findings:?}"
    );
    Ok(())
}

/// **Contract:** A catalog document whose `id:` value is empty (e.g. `id: `)
/// is malformed. The lane rejects it explicitly instead of silently enabling
/// every rule or accepting the unknown value.
#[test]
fn empty_id_is_rejected_as_unknown() -> TestResult {
    let catalog = "id: \n";
    let fixture = fixture_root("functional/func_loops_for_violation.rs");
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

// ===========================================================================
// BYPASS_GENERATED_INCLUDE — positive control and false-positive resistance
// ===========================================================================

/// **Contract:** A real `include!(concat!(env!("OUT_DIR"), "..."))` in
/// executable code MUST trigger `BYPASS_GENERATED_INCLUDE`. Positive control
/// proving the AST-structural detector still fires on genuine generated-
/// include macros.
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

/// **Contract:** A whole-line comment containing the generated-include
/// pattern must NOT trigger `BYPASS_GENERATED_INCLUDE`. The AST detector
/// matches only `macro_invocation` nodes; line comments never produce one.
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

/// **Contract:** A block comment containing the generated-include pattern
/// must NOT trigger `BYPASS_GENERATED_INCLUDE`. Block comments never
/// produce `macro_invocation` AST nodes.
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

/// **Contract:** A string literal containing the generated-include pattern
/// must NOT trigger `BYPASS_GENERATED_INCLUDE`. String content is data, not
/// a macro invocation, and never produces a `macro_invocation` AST node.
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

/// **Contract:** A raw string literal containing the generated-include
/// pattern must NOT trigger `BYPASS_GENERATED_INCLUDE`. Raw string content
/// is data, not a macro invocation.
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

/// **Contract:** A raw-string `include!(concat!(env!(r#"OUT_DIR"#), r#"/gen.rs"#));`
/// in executable code MUST trigger `BYPASS_GENERATED_INCLUDE`. Tree-sitter
/// parses the raw string form as `raw_string_literal`, not `string_literal`,
/// so the detector has to decode the inner content of either literal kind
/// and still recognise the exact `OUT_DIR` identifier plus a non-empty path.
/// This proves the raw-string bypass is closed.
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
