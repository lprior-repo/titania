# tn-ce7x — STATE.md

Bead: `tn-ce7x` — aggregate: block skipped and unproven mandatory Phase 1 obligations.
Worktree: `/home/lewis/src/titania/.muse/worktrees/subagent-v2-01a14623-5a8f-76e0-be4a-be4eb153aaf7-01a146d6-8277-7a00-86f7-0f6fae9d70ee`
Claimed: 2026-10-16 via `bd update tn-ce7x --claim` (was OPEN, unblocked).

## Scope (from bead text; no local contract.md exists)

Introduce an explicit required-versus-optional obligation contract at the
aggregation boundary. Preserve v1 `NotApplicable` compatibility for genuinely
optional legacy work. A mandatory critical check must be rejected when
Skipped, UNPROVEN, missing, failed, timed out, or substituted by another lane.
No global rewrite of v1 skip semantics without a versioned contract.

## Research findings (see research.md)

- `assemble_report` (v1) treats ALL `Skipped` as pass-shaped. Only hole.
- Failed (incl. timeouts) already → Reject. Missing/substituted/duplicated →
  `Err`, no pass receipt. Proof-lane no-verdict states arrive as reject
  findings or `Failed` (verified in lane sources, not executed).
- Kani/Mutants "tool not installed" → `Skipped{ToolUnavailable}` → currently
  PASSES Full-scope aggregation. This is the concrete hole.
- UNPROVEN has no `LaneOutcome` producer today.

## Decision

Versioned contract in `titania-aggregate` (no `titania-core` changes):

- `ObligationSet` bound to one `GateScope`: `v1_legacy` (all optional,
  byte-identical v1 semantics), `all_required`, `requiring(scope, lanes)`.
- `assemble_report_with_obligations(obligations, outcomes, receipt,
  policy_diags, input_diags)` — scope carried by the obligation set
  (stays within the 5-arg clippy ceiling). Required + `Skipped` →
  `Report::Reject` (GateOnly) with a typed `LaneFailure::Infra` failure.
  Exhaustive outcome match: a future `LaneOutcome` variant fails
  compilation until classified.
- v1 `assemble_report` delegates to the new path with `v1_legacy`.
- No caller migration (`titania-check aggregate` stays on v1): no policy
  source declares required lanes yet; wiring strict-by-default would be the
  global rewrite the bead forbids. Follow-up bead needed.

## Gates (parent-mandated; moon ci and kani/mutants lanes excluded, owned by tn-4isu/tn-7bq2)

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --all-features`
- source-only strict clippy per AGENTS.md
- `cargo test` for touched crate (`titania-aggregate`) fully green
- `RUSTDOCFLAGS='-D warnings' cargo doc`

`tools/go-skill-v9-validate` does not exist in this checkout (no `tools/`
dir); validator STATUS cannot be produced — recorded as residual.
Specialist skills executed inline (no subagent specialist pool in this
child); every handoff logged in routing-ledger.jsonl as degraded fallback.

## Status — DONE, handoff (no commit/push/sync, bead left claimed)

- [x] Research
- [x] Failing-first tests (red E0432 in `01-red-phase.log`, green 11/11 after)
- [x] Implementation (`obligations.rs` + `assemble_report_with_obligations`;
      v1 delegates with `v1_legacy`)
- [x] Targeted gates (fmt 0, check 0, strict source clippy 0, crate tests
      56/56, workspace doc 0 — logs in `evidence/`)
- [x] Inline adversarial review (labeled, not an independent specialist):
      contract parity holds for all six bead statuses; no gate weakened
      (all 45 pre-existing aggregate tests pass unchanged); UNPROVEN has no
      producer — exhaustive match forces future classification at compile
      time; `skip_reason_text` asserts only the stable prefix, not the full
      Debug string.
- [x] Evidence bundle + final report (this file + `research.md` +
      `routing-ledger.jsonl` + `evidence/01-08`).

## Follow-ups (not done, out of scope)

- Wire a policy source declaring required lanes and route
  `titania-check aggregate` through `assemble_report_with_obligations`
  (new bead; strict-by-default now would be the forbidden global rewrite).
- `tools/go-skill-v9-validate` absent in this checkout → no validator STATUS.
