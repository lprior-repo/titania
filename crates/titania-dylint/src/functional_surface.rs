use rustc_ast::{
    Expr as AstExpr, ExprKind as AstExprKind, Item as AstItem, ItemKind as AstItemKind,
    MetaItemInner,
};
use rustc_errors::DiagDecorator;
use rustc_hir::{
    Expr as HirExpr, ExprKind as HirExprKind, QPath,
    def::{DefKind, Res},
};
use rustc_lint::{EarlyContext, EarlyLintPass, LateContext, LateLintPass, LintContext, LintStore};
use rustc_middle::ty::{self, TyKind};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::{Span, def_id::DefId, sym};

declare_lint! {
    pub FUNC_UNWRAP_USED,
    Forbid,
    "FUNC_UNWRAP_USED: replace unwrap with explicit typed error handling"
}

declare_lint! {
    pub FUNC_EXPECT_USED,
    Forbid,
    "FUNC_EXPECT_USED: replace expect with explicit typed error handling"
}

declare_lint! {
    pub FUNC_UNWRAP_OR,
    Forbid,
    "FUNC_UNWRAP_OR: replace unwrap_or defaults with explicit typed recovery"
}

declare_lint! {
    pub BOUNDED_EXECUTION_UNBOUNDED_LOOP,
    Forbid,
    "BOUNDED_EXECUTION_UNBOUNDED_LOOP: unbounded retry requires a named bound with cited evidence"
}

declare_lint! {
    pub BOUNDED_EXECUTION_UNBOUNDED_ITERATOR,
    Forbid,
    "BOUNDED_EXECUTION_UNBOUNDED_ITERATOR: unbounded iterator source requires a named bound with cited evidence"
}

declare_lint! {
    pub FUNC_RESULT_OK,
    Forbid,
    "FUNC_RESULT_OK: replace ok() erasure with explicit typed error handling"
}

#[derive(Default)]
struct FunctionalSurface {
    test_module_depth: usize,
}

impl_lint_pass!(FunctionalSurface => [
    FUNC_UNWRAP_USED,
    FUNC_EXPECT_USED,
    FUNC_UNWRAP_OR,
    FUNC_RESULT_OK,
    BOUNDED_EXECUTION_UNBOUNDED_LOOP,
    BOUNDED_EXECUTION_UNBOUNDED_ITERATOR,
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
            outside_test_module(self.test_module_depth).then(|| emit_bounded_policy(cx, expr));
    }
}

impl<'tcx> LateLintPass<'tcx> for FunctionalSurface {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx HirExpr<'tcx>) {
        match &expr.kind {
            HirExprKind::MethodCall(..) => emit_all_method_policies(cx, expr),
            HirExprKind::Call(func, _) => emit_all_call_policies(cx, expr, func),
            HirExprKind::Path(qpath) => emit_path_value_policy(cx, expr, qpath),
            _ => (),
        }
    }
}

fn emit_all_method_policies(cx: &LateContext<'_>, expr: &HirExpr<'_>) {
    emit_method_policy(cx, expr);
    emit_iterator_method_policy(cx, expr);
}

fn emit_all_call_policies(cx: &LateContext<'_>, expr: &HirExpr<'_>, func: &HirExpr<'_>) {
    emit_path_call_policy(cx, expr, func);
    emit_iterator_call_policy(cx, expr, func);
}

pub(crate) fn register(lint_store: &mut LintStore) {
    lint_store.register_lints(&[
        FUNC_UNWRAP_USED,
        FUNC_EXPECT_USED,
        FUNC_UNWRAP_OR,
        FUNC_RESULT_OK,
        BOUNDED_EXECUTION_UNBOUNDED_LOOP,
        BOUNDED_EXECUTION_UNBOUNDED_ITERATOR,
    ]);
    lint_store.register_early_pass(|| Box::new(FunctionalSurface::default()));
    lint_store.register_late_pass(|_| Box::new(FunctionalSurface::default()));
}

fn emit_method_policy(cx: &LateContext<'_>, expr: &HirExpr<'_>) {
    let HirExprKind::MethodCall(segment, _, _, _) = &expr.kind else {
        return;
    };
    let name = segment.ident.name.as_str();
    let admitted = match method_policy(name) {
        MethodPolicy::None => false,
        MethodPolicy::Ok => !matches!(identify_result_method(cx, expr), MethodIdentity::Other),
        MethodPolicy::Unwrap | MethodPolicy::Expect | MethodPolicy::UnwrapOr => {
            is_core_or_unknown_method(cx, expr)
        }
    };
    if !admitted {
        return;
    }
    emit_named_policy(cx, actionable_span(segment.ident.span), name);
}

fn emit_path_call_policy(cx: &LateContext<'_>, expr: &HirExpr<'_>, func: &HirExpr<'_>) {
    let HirExprKind::Path(qpath) = &func.kind else {
        return;
    };
    let Res::Def(DefKind::AssocFn, def_id) = cx.qpath_res(qpath, func.hir_id) else {
        return;
    };
    let Some(name) = qpath_final_name(qpath) else {
        return;
    };
    let admitted = match method_policy(name) {
        MethodPolicy::None => false,
        MethodPolicy::Ok => is_core_result_ok_method(cx, def_id),
        MethodPolicy::Unwrap | MethodPolicy::Expect | MethodPolicy::UnwrapOr => {
            is_core_option_result_method(cx, def_id)
        }
    };
    if !admitted {
        return;
    }
    emit_named_policy(cx, actionable_span(expr.span), name);
}

fn emit_path_value_policy(cx: &LateContext<'_>, expr: &HirExpr<'_>, qpath: &QPath<'_>) {
    let Res::Def(DefKind::AssocFn, def_id) = cx.qpath_res(qpath, expr.hir_id) else {
        return;
    };
    let Some(name) = qpath_final_name(qpath) else {
        return;
    };
    if !matches!(method_policy(name), MethodPolicy::Ok) {
        return;
    }
    if !is_core_result_ok_method(cx, def_id) {
        return;
    }
    if is_call_callee(cx, expr) {
        return;
    }
    emit_named_policy(cx, actionable_span(expr.span), name);
}

fn is_call_callee(cx: &LateContext<'_>, expr: &HirExpr<'_>) -> bool {
    let parent_id = cx.tcx.parent_hir_id(expr.hir_id);
    let parent = cx.tcx.hir_node(parent_id);
    match parent {
        rustc_hir::Node::Expr(parent_expr) => match &parent_expr.kind {
            HirExprKind::Call(func, _) => func.hir_id == expr.hir_id,
            _ => false,
        },
        _ => false,
    }
}

fn emit_named_policy(cx: &LateContext<'_>, span: Span, name: &str) {
    match method_policy(name) {
        MethodPolicy::None => (),
        MethodPolicy::Unwrap => emit_unwrap(cx, span),
        MethodPolicy::Expect => emit_expect(cx, span),
        MethodPolicy::UnwrapOr => emit_unwrap_or(cx, span),
        MethodPolicy::Ok => emit_result_ok(cx, span),
    }
}

enum MethodIdentity {
    CoreStd,
    Other,
    Unknown,
}

fn is_core_or_unknown_method(cx: &LateContext<'_>, expr: &HirExpr<'_>) -> bool {
    match identify_method(cx, expr) {
        MethodIdentity::CoreStd | MethodIdentity::Unknown => true,
        MethodIdentity::Other => false,
    }
}

fn identify_method(cx: &LateContext<'_>, expr: &HirExpr<'_>) -> MethodIdentity {
    let Some(typeck) = cx.maybe_typeck_results() else {
        return MethodIdentity::Unknown;
    };
    let Some((DefKind::AssocFn, def_id)) = typeck.type_dependent_def(expr.hir_id) else {
        return MethodIdentity::Unknown;
    };
    if is_core_option_result_method(cx, def_id) {
        MethodIdentity::CoreStd
    } else {
        MethodIdentity::Other
    }
}

fn identify_result_method(cx: &LateContext<'_>, expr: &HirExpr<'_>) -> MethodIdentity {
    let HirExprKind::MethodCall(..) = &expr.kind else {
        return MethodIdentity::Unknown;
    };
    let Some(typeck) = cx.maybe_typeck_results() else {
        return MethodIdentity::Unknown;
    };
    let Some((DefKind::AssocFn, def_id)) = typeck.type_dependent_def(expr.hir_id) else {
        return MethodIdentity::Unknown;
    };
    if is_core_result_ok_method(cx, def_id) {
        MethodIdentity::CoreStd
    } else {
        MethodIdentity::Other
    }
}

fn is_core_result_ok_method(cx: &LateContext<'_>, def_id: DefId) -> bool {
    let Some(assoc) = cx.tcx.opt_associated_item(def_id) else {
        return false;
    };
    if assoc.container != ty::AssocContainer::InherentImpl {
        return false;
    }
    let Some(impl_id) = cx.tcx.opt_parent(def_id) else {
        return false;
    };
    let self_ty = cx.tcx.type_of(impl_id).skip_binder();
    let TyKind::Adt(adt, _) = *self_ty.kind() else {
        return false;
    };
    if !is_core_result_adt(cx, adt.did()) {
        return false;
    }
    assoc.name().as_str() == "ok"
}

fn is_core_result_adt(cx: &LateContext<'_>, adt_id: DefId) -> bool {
    if adt_id.is_local() {
        return false;
    }
    if cx.tcx.crate_name(adt_id.krate).as_str() != "core" {
        return false;
    }
    matches!(cx.tcx.def_path_str(adt_id).as_str(), "core::result::Result" | "std::result::Result")
}

fn is_core_option_result_method(cx: &LateContext<'_>, def_id: DefId) -> bool {
    let Some(assoc) = cx.tcx.opt_associated_item(def_id) else {
        return false;
    };
    if assoc.container != ty::AssocContainer::InherentImpl {
        return false;
    }
    let Some(impl_id) = cx.tcx.opt_parent(def_id) else {
        return false;
    };
    let self_ty = cx.tcx.type_of(impl_id).skip_binder();
    let TyKind::Adt(adt, _) = *self_ty.kind() else {
        return false;
    };
    let adt_id = adt.did();
    if adt_id.is_local() {
        return false;
    }
    if cx.tcx.crate_name(adt_id.krate).as_str() != "core" {
        return false;
    }
    matches!(
        cx.tcx.def_path_str(adt_id).as_str(),
        "core::option::Option"
            | "std::option::Option"
            | "core::result::Result"
            | "std::result::Result"
    )
}

fn qpath_final_name<'a>(qpath: &'a QPath<'a>) -> Option<&'a str> {
    match qpath {
        QPath::Resolved(_, path) => path.segments.last().map(|segment| segment.ident.name.as_str()),
        QPath::TypeRelative(_, segment) => Some(segment.ident.name.as_str()),
    }
}

fn actionable_span(span: Span) -> Span {
    let unfolded = std::iter::successors(Some(span), |current| {
        current.from_expansion().then(|| current.ctxt().outer_expn_data().call_site)
    })
    .last();
    unfolded.map_or(span, |found| found)
}

fn emit_bounded_policy(cx: &EarlyContext<'_>, expr: &AstExpr) {
    match &expr.kind {
        AstExprKind::While(condition, _, _) => emit_condition_loop(cx, expr.span, condition),
        AstExprKind::Loop(_, _, _) => emit_unbounded_loop(cx, expr.span),
        _ => (),
    }
}

fn emit_condition_loop(cx: &EarlyContext<'_>, span: Span, condition: &AstExpr) {
    if is_let_condition(condition) {
        return;
    }
    emit_unbounded_loop(cx, span);
}

const fn is_let_condition(condition: &AstExpr) -> bool {
    matches!(condition.kind, AstExprKind::Let(..))
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
    Ok,
}

fn method_policy(name: &str) -> MethodPolicy {
    match name {
        "unwrap" => MethodPolicy::Unwrap,
        "expect" => MethodPolicy::Expect,
        "unwrap_or" | "unwrap_or_else" | "unwrap_or_default" => MethodPolicy::UnwrapOr,
        "ok" => MethodPolicy::Ok,
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

fn emit_result_ok(cx: &LateContext<'_>, span: Span) {
    cx.emit_span_lint(
        FUNC_RESULT_OK,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message(
                    "FUNC_RESULT_OK: replace ok() erasure with explicit typed error handling",
                )
                .help("match on the result, propagate with ?, or map the error instead of discarding it into Option");
        }),
    );
}

fn emit_unbounded_loop(cx: &EarlyContext<'_>, span: Span) {
    cx.emit_span_lint(
        BOUNDED_EXECUTION_UNBOUNDED_LOOP,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message("BOUNDED_EXECUTION_UNBOUNDED_LOOP: unbounded retry requires a named bound with cited evidence")
                .help("cite Bound::FiniteInput, Bound::Capped with a regression test, or Bound::ProtocolTerminal; `for` over finite input and `while let` are permitted");
        }),
    );
}

fn emit_unbounded_iterator(cx: &LateContext<'_>, span: Span) {
    cx.emit_span_lint(
        BOUNDED_EXECUTION_UNBOUNDED_ITERATOR,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message("BOUNDED_EXECUTION_UNBOUNDED_ITERATOR: unbounded iterator source requires a named bound with cited evidence")
                .help("bound with take/take_while/nth or a checked-in termination claim; combinator syntax never establishes boundedness");
        }),
    );
}

fn emit_iterator_method_policy(cx: &LateContext<'_>, expr: &HirExpr<'_>) {
    let HirExprKind::MethodCall(segment, _, args, _) = &expr.kind else {
        return;
    };
    if segment.ident.name.as_str() != "cycle" || !args.is_empty() {
        return;
    }
    emit_unbounded_iterator(cx, actionable_span(segment.ident.span));
}

fn emit_iterator_call_policy(cx: &LateContext<'_>, expr: &HirExpr<'_>, func: &HirExpr<'_>) {
    let HirExprKind::Path(qpath) = &func.kind else {
        return;
    };
    let Some(name) = qpath_final_name(qpath) else {
        return;
    };
    if !is_unbounded_iterator_name(name) {
        return;
    }
    if !is_core_iter_fn(cx, func) {
        return;
    }
    emit_unbounded_iterator(cx, actionable_span(expr.span));
}

fn is_unbounded_iterator_name(name: &str) -> bool {
    matches!(name, "repeat" | "repeat_with" | "successors" | "from_fn")
}

fn is_core_iter_fn(cx: &LateContext<'_>, func: &HirExpr<'_>) -> bool {
    let HirExprKind::Path(qpath) = &func.kind else {
        return false;
    };
    let Res::Def(DefKind::Fn, def_id) = cx.qpath_res(qpath, func.hir_id) else {
        return false;
    };
    if def_id.is_local() {
        return false;
    }
    if !is_core_or_std_crate(cx, def_id) {
        return false;
    }
    cx.tcx.def_path_str(def_id).contains("iter::")
}

fn is_core_or_std_crate(cx: &LateContext<'_>, def_id: DefId) -> bool {
    matches!(cx.tcx.crate_name(def_id.krate).as_str(), "core" | "std")
}
