//! `titania_dylint` - a Dylint plugin cdylib for the Titania CI lane.
//!
//! This crate contains Titania's custom Dylint rules for v1 strict-AI bypass
//! detection. The only unsafe surface is Dylint's required ABI export
//! attribute on `dylint_version` and `register_lints`; both exports carry
//! local `#[expect(unsafe_code)]` reasons while the crate lint remains denied.

#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

use rustc_ast::{AttrStyle, Crate, MetaItem, MetaItemInner};
use rustc_errors::DiagDecorator;
use rustc_hir::{ImplItem, Item, TraitItem};
use rustc_lint::{EarlyContext, EarlyLintPass, LateContext, LateLintPass, LintContext, LintStore};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::{Span, sym};

/// Return the Dylint ABI version string to `cargo-dylint`.
#[expect(
    clippy::option_if_let_else,
    reason = "Dylint ABI bootstrap keeps the impossible CString error explicit"
)]
#[expect(
    unsafe_code,
    reason = "Dylint requires an unsafe no_mangle ABI export for version discovery"
)]
#[unsafe(no_mangle)]
pub extern "C" fn dylint_version() -> *mut std::os::raw::c_char {
    match std::ffi::CString::new(dylint_linting::DYLINT_VERSION) {
        Ok(version) => version.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

declare_lint! {
    /// Detects public items with local `#[allow(...)]` suppressions.
    pub BYPASS_PUB_ALLOW,
    Forbid,
    "BYPASS_PUB_ALLOW: public API item weakens lint policy with an allow attribute"
}

declare_lint! {
    /// Detects public `#[allow(...)]` attributes produced by macro expansion.
    pub BYPASS_ATTR_CONTEXT,
    Forbid,
    "BYPASS_ATTR_CONTEXT: public allow attribute comes from expanded code"
}

declare_lint! {
    /// Detects crate-level downgrades of mandatory Titania lint policy.
    pub BYPASS_REQUIRED_LINT_WEAKENING,
    Forbid,
    "BYPASS_REQUIRED_LINT_WEAKENING: crate-level allow weakens mandatory Titania lint policy"
}

declare_lint! {
    /// Detects `#[allow_internal_unstable(...)]` bypass attributes.
    pub BYPASS_INTERNAL_UNSTABLE,
    Forbid,
    "BYPASS_INTERNAL_UNSTABLE: macro permits unstable internals through expansion"
}

declare_lint! {
    /// Detects `#[allow_internal_unsafe]` bypass attributes.
    pub BYPASS_INTERNAL_UNSAFE,
    Forbid,
    "BYPASS_INTERNAL_UNSAFE: macro permits unsafe internals through expansion"
}

struct PubAllow;

impl_lint_pass!(PubAllow => [BYPASS_PUB_ALLOW, BYPASS_ATTR_CONTEXT]);

impl<'tcx> LateLintPass<'tcx> for PubAllow {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let attrs = cx
            .tcx
            .local_visibility(item.owner_id.def_id)
            .is_public()
            .then(|| cx.tcx.hir_attrs(item.hir_id()));
        emit_pub_allow_attrs(cx, attrs.into_iter().flatten());
    }

    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        let attrs = cx
            .tcx
            .local_visibility(item.owner_id.def_id)
            .is_public()
            .then(|| cx.tcx.hir_attrs(item.hir_id()));
        emit_pub_allow_attrs(cx, attrs.into_iter().flatten());
    }

    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'tcx>) {
        let attrs = cx
            .tcx
            .local_visibility(item.owner_id.def_id)
            .is_public()
            .then(|| cx.tcx.hir_attrs(item.hir_id()));
        emit_pub_allow_attrs(cx, attrs.into_iter().flatten());
    }
}

fn emit_pub_allow_attrs<'a>(
    cx: &LateContext<'_>,
    attrs: impl IntoIterator<Item = &'a rustc_hir::Attribute>,
) {
    attrs.into_iter().filter(|attr| attr.has_name(sym::allow)).for_each(|attr| {
        emit_pub_allow_attr(cx, attr.span());
    });
}

struct RequiredLintWeakening;

impl_lint_pass!(RequiredLintWeakening => [BYPASS_REQUIRED_LINT_WEAKENING]);

impl EarlyLintPass for RequiredLintWeakening {
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        krate
            .attrs
            .iter()
            .filter(|attr| attr.style == AttrStyle::Inner && attr.has_name(sym::allow))
            .flat_map(|attr| lint_args(attr).map(move |lint| (attr.span, lint)))
            .filter(|(_, lint)| is_required_lint(lint))
            .for_each(|(span, lint)| emit_required_lint_weakening(cx, span, &lint));
    }
}

struct InternalEscape;

impl_lint_pass!(InternalEscape => [BYPASS_INTERNAL_UNSTABLE, BYPASS_INTERNAL_UNSAFE]);

impl EarlyLintPass for InternalEscape {
    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &rustc_ast::Attribute) {
        emit_internal_escapes(cx, attr);
    }
}

/// Register Titania's Dylint passes with the rustc lint store.
#[expect(
    clippy::no_mangle_with_rust_abi,
    reason = "Dylint's documented register_lints ABI is a Rust function with no_mangle"
)]
#[expect(
    unsafe_code,
    reason = "Dylint requires an unsafe no_mangle ABI export for lint registration"
)]
#[unsafe(no_mangle)]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut LintStore) {
    dylint_linting::init_config(sess);
    lint_store.register_lints(&[
        BYPASS_PUB_ALLOW,
        BYPASS_ATTR_CONTEXT,
        BYPASS_REQUIRED_LINT_WEAKENING,
        BYPASS_INTERNAL_UNSTABLE,
        BYPASS_INTERNAL_UNSAFE,
    ]);
    lint_store.register_early_pass(|| Box::new(RequiredLintWeakening));
    lint_store.register_early_pass(|| Box::new(InternalEscape));
    lint_store.register_late_pass(|_| Box::new(PubAllow));
}

fn emit_pub_allow_attr(cx: &LateContext<'_>, span: Span) {
    if span.from_expansion() {
        emit_attr_context(cx, span);
    } else {
        emit_pub_allow(cx, span);
    }
}

fn emit_pub_allow(cx: &LateContext<'_>, span: Span) {
    cx.emit_span_lint(
        BYPASS_PUB_ALLOW,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(
                "BYPASS_PUB_ALLOW: public API item weakens lint policy with #[allow(...)]",
            );
            let _ = diag.help("move any justified lint exception to the policy exceptions ledger");
        }),
    );
}

fn emit_attr_context(cx: &LateContext<'_>, span: Span) {
    cx.emit_span_lint(
        BYPASS_ATTR_CONTEXT,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(
                "BYPASS_ATTR_CONTEXT: public #[allow(...)] comes from macro expansion",
            );
            let _ = diag.help("write lint exceptions directly in reviewed source or ledger them");
        }),
    );
}

fn emit_required_lint_weakening(cx: &EarlyContext<'_>, span: Span, lint: &str) {
    cx.emit_span_lint(
        BYPASS_REQUIRED_LINT_WEAKENING,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(format!(
                "BYPASS_REQUIRED_LINT_WEAKENING: crate-level allow weakens required lint `{lint}`"
            ));
            let _ = diag
                .help("keep the lint at deny/forbid level or use the audited exceptions ledger");
        }),
    );
}

fn emit_internal_unstable(cx: &EarlyContext<'_>, span: Span) {
    cx.emit_span_lint(
        BYPASS_INTERNAL_UNSTABLE,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(
                "BYPASS_INTERNAL_UNSTABLE: macro uses #[allow_internal_unstable(...)]",
            );
            let _ = diag.help("remove compiler-internal unstable escape hatches from macros");
        }),
    );
}

fn emit_internal_unsafe(cx: &EarlyContext<'_>, span: Span) {
    cx.emit_span_lint(
        BYPASS_INTERNAL_UNSAFE,
        span,
        DiagDecorator(|diag| {
            let _ =
                diag.primary_message("BYPASS_INTERNAL_UNSAFE: macro uses #[allow_internal_unsafe]");
            let _ = diag.help("remove compiler-internal unsafe escape hatches from macros");
        }),
    );
}

fn emit_internal_escapes(cx: &EarlyContext<'_>, attr: &rustc_ast::Attribute) {
    if attr.has_name(sym::allow_internal_unstable) {
        emit_internal_unstable(cx, attr.span);
    }
    if attr.has_name(sym::allow_internal_unsafe) {
        emit_internal_unsafe(cx, attr.span);
    }
}

fn lint_args(attr: &rustc_ast::Attribute) -> impl Iterator<Item = String> + '_ {
    attr.meta_item_list().into_iter().flatten().filter_map(lint_arg)
}

fn lint_arg(inner: MetaItemInner) -> Option<String> {
    match inner {
        MetaItemInner::MetaItem(meta) => Some(lint_path(&meta)),
        MetaItemInner::Lit(_) => None,
    }
}

fn lint_path(meta: &MetaItem) -> String {
    meta.path
        .segments
        .iter()
        .map(|segment| segment.ident.as_str().to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn is_required_lint(lint: &str) -> bool {
    REQUIRED_LINTS.contains(&lint)
}

const REQUIRED_LINTS: &[&str] = &[
    "warnings",
    "future_incompatible",
    "rust_2018_idioms",
    "unexpected_cfgs",
    "unsafe_code",
    "unsafe_op_in_unsafe_fn",
    "unused_must_use",
    "unused_results",
    "missing_docs",
    "unreachable_pub",
    "let_underscore_drop",
    "elided_lifetimes_in_paths",
    "explicit_outlives_requirements",
    "trivial_casts",
    "trivial_numeric_casts",
    "variant_size_differences",
    "unused_extern_crates",
    "unused_import_braces",
    "keyword_idents_2024",
    "clippy::all",
    "clippy::cargo",
    "clippy::pedantic",
    "clippy::nursery",
    "clippy::allow_attributes",
    "clippy::allow_attributes_without_reason",
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::unwrap_in_result",
    "clippy::panic",
    "clippy::panic_in_result_fn",
    "clippy::todo",
    "clippy::unimplemented",
    "clippy::unreachable",
    "clippy::dbg_macro",
    "clippy::print_stdout",
    "clippy::print_stderr",
    "clippy::indexing_slicing",
    "clippy::string_slice",
    "clippy::get_unwrap",
    "clippy::arithmetic_side_effects",
    "clippy::as_conversions",
    "clippy::cast_possible_truncation",
    "clippy::cast_possible_wrap",
    "clippy::cast_sign_loss",
    "clippy::cast_precision_loss",
    "clippy::integer_division",
    "clippy::integer_division_remainder_used",
    "clippy::modulo_arithmetic",
    "clippy::float_arithmetic",
    "clippy::result_large_err",
    "clippy::result_unit_err",
    "clippy::map_err_ignore",
    "clippy::missing_errors_doc",
    "clippy::missing_panics_doc",
    "clippy::missing_safety_doc",
    "clippy::large_enum_variant",
    "clippy::cognitive_complexity",
    "clippy::too_many_arguments",
    "clippy::too_many_lines",
    "clippy::type_complexity",
    "clippy::excessive_nesting",
    "clippy::await_holding_lock",
    "clippy::await_holding_refcell_ref",
    "clippy::future_not_send",
    "clippy::large_futures",
    "clippy::disallowed_methods",
    "clippy::disallowed_macros",
    "clippy::disallowed_types",
    "clippy::disallowed_fields",
    "clippy::multiple_crate_versions",
    "clippy::wildcard_dependencies",
    "clippy::negative_feature_names",
    "clippy::redundant_feature_names",
];
