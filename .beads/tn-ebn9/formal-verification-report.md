# tn-ebn9 Formal-Verification Report

STATUS: PASS

Machine execution of all required proof/refinement/test obligations
(verifier: proptest, executable corpus form):

- tn-ebn9.O-P1-PROP: `cargo test -p titania-dylint --test policy_surface
  --all-features` + `cargo test -p titania-lanes --all-features` +
  `cargo test -p titania-check --test killer_demo
  bounded_for_with_question_passes` — all exit 0.
- tn-ebn9.O-P2-PROP / O-P3-PROP / O-P4-PROP: same suites, exit 0
  (855 + 17 passing assertions; raw logs retained).

`verification-ledger.jsonl` closes 4 proof + 4 bridge rows as PASS with
commands, workdirs, exit statuses, tool versions, raw-log excerpts, and
evidence artifacts. No `PENDING_FORMAL_EXECUTION`, no `planned` mappings,
no pending base dispositions, no waivers (behavior or otherwise).

Out of scope (recorded, never counted): Kani, mutants, `moon ci` (parent
direction; owners tn-4isu/tn-7bq2); two environmental test failures
(cli_dispatch doctest tempfile quota; template_prepush network installs),
both reproduced-or-self-evident off the critical path and classified
BLOCK_GLOBAL in `global-readiness-report.md`.
