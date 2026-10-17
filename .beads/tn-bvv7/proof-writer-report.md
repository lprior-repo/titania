# Proof Writer Report — tn-bvv7

No separate proof/model artifacts: both obligations are executable
property harnesses over production code (implementation-bound lanes).

- Harness 1: `tn_bvv7_fq_bypass_property` in
  `crates/titania-lanes/src/ast_grep_lane/rules/detectors.rs`
  (64 cases; covers fully-qualified, renamed-import, and macro-hidden
  legacy effects under benign noise).
- Harness 2: `tn_bvv7_effect_boundary_property` in the same module
  (64 cases; covers process/async/state firing plus the frozen-catalog
  allowance staying silent under the same noise).

Smoke evidence: both harnesses typecheck under
`cargo check --workspace --all-targets --all-features` (exit 0) and
pass under `cargo test` (exit 0); raw log in
`evidence/proptest-obligations.txt`. The crate has zero unsafe blocks
(workspace-wide forbid), zero model copies of production logic, and
zero proof-only helpers. Base ledger: the accompanying ledger file
records a clean marker scan with grep evidence.
