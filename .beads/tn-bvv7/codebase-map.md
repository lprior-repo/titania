# Codebase Map — tn-bvv7 (explore)

Scope: pure-core effect-boundary enforcement. Only the ast-grep
architecture lane, its catalog/tests/fixtures, the repair catalog, and
the spec table change. No other lane, crate API, or Moon task changes.

| Path | Role | Change |
|------|------|--------|
| `crates/titania-lanes/src/ast_grep_lane/rules/detectors.rs` | String detectors for architecture rules | Extended 4 detectors with fully-qualified-path arms; added process/async/state detectors plus token-boundary helper; exact + property tests |
| `crates/titania-lanes/src/ast_grep_lane/rules.rs` | Runtime rule dispatch table | Added 3 `RuleDef` rows (`CoreDomainRust`, Reject) |
| `crates/titania-lanes/src/ast_grep_lane/rules/filters.rs` | Path-scope filter | Unchanged logic; added scope unit tests proving shell paths stay out of scope |
| `crates/titania-lanes/rules/architecture.yml` | Embedded catalog | Added fully-qualified regexes to 4 rules; added 3 rule documents |
| `crates/titania-lanes/tests/ast_grep_architecture.rs` | Catalog contract suite | 7 rule IDs, new fixtures, fully-qualified-pattern test, manifest-dependency test |
| `crates/titania-lanes/tests/ast_grep_lane.rs` | Lane behavior suite | 3 new end-to-end tests; full-matrix fixture list extended |
| `crates/titania-lanes/tests/fixtures/ast_grep/architecture/` | Content-checked fixtures | 13 new violation/allowed files |
| `crates/titania-lanes/tests/fixtures/ast_grep/crates/example-core/src/` | Lane-scanned fixtures | 8 mirrored files |
| `crates/titania-lanes/Cargo.toml` + `Cargo.lock` | Dev tooling | Added `proptest = "1"` dev-dependency (one lock line) for genuine property evidence |
| `crates/titania-core/src/finding/repair_catalog.{rs,tsv}` | Explain catalog | 3 new rows; async row mapped in `replace_dependency_hint`; count 80 → 83 |
| `v1-spec.md` | Normative rule table | 3 new rows plus fully-qualified/allowance paragraph |

Risks: string detectors cannot resolve HIR identities (future dylint
effect lint remains the residual); new STATE rule must never match the
frozen-catalog `LazyLock` pattern (pinned by exact + property tests);
`allowed_*` fixture prose must avoid banned literals (contract test
reads raw text including comments).
