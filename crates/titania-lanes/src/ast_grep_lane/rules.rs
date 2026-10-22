#[cfg(test)]
mod detector_tests;
mod detectors;
mod filters;

use titania_core::{FindingEffect, RepairHint, WorkspacePath};

use super::{
    engine::AstEngine,
    span::{FILE_LEVEL_SITE, MatchSite},
};

type EngineDetector = fn(&AstEngine) -> Vec<MatchSite>;

#[derive(Debug, Clone, Copy)]
pub(super) struct RuleDef {
    pub(super) id: &'static str,
    pub(super) message: &'static str,
    pub(super) effect: FindingEffect,
    pub(super) repair: RepairKind,
    scope: RuleScope,
    pub(super) detect: Detector,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Detector {
    Engine(EngineDetector),
    String(fn(&str) -> bool),
}

impl Detector {
    pub(super) fn run(self, engine: &AstEngine, source: &str) -> Vec<MatchSite> {
        match self {
            Self::Engine(detect) => detect(engine),
            Self::String(detect) => detect(source).then_some(FILE_LEVEL_SITE).into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum RepairKind {
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
        id: "BOUNDED_EXECUTION_UNBOUNDED_LOOP",
        message: "BOUNDED_EXECUTION_UNBOUNDED_LOOP: unbounded retry requires a named bound with cited evidence",
        effect: FindingEffect::Reject,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::Engine(AstEngine::detect_unbounded_loop),
        scope: RuleScope::ProductionRust,
    },
    RuleDef {
        id: "BOUNDED_EXECUTION_UNBOUNDED_ITERATOR",
        message: "BOUNDED_EXECUTION_UNBOUNDED_ITERATOR: unbounded iterator source requires a named bound with cited evidence",
        effect: FindingEffect::Reject,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::Engine(AstEngine::detect_unbounded_iterator),
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
        detect: Detector::Engine(AstEngine::detect_inline_suppression),
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
    RuleDef {
        id: "ARCHITECTURE_IMPORT_CORE_PROCESS",
        message: "ARCHITECTURE_IMPORT_CORE_PROCESS: core/domain code must delegate process spawning to the shell through a typed port",
        effect: FindingEffect::Reject,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::String(detectors::detect_core_process_import),
        scope: RuleScope::CoreDomainRust,
    },
    RuleDef {
        id: "ARCHITECTURE_EFFECT_CORE_ASYNC",
        message: "ARCHITECTURE_EFFECT_CORE_ASYNC: core/domain code must stay synchronous; async execution belongs to the shell",
        effect: FindingEffect::Reject,
        repair: RepairKind::ReplaceDependency,
        detect: Detector::String(detectors::detect_core_async_effect),
        scope: RuleScope::CoreDomainRust,
    },
    RuleDef {
        id: "ARCHITECTURE_EFFECT_CORE_STATE",
        message: "ARCHITECTURE_EFFECT_CORE_STATE: core/domain code must not use ambient mutable state; pass explicit values through typed ports",
        effect: FindingEffect::Reject,
        repair: RepairKind::RequiresHumanReview,
        detect: Detector::String(detectors::detect_core_state_effect),
        scope: RuleScope::CoreDomainRust,
    },
];
