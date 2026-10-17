# Assurance bundle — tn-vr2n

## Requirement → evidence

| Requirement (bead §1) | Clause | Obligation/Bridge | Command (exit) | Log | Review |
|---|---|---|---|---|---|
| preserve mandatory lint severity in effective invocation | C1 | O-P1-BEH / B-P1 | `cargo test -p titania-lanes --test run_cargo_invocation` (0) | `logs/gate-test-lanes.log` | plan REVIEWED, suite REVIEWED, black-hat APPROVED |
| reject cap-lints/wrapper/config substitution | C1+C3 | O-P1-BEH, O-P3-BEH / B-P1, B-P3 | lanes suite (0); `cli_dispatch` 56/56 (0) | `logs/gate-test-lanes.log`, `logs/gate-test-check.log` | same as above |
| valid invocation evidence matches policy | C2 | O-P2-BEH / B-P2 | `cargo test -p titania-lanes --lib run_cargo` (0) | `logs/gate-test-lanes.log` | same as above |
| sccache wrapper without changed args may run | unchanged rule | (no change; existing tests) | full lanes suite (0) | `logs/gate-test-lanes.log` | black-hat A5 |

## Gates (all exit 0 unless noted)

fmt 0 · check 0 · source-only strict clippy 0 · lanes tests 0 ·
cli_dispatch 56/56 · doc 0 · failing-before 101 (expected red) ·
template_prepush 101 on base too (BLOCK_GLOBAL, environmental).

Excluded (residuals, not passes): moon ci, kani, mutants.

## Reviewer findings and dispositions

- F1 `-A` allow-scope follow-up: owner_approved_no_action (this bead).
- F2 String-over-newtype: owner_approved_no_action.
- F3 inline-only reviews: blocker for bead closure; handoff approved.
- No behavior waivers. No formal waivers. Trusted base: zero markers
  (attestation row tnvr2n.T-NONE).

## Unresolved debt

- Independent (non-inline) black-hat/truth-serum review: pending.
- Canonical `moon ci`: not run (delegation).
- `template_prepush` smoke tests: failing on base (environmental).
