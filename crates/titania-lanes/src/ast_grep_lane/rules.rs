//! Runtime dispatch table for embedded ast-grep YAML rules.
//!
//! Each [`RuleDef`] pairs a rule id with a [`Detector`] closure. Real
//! ast-grep patterns power every structural rule (FUNC_*). The bypass
//! attribute rules also use ast-grep. Architecture import rules keep
//! their hand-rolled string detectors because the path-scope filter and
//! grouped-import boundary logic is not naturally expressible as a
//! single ast-grep pattern — see the residual section in the bead
//! report.

mod detectors;
mod filters;

use titania_core::{FindingEffect, RepairHint, WorkspacePath};

use super::engine::AstEngine;

/// Detector function pointer over the parsed ast-grep engine.
type EngineDetector = fn(&AstEngine) -> Option<usize>;

/// Runtime rule definition corresponding to one embedded YAML document.
#[derive(Debug, Clone, Copy)]
pub(super) struct RuleDef {
    /// YAML rule id.
    pub(super) id: &'static str,
    /// Finding message copied from YAML.
    pub(super) message: &'static str,
    /// Finding effect copied from YAML metadata.
    pub(super) effect: FindingEffect,
    /// Repair hint kind copied from YAML metadata.
    pub(super) repair: RepairKind,
    /// Path scope copied from YAML `files` and `ignores` selectors.
    scope: RuleScope,
    /// Engine-or-string detector producing the first match line.
    pub(super) detect: Detector,
}

/// Engine-vs-string detector selector.
///
/// The engine path runs against the real ast-grep parse tree. The string
/// path is reserved for rules that ast-grep cannot express (path-scope
/// filter + grouped imports, inline-suppression comments).
#[derive(Debug, Clone, Copy)]
pub(super) enum Detector {
    /// Real ast-grep engine detector.
    Engine(EngineDetector),
    /// Legacy hand-rolled string detector.
    String(fn(&str) -> bool),
}

impl Detector {
    /// Run the detector against the parsed engine and the raw source.
    ///
    /// Engine detectors delegate to ast-grep; string detectors ignore the
    /// engine and report line 0 on a successful text match (preserving the
    /// legacy "file-level finding" semantics of the hand-rolled scanners).
    pub(super) fn run(self, engine: &AstEngine, source: &str) -> Option<usize> {
        match self {
            Self::Engine(detect) => detect(engine),
            Self::String(detect) => detect(source).then_some(0),
        }
    }
}

/// Internal repair hint selector.
#[derive(Debug, Clone, Copy)]
pub(super) enum RepairKind {
    UseIteratorPipeline,
    RemoveAllowAttribute,
    ReplaceDependency,
    RequiresHumanReview,
    FlattenNesting,
}

#[derive(Debug, Clone, Copy)]
enum RuleScope {
    ProductionRust,
    CoreDomainRust,
}

pub(super) fn rule_applies(rule: &RuleDef, workspace_path: &WorkspacePath) -> bool {
    filters::rule_applies(rule, workspace_path)
}

pub(super) fn repair_hint(kind: RepairKind) -> RepairHint {
    match kind {
        RepairKind::UseIteratorPipeline => RepairHint::use_iterator_pipeline(
            "replace imperative control flow with an iterator pipeline".to_owned(),
        ),
        RepairKind::RemoveAllowAttribute => RepairHint::remove_allow_attribute("allow".to_owned()),
        RepairKind::ReplaceDependency => RepairHint::replace_dependency(
            "tokio/axum/sqlx/reqwest".to_owned(),
            "typed port".to_owned(),
        ),
        RepairKind::RequiresHumanReview => {
            RepairHint::requires_human_review("manual structural rewrite required".to_owned())
        }
        RepairKind::FlattenNesting => {
            RepairHint::flatten_nesting("flatten deeply nested control flow".to_owned())
        }
    }
}

pub(super) const RULES: &[RuleDef] = &[
    RuleDef {
        id: "FUNC_LOOPS_FOR",
        message: "FUNC_LOOPS_FOR: replace imperative for loops with iterator pipelines",
        effect: FindingEffect::Reject,
        repair: RepairKind::UseIteratorPipeline,
        detect: Detector::Engine(AstEngine::detect_for_loop),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "FUNC_LOOPS_WHILE",
        message: "FUNC_LOOPS_WHILE: replace while loops with bounded iterator or state-machine transitions",
        effect: FindingEffect::Reject,
        repair: RepairKind::UseIteratorPipeline,
        detect: Detector::Engine(AstEngine::detect_while_loop),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "FUNC_LOOPS_LOOP",
        message: "FUNC_LOOPS_LOOP: replace open-ended loop blocks with explicit bounded control flow",
        effect: FindingEffect::Reject,
        repair: RepairKind::UseIteratorPipeline,
        detect: Detector::Engine(AstEngine::detect_loop_block),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "FUNC_NESTING_DEPTH",
        message: "FUNC_NESTING_DEPTH: flatten deeply nested control flow (max nesting depth > 2)",
        effect: FindingEffect::Reject,
        repair: RepairKind::FlattenNesting,
        detect: Detector::Engine(AstEngine::detect_nesting_depth),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "FUNC_RECURSION_DIRECT",
        message: "FUNC_RECURSION_DIRECT: function calls itself by name; rewrite to iteration or bound recursion",
        effect: FindingEffect::Reject,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::Engine(AstEngine::detect_recursion_direct),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "FUNC_PRINT_STDOUT",
        message: "FUNC_PRINT_STDOUT: use typed output/reporting instead of print! or println!",
        effect: FindingEffect::Reject,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::Engine(AstEngine::detect_print_stdout),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "FUNC_PRINT_STDERR",
        message: "FUNC_PRINT_STDERR: use typed diagnostics instead of eprint! or eprintln!",
        effect: FindingEffect::Reject,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::Engine(AstEngine::detect_print_stderr),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "FUNC_WILDCARD_IMPORT",
        message: "FUNC_WILDCARD_IMPORT: replace wildcard imports with explicit imports",
        effect: FindingEffect::Informational,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::Engine(AstEngine::detect_wildcard_import),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "FUNC_UNWRAP_OR",
        message: "FUNC_UNWRAP_OR: replace unwrap_or defaults with explicit typed recovery",
        effect: FindingEffect::Reject,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::Engine(AstEngine::detect_unwrap_or),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "FUNC_RESULT_STRING",
        message: "FUNC_RESULT_STRING: use typed error types instead of String for Result error variants",
        effect: FindingEffect::Reject,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::Engine(AstEngine::detect_result_string),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "BYPASS_ALLOW_ATTR",
        message: "BYPASS_ALLOW_ATTR: remove item-level #[allow(...)] suppression",
        effect: FindingEffect::Reject,
        repair: RepairKind::RemoveAllowAttribute,
        detect: Detector::Engine(AstEngine::detect_allow_attr),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "BYPASS_EXPECT_ATTR",
        message: "BYPASS_EXPECT_ATTR: remove item-level #[expect(...)] suppression",
        effect: FindingEffect::Reject,
        repair: RepairKind::RemoveAllowAttribute,
        detect: Detector::Engine(AstEngine::detect_expect_attr),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "BYPASS_CFG_ATTR_ALLOW",
        message: "BYPASS_CFG_ATTR_ALLOW: remove cfg_attr allow(...) suppression",
        effect: FindingEffect::Reject,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::Engine(AstEngine::detect_cfg_attr_allow),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "BYPASS_CRATE_ALLOW",
        message: "BYPASS_CRATE_ALLOW: remove crate-level #![allow(...)] suppression",
        effect: FindingEffect::Reject,
        repair: RepairKind::RemoveAllowAttribute,
        detect: Detector::Engine(AstEngine::detect_crate_allow),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "BYPASS_CRATE_EXPECT",
        message: "BYPASS_CRATE_EXPECT: remove crate-level #![expect(...)] suppression",
        effect: FindingEffect::Reject,
        repair: RepairKind::RemoveAllowAttribute,
        detect: Detector::Engine(AstEngine::detect_crate_expect),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "BYPASS_INLINE_SUPPRESSION",
        message: "BYPASS_INLINE_SUPPRESSION: remove ast-grep-ignore or sg-ignore inline suppression",
        effect: FindingEffect::Reject,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::String(detectors::detect_inline_suppression),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "BYPASS_GENERATED_INCLUDE",
        message: "BYPASS_GENERATED_INCLUDE: do not include generated code through OUT_DIR",
        effect: FindingEffect::Reject,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::Engine(AstEngine::detect_generated_include),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "ARCHITECTURE_IMPORT_CORE_INFRA",
        message: "ARCHITECTURE_IMPORT_CORE_INFRA: core/domain code must not import infrastructure crates",
        effect: FindingEffect::Reject,
        repair: RepairKind::ReplaceDependency,
        detect: Detector::String(detectors::detect_core_infra_import),
        scope: RuleScope::CoreDomainRust,
    },
    RuleDef {
        id: "ARCHITECTURE_IMPORT_CORE_FS",
        message: "ARCHITECTURE_IMPORT_CORE_FS: core/domain code must use typed ports instead of direct filesystem, environment, or network I/O",
        effect: FindingEffect::Reject,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::String(detectors::detect_core_fs_import),
        scope: RuleScope::CoreDomainRust,
    },
    RuleDef {
        id: "ARCHITECTURE_IMPORT_CORE_TIME",
        message: "ARCHITECTURE_IMPORT_CORE_TIME: core/domain code must accept time through an injected clock port",
        effect: FindingEffect::Reject,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::String(detectors::detect_core_time_import),
        scope: RuleScope::CoreDomainRust,
    },
    RuleDef {
        id: "ARCHITECTURE_IMPORT_CORE_RANDOM",
        message: "ARCHITECTURE_IMPORT_CORE_RANDOM: core/domain code must accept entropy through an injected random source",
        effect: FindingEffect::Reject,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::String(detectors::detect_core_random_import),
        scope: RuleScope::CoreDomainRust,
    },
];
