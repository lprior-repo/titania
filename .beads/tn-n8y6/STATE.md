# tn-n8y6 STATE

Bead: `tn-n8y6` — tests: prove contract boundaries with behavior and real mutation execution.
Status: claimed by this worker. Worktree (isolated):
`/home/lewis/src/titania/.muse/worktrees/subagent-v2-01a14623-5a8f-76e0-be4a-be4eb153aaf7-01a146d6-875b-7803-8e34-67524bcc8fdd`
Base commit: `927893f922fe2b9c658a0c1574086afe7d6a6dc6`.

## Delegated scope (parent overrides bead text where they differ)

- Mutation evidence: `cargo mutants --check` scoped to the touched package
  (`titania-core`) only — fast build-check mode, NOT full mutation testing.
  Scope recorded honestly; `--check` output is never claimed as kill evidence.
- Kill evidence instead: named deliberately-defective production predicate
  (temp patch, reverted) killed by the new real behavior tests.
- Do NOT run full `moon ci` or kani/mutants lanes (pre-existing, owned by
  `tn-4isu`/`tn-7bq2`). Do NOT commit/push/sync.
- Mandatory gates: `cargo fmt --check`;
  `cargo check --workspace --all-targets --all-features`; source-only strict
  clippy per `AGENTS.md`; `cargo test` for touched crates fully green;
  `RUSTDOCFLAGS='-D warnings' cargo doc`.
- Validator `tools/go-skill-v9-validate` is ABSENT in this checkout
  (no `tools/` dir) — recorded as residual, not as pass.
- Specialist subagents: only `general-purpose` / `workflow-subagent` executors
  exist in this run, so skill bodies (`functional-rust`, `test-writer`) were
  loaded and applied inline; see `routing-ledger.jsonl`.

## Selected behaviors (smallest correct change, test-only)

Production APIs under test (all `titania-core` public API, no source changes):

1. `LaneDigest::new` boundary `passed <= scanned` — equality and zero-zero
   accepted with preserved getters; one-above rejected with exact
   `PassedExceedsScanned { passed, scanned }` payload; serde boundary rejects
   `passed > scanned` too.
2. `MutantsBaseline::contains` / `diff` transition — covered survivor is
   contained and excluded from diff; uncovered survivor is not contained and
   the diff returns the identical reference (`ptr::eq`); expired entry covers
   nothing; empty baseline contains nothing.
3. `ReceiptPeriod::new` boundary `finished_at >= started_at` — equality
   accepted with preserved times; one-below rejected with exact
   `FinishedBeforeStarted { started_at, finished_at }` payload.

## Progress

- [x] Research (Gate 0): read `killer_demo.rs`, `run_lane_mutants/command.rs`,
  `kani.rs`, `tn-6r36` contract/boundary/traceability/seeds, production
  `mutants_baseline.rs` / `receipt.rs` / `lane_name.rs` / `proof_id.rs` /
  `error.rs`, existing tests (`receipt_invariants`, `v15_mutants_baseline_*`).
- [x] Behavior tests written first:
  `crates/titania-core/tests/tn_n8y6_contract_boundaries.rs` (10 tests).
- [ ] Gates green (record below in `verification-ledger.jsonl`).
- [ ] Named-defect kill demo (temp patch + revert).
- [ ] `cargo mutants --check -p titania-core` (build-check inventory only).
- [ ] Review + evidence bundle + truth-serum self-audit.

## Resume

Read this file, then `routing-ledger.jsonl` (newest last), then the newest raw
command evidence referenced by `verification-ledger.jsonl`.
