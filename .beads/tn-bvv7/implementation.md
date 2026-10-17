# Implementation — tn-bvv7 (holzman-rust)

Production diff (Holzman zero-panic discipline: no unwrap/expect/
panic/todo, no indexing, checked splits, saturating arithmetic,
iterator pipelines, functions under 60 lines, max two nesting levels):

## Source Coverage Matrix

| Source | Change | Tests |
|--------|--------|-------|
| `rules/detectors.rs` | 4 detectors gain fully-qualified arms; 3 new detectors (process/async/state); `code_contains_token` boundary helper | 9 exact + 2 property |
| `rules.rs` | 3 new `RuleDef` rows (Reject, CoreDomainRust) | lane matrix test |
| `rules/filters.rs` | logic unchanged; 2 scope tests added | scope tests |
| `rules/architecture.yml` | FQ regexes + 3 rule documents | catalog tests (11) |
| `repair_catalog.{rs,tsv}` | 3 rows; async `ReplaceDependency` mapping; count 83 | catalog tests (8) |
| `titania-lanes/Cargo.toml` | `proptest = "1"` dev-dep | property harness |
| `v1-spec.md` | 3 rule rows + FQ/allowance paragraph | docs only |

Rust safety evidence: `cargo fmt --check` 0, `cargo check
--workspace --all-targets --all-features` 0, source-only strict clippy
0, `cargo doc -D warnings` 0, workspace `#![forbid(unsafe_code)]`
untouched (zero unsafe added; `cargo geiger` not run — not in the
parent's gate list). No function exceeds reviewability ceilings; the
longest new function (`detect_core_state_effect`) is a single
boolean chain.
