# tn-ce7x — research notes

Exact source refs (all read before writing).

## Boundary under change

- `crates/titania-aggregate/src/report_assembly.rs:86-114` — `assemble_report`:
  diagnostics precedence → empty check → identity check → reject iff
  rejecting findings or gate failures → else pass. `Skipped` never
  contributes to either reject collection (`findings()` returns `None`,
  `failed()` returns `None` for `Skipped`, lines 127-152).
- `crates/titania-core/src/outcome.rs:300-312` — `LaneOutcome::is_pass`:
  `Clean | Skipped => true`. Root cause of the hole.
- `crates/titania-core/src/report/validators.rs:34-41` —
  `validate_per_lane_pass` uses `is_pass`; `Report::pass`
  (`report/accessors.rs:60-69`) therefore accepts any `Skipped`.
- `crates/titania-aggregate/tests/report_assembly.rs:256-282` — existing
  test pins skip-tolerant v1 behavior (`skipped_lanes_do_not_prevent_pass`).
  Must keep passing (v1 compat).

## Status matrix at the boundary (verified in sources)

- Skipped any reason → Pass. HOLE (this bead).
- `Failed` incl. `Tool{TimedOut}` → `gate_failures` → Reject (pre-existing).
- Missing entry → `Err(LaneIdentityMismatch)`; substituted/duplicated/
  reordered → `Err` (pre-existing, tested lines 484-666).
- Missing artifact file → `Failed{Infra "output file missing"}`
  (`artifact_reader.rs:291-298`) → Reject (pre-existing).
- Kani no-verdict states → reject findings (`PROOF_KANI_NO_VERDICT`,
  `PROOF_KANI_BLOCKED`, `PROOF_KANI_INFRA`) or `Failed`
  (`run_lane_kani.rs` docs lines 28-52 + `Finding::reject` at line ~900).
- Mutants survivors → `Finding::reject` (`MUTANT_SURVIVED`,
  `run_lane_mutants/outcomes.rs:11,50,123`).
- Kani/Mutants tool missing → `Skipped{ToolUnavailable}` → currently Pass
  (`run_lane_kani.rs:495,576,1550`; `run_lane_mutants/mod.rs:185`).
- UNPROVEN: no `LaneOutcome` variant, no producer. Informational proof
  findings (`SUCCESSFUL`, `UNSUPPORTED`, `NO_HARNESSES`) are pass-shaped
  by lane contract and must keep passing.

## Consumers (read, not changed)

- `crates/titania-check/src/aggregate.rs:148` calls v1 `assemble_report`;
  `lane_receipt` marks `clean = outcome.is_pass()` (line 231) — receipt
  layer unchanged by this bead.

## Duplicate-bead check

Parent states kani/mutants lanes owned by tn-4isu/tn-7bq2; this bead
touches only `titania-aggregate` classification. No overlap: no lane
mapping is changed, only the aggregation acceptance rule for a
versioned required set.

## Smallest correct change

New `titania-aggregate/src/obligations.rs` + one new assembly entry
point; v1 delegates with `v1_legacy`. No `titania-core` edits, no CLI
wiring, no spec rewrite.
