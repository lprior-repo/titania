//! Functional-surface Dylint rules replacing brittle Rust text scans.

use rustc_ast::{
    Expr as AstExpr, ExprKind as AstExprKind, Item as AstItem, ItemKind as AstItemKind,
    MetaItemInner,
};
use rustc_errors::DiagDecorator;
use rustc_hir::{Expr as HirExpr, ExprKind as HirExprKind, PathSegment};
use rustc_lint::{EarlyContext, EarlyLintPass, LateContext, LateLintPass, LintContext, LintStore};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::{Span, sym};

declare_lint! {
    /// Detects `.unwrap()` method calls in production Rust.
    pub FUNC_UNWRAP_USED,
    Forbid,
    "FUNC_UNWRAP_USED: replace unwrap with explicit typed error handling"
}

declare_lint! {
    /// Detects `.expect(...)` method calls in production Rust.
    pub FUNC_EXPECT_USED,
    Forbid,
    "FUNC_EXPECT_USED: replace expect with explicit typed error handling"
}

declare_lint! {
    /// Detects `.unwrap_or*` defaults in production Rust.
    pub FUNC_UNWRAP_OR,
    Forbid,
    "FUNC_UNWRAP_OR: replace unwrap_or defaults with explicit typed recovery"
}

declare_lint! {
    /// Detects `for` loops in production Rust.
    pub FUNC_LOOPS_FOR,
    Forbid,
    "FUNC_LOOPS_FOR: replace imperative for loops with iterator pipelines"
}

declare_lint! {
    /// Detects `while` loops in production Rust.
    pub FUNC_LOOPS_WHILE,
    Forbid,
    "FUNC_LOOPS_WHILE: replace while loops with bounded iterator or state-machine transitions"
}

declare_lint! {
    /// Detects open-ended `loop` blocks in production Rust.
    pub FUNC_LOOPS_LOOP,
    Forbid,
    "FUNC_LOOPS_LOOP: replace open-ended loop blocks with explicit bounded control flow"
}

#[derive(Default)]
struct FunctionalSurface {
    test_module_depth: usize,
}

impl_lint_pass!(FunctionalSurface => [
    FUNC_UNWRAP_USED,
    FUNC_EXPECT_USED,
    FUNC_UNWRAP_OR,
    FUNC_LOOPS_FOR,
    FUNC_LOOPS_WHILE,
    FUNC_LOOPS_LOOP,
]);

impl EarlyLintPass for FunctionalSurface {
    fn check_item(&mut self, _cx: &EarlyContext<'_>, item: &AstItem) {
        self.test_module_depth =
            self.test_module_depth.saturating_add(usize::from(is_cfg_test_module(item)));
    }

    fn check_item_post(&mut self, _cx: &EarlyContext<'_>, item: &AstItem) {
        self.test_module_depth =
            self.test_module_depth.saturating_sub(usize::from(is_cfg_test_module(item)));
    }

    fn check_expr(&mut self, cx: &EarlyContext<'_>, expr: &AstExpr) {
        let _emitted =
            outside_test_module(self.test_module_depth).then(|| emit_loop_policy(cx, expr));
    }
}

impl<'tcx> LateLintPass<'tcx> for FunctionalSurface {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx HirExpr<'tcx>) {
        emit_method_policy(cx, expr);
    }
}

/// Register functional-surface lints and pass.
pub(crate) fn register(lint_store: &mut LintStore) {
    lint_store.register_lints(&[
        FUNC_UNWRAP_USED,
        FUNC_EXPECT_USED,
        FUNC_UNWRAP_OR,
        FUNC_LOOPS_FOR,
        FUNC_LOOPS_WHILE,
        FUNC_LOOPS_LOOP,
    ]);
    lint_store.register_early_pass(|| Box::new(FunctionalSurface::default()));
    lint_store.register_late_pass(|_| Box::new(FunctionalSurface::default()));
}

fn emit_method_policy(cx: &LateContext<'_>, expr: &HirExpr<'_>) {
    let HirExprKind::MethodCall(segment, _, _, _) = expr.kind else {
        return;
    };
    if segment.ident.span.from_expansion() {
        return;
    }
    match method_policy(segment) {
        MethodPolicy::None => (),
        MethodPolicy::Unwrap => emit_unwrap(cx, segment.ident.span),
        MethodPolicy::Expect => emit_expect(cx, segment.ident.span),
        MethodPolicy::UnwrapOr => emit_unwrap_or(cx, segment.ident.span),
    }
}

fn emit_loop_policy(cx: &EarlyContext<'_>, expr: &AstExpr) {
    match &expr.kind {
        AstExprKind::ForLoop { .. } => emit_for_loop(cx, expr.span),
        AstExprKind::While(_, _, _) => emit_while_loop(cx, expr.span),
        AstExprKind::Loop(_, _, _) => emit_loop_block(cx, expr.span),
        _ => (),
    }
}

const fn outside_test_module(test_module_depth: usize) -> bool {
    test_module_depth == 0
}

fn is_cfg_test_module(item: &AstItem) -> bool {
    matches!(item.kind, AstItemKind::Mod(_, _, _)) && item.attrs.iter().any(is_cfg_test_attr)
}

fn is_cfg_test_attr(attr: &rustc_ast::Attribute) -> bool {
    attr.has_name(sym::cfg) && attr.meta_item_list().into_iter().flatten().any(matches_cfg_test)
}

fn matches_cfg_test(inner: MetaItemInner) -> bool {
    match inner {
        MetaItemInner::MetaItem(meta) => meta
            .path
            .segments
            .iter()
            .next_back()
            .is_some_and(|segment| segment.ident.name == sym::test),
        MetaItemInner::Lit(_) => false,
    }
}

#[derive(Clone, Copy)]
enum MethodPolicy {
    None,
    Unwrap,
    Expect,
    UnwrapOr,
}

fn method_policy(segment: &PathSegment<'_>) -> MethodPolicy {
    match segment.ident.name.as_str() {
        "unwrap" => MethodPolicy::Unwrap,
        "expect" => MethodPolicy::Expect,
        "unwrap_or" | "unwrap_or_else" | "unwrap_or_default" => MethodPolicy::UnwrapOr,
        _ => MethodPolicy::None,
    }
}

fn emit_unwrap(cx: &LateContext<'_>, span: Span) {
    cx.emit_span_lint(
        FUNC_UNWRAP_USED,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message("FUNC_UNWRAP_USED: replace unwrap with explicit typed error handling")
                .help("return a typed error, match explicitly, or make the invalid state unrepresentable");
        }),
    );
}

fn emit_expect(cx: &LateContext<'_>, span: Span) {
    cx.emit_span_lint(
        FUNC_EXPECT_USED,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message(
                    "FUNC_EXPECT_USED: replace expect with explicit typed error handling",
                )
                .help("carry context through the error type instead of panicking with a string");
        }),
    );
}

fn emit_unwrap_or(cx: &LateContext<'_>, span: Span) {
    cx.emit_span_lint(
        FUNC_UNWRAP_OR,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message(
                    "FUNC_UNWRAP_OR: replace unwrap_or defaults with explicit typed recovery",
                )
                .help("model the default as a domain decision instead of hiding invalid state");
        }),
    );
}

fn emit_for_loop(cx: &EarlyContext<'_>, span: Span) {
    cx.emit_span_lint(
        FUNC_LOOPS_FOR,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message(
                    "FUNC_LOOPS_FOR: replace imperative for loops with iterator pipelines",
                )
                .help("use map/filter/fold/try_fold/try_for_each with explicit bounds and errors");
        }),
    );
}

fn emit_while_loop(cx: &EarlyContext<'_>, span: Span) {
    cx.emit_span_lint(
        FUNC_LOOPS_WHILE,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message("FUNC_LOOPS_WHILE: replace while loops with bounded iterator or state-machine transitions")
                .help("encode the bound in data or use an explicit transition function");
        }),
    );
}

fn emit_loop_block(cx: &EarlyContext<'_>, span: Span) {
    cx.emit_span_lint(
        FUNC_LOOPS_LOOP,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message("FUNC_LOOPS_LOOP: replace open-ended loop blocks with explicit bounded control flow")
                .help("use a bounded iterator, transition table, or cancellation-aware protocol");
        }),
    );
}
