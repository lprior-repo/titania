//! Titania v1 dylint rule catalog.
//!
//! The cdylib crate is intentionally dependency-free at this layer. Rule ids,
//! diagnostics, and repair hints are stable data consumed by the loader and by
//! future `LateLintPass` adapters.

/// One v1 dylint rule contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DylintRule {
    id: &'static str,
    diagnostic: &'static str,
    repair_hint: &'static str,
}

impl DylintRule {
    /// Stable `titania-check` rule id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        self.id
    }

    /// Diagnostic message emitted for the lint.
    #[must_use]
    pub const fn diagnostic(self) -> &'static str {
        self.diagnostic
    }

    /// Concrete repair hint shown with the diagnostic.
    #[must_use]
    pub const fn repair_hint(self) -> &'static str {
        self.repair_hint
    }
}

/// Required v1 type-aware dylint rules.
pub const RULES: &[DylintRule] = &[
    DylintRule {
        id: "RESULT_STRING_ERROR_TYPED",
        diagnostic: "Result error types must be typed enums, not String",
        repair_hint: "Replace `Result<T, String>` with a domain error enum deriving `thiserror::Error`.",
    },
    DylintRule {
        id: "UNWRAP_IN_MACRO_EXPANSION",
        diagnostic: "macro expansions must not hide unwrap/expect panic paths",
        repair_hint: "Return `Result` from the macro-expanded code and propagate with `?` or map to a typed error.",
    },
    DylintRule {
        id: "BYPASS_PUB_ALLOW",
        diagnostic: "public items must not carry lint-suppression attributes",
        repair_hint: "Remove the public `#[allow(...)]`; move any justified exception to the strict-ai exception ledger.",
    },
    DylintRule {
        id: "BYPASS_INTERNAL_UNSAFE",
        diagnostic: "internal unsafe bypasses are forbidden in v1 strict-ai code",
        repair_hint: "Delete the unsafe path or isolate it behind a pre-approved waiver with a safe audited adapter.",
    },
    DylintRule {
        id: "BYPASS_INTERNAL_UNSTABLE",
        diagnostic: "unstable-feature bypasses are forbidden without explicit policy approval",
        repair_hint: "Remove the unstable feature gate or document the approved policy exception with owner and expiry.",
    },
    DylintRule {
        id: "ASYNC_IN_SYNC_VIA_TRAIT",
        diagnostic: "sync-core traits must not hide async runtime coupling",
        repair_hint: "Move async work to an adapter boundary and expose a synchronous pure-core trait or command value.",
    },
];

/// Returns the stable v1 dylint catalog.
#[must_use]
pub const fn rules() -> &'static [DylintRule] {
    RULES
}
