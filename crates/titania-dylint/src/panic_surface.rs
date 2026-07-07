//! Panic-surface Dylint rules replacing brittle Rust text scans.

use rustc_ast::{Item, ItemKind, MacCall, MetaItemInner};
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext, LintStore};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::{Span, sym};

declare_lint! {
    /// Detects `panic!` in production Rust.
    pub HOLZMAN_PANIC_PANIC,
    Forbid,
    "HOLZMAN_PANIC_PANIC: replace panic! with typed failure"
}

declare_lint! {
    /// Detects production `assert!` panic paths.
    pub HOLZMAN_PANIC_ASSERT,
    Forbid,
    "HOLZMAN_PANIC_ASSERT: replace assert! with typed validation"
}

declare_lint! {
    /// Detects production `assert_eq!` panic paths.
    pub HOLZMAN_PANIC_ASSERT_EQ,
    Forbid,
    "HOLZMAN_PANIC_ASSERT_EQ: replace assert_eq! with typed validation"
}

declare_lint! {
    /// Detects production `assert_ne!` panic paths.
    pub HOLZMAN_PANIC_ASSERT_NE,
    Forbid,
    "HOLZMAN_PANIC_ASSERT_NE: replace assert_ne! with typed validation"
}

declare_lint! {
    /// Detects production `todo!` placeholders.
    pub HOLZMAN_PANIC_TODO,
    Forbid,
    "HOLZMAN_PANIC_TODO: replace todo! with implemented behavior"
}

declare_lint! {
    /// Detects production `unimplemented!` placeholders.
    pub HOLZMAN_PANIC_UNIMPLEMENTED,
    Forbid,
    "HOLZMAN_PANIC_UNIMPLEMENTED: replace unimplemented! with implemented behavior"
}

declare_lint! {
    /// Detects production `unreachable!` panic paths.
    pub HOLZMAN_PANIC_UNREACHABLE,
    Forbid,
    "HOLZMAN_PANIC_UNREACHABLE: replace unreachable! with typed impossible-state modeling"
}

declare_lint! {
    /// Detects production `dbg!` debug surface.
    pub HOLZMAN_PANIC_DBG,
    Forbid,
    "HOLZMAN_PANIC_DBG: remove dbg! debug surface"
}

#[derive(Default)]
struct PanicSurface {
    test_module_depth: usize,
}

impl_lint_pass!(PanicSurface => [
    HOLZMAN_PANIC_PANIC,
    HOLZMAN_PANIC_ASSERT,
    HOLZMAN_PANIC_ASSERT_EQ,
    HOLZMAN_PANIC_ASSERT_NE,
    HOLZMAN_PANIC_TODO,
    HOLZMAN_PANIC_UNIMPLEMENTED,
    HOLZMAN_PANIC_UNREACHABLE,
    HOLZMAN_PANIC_DBG,
]);

impl EarlyLintPass for PanicSurface {
    fn check_item(&mut self, _cx: &EarlyContext<'_>, item: &Item) {
        self.test_module_depth =
            self.test_module_depth.saturating_add(usize::from(is_cfg_test_module(item)));
    }

    fn check_item_post(&mut self, _cx: &EarlyContext<'_>, item: &Item) {
        self.test_module_depth =
            self.test_module_depth.saturating_sub(usize::from(is_cfg_test_module(item)));
    }

    fn check_mac(&mut self, cx: &EarlyContext<'_>, mac: &MacCall) {
        let _emitted =
            outside_test_module(self.test_module_depth).then(|| emit_macro_policy(cx, mac));
    }
}

/// Register panic-surface lints and pass.
pub(crate) fn register(lint_store: &mut LintStore) {
    lint_store.register_lints(&[
        HOLZMAN_PANIC_PANIC,
        HOLZMAN_PANIC_ASSERT,
        HOLZMAN_PANIC_ASSERT_EQ,
        HOLZMAN_PANIC_ASSERT_NE,
        HOLZMAN_PANIC_TODO,
        HOLZMAN_PANIC_UNIMPLEMENTED,
        HOLZMAN_PANIC_UNREACHABLE,
        HOLZMAN_PANIC_DBG,
    ]);
    lint_store.register_pre_expansion_pass(|| -> Box<dyn EarlyLintPass> {
        Box::new(PanicSurface::default())
    });
}

fn is_cfg_test_module(item: &Item) -> bool {
    matches!(item.kind, ItemKind::Mod(_, _, _)) && item.attrs.iter().any(is_cfg_test_attr)
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

const fn outside_test_module(test_module_depth: usize) -> bool {
    test_module_depth == 0
}

fn emit_macro_policy(cx: &EarlyContext<'_>, mac: &MacCall) {
    match macro_name(mac).as_deref() {
        Some("panic") => emit_panic(cx, mac.span()),
        Some("assert") => emit_assert(cx, mac.span()),
        Some("assert_eq") => emit_assert_eq(cx, mac.span()),
        Some("assert_ne") => emit_assert_ne(cx, mac.span()),
        Some("todo") => emit_todo(cx, mac.span()),
        Some("unimplemented") => emit_unimplemented(cx, mac.span()),
        Some("unreachable") => emit_unreachable(cx, mac.span()),
        Some("dbg") => emit_dbg(cx, mac.span()),
        Some(_) | None => (),
    }
}

fn macro_name(mac: &MacCall) -> Option<String> {
    mac.path.segments.iter().next_back().map(|segment| segment.ident.as_str().to_string())
}

fn emit_panic(cx: &EarlyContext<'_>, span: Span) {
    cx.emit_span_lint(
        HOLZMAN_PANIC_PANIC,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message("HOLZMAN_PANIC_PANIC: replace panic! with typed failure")
                .help(
                    "return an explicit error or model the invariant as an unconstructable state",
                );
        }),
    );
}

fn emit_assert(cx: &EarlyContext<'_>, span: Span) {
    cx.emit_span_lint(
        HOLZMAN_PANIC_ASSERT,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message("HOLZMAN_PANIC_ASSERT: replace assert! with typed validation")
                .help("validate at construction boundaries and return typed errors");
        }),
    );
}

fn emit_assert_eq(cx: &EarlyContext<'_>, span: Span) {
    cx.emit_span_lint(
        HOLZMAN_PANIC_ASSERT_EQ,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message(
                    "HOLZMAN_PANIC_ASSERT_EQ: replace assert_eq! with typed validation",
                )
                .help("compare explicitly and return a typed domain error");
        }),
    );
}

fn emit_assert_ne(cx: &EarlyContext<'_>, span: Span) {
    cx.emit_span_lint(
        HOLZMAN_PANIC_ASSERT_NE,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message(
                    "HOLZMAN_PANIC_ASSERT_NE: replace assert_ne! with typed validation",
                )
                .help("compare explicitly and return a typed domain error");
        }),
    );
}

fn emit_todo(cx: &EarlyContext<'_>, span: Span) {
    cx.emit_span_lint(
        HOLZMAN_PANIC_TODO,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message("HOLZMAN_PANIC_TODO: replace todo! with implemented behavior")
                .help("finish the branch or return an explicit unsupported-operation error");
        }),
    );
}

fn emit_unimplemented(cx: &EarlyContext<'_>, span: Span) {
    cx.emit_span_lint(
        HOLZMAN_PANIC_UNIMPLEMENTED,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message(
                    "HOLZMAN_PANIC_UNIMPLEMENTED: replace unimplemented! with implemented behavior",
                )
                .help("finish the branch or return an explicit unsupported-operation error");
        }),
    );
}

fn emit_unreachable(cx: &EarlyContext<'_>, span: Span) {
    cx.emit_span_lint(
        HOLZMAN_PANIC_UNREACHABLE,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message("HOLZMAN_PANIC_UNREACHABLE: replace unreachable! with typed impossible-state modeling")
                .help("encode impossible states in the type system or return a typed invariant error");
        }),
    );
}

fn emit_dbg(cx: &EarlyContext<'_>, span: Span) {
    cx.emit_span_lint(
        HOLZMAN_PANIC_DBG,
        span,
        DiagDecorator(|diag| {
            let _decorated = diag
                .primary_message("HOLZMAN_PANIC_DBG: remove dbg! debug surface")
                .help("use structured tracing or typed diagnostics at the shell boundary");
        }),
    );
}
