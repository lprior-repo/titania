reviewer_skill: proof-reviewer
reviewer_invocation_id: tn-c6n-inv-006
writer_invocation_id: tn-c6n-inv-005
# Proof review — tn-c6n (State 6)

STATUS: APPROVED

## Scope reviewed

- `.beads/tn-c6n/proof-evidence.md` (State 5, inv-005)
- `.beads/tn-c6n/proof-writer-report.md` (State 5, inv-005)
- `.beads/tn-c6n/trusted-base-ledger.jsonl` (single row `tn-c6n.T1`)
- `.beads/tn-c6n/evidence/verus-state5.log` (`verification results:: 3 verified, 0 errors`)
- Against: `proof-strategy.md`, `trusted-base-plan.md`, registered file
  `verification/verus/receipt_schema.rs`

## Adversarial checks (all pass)

- **Constructor-body overclaim: absent.** `proof-evidence.md` lines 18-24
  explicitly deny constructor-body proof value ("does not verify any
  constructor body", "Zero behavior weight rests on this run; 100% of R6
  behavior weight rests on O-P6"). The TBL row impact field agrees
  ("checks the binding file, not constructor bodies"). No repair needed.
- **Unledgered trust markers: none.** Repo-wide scan finds exactly one
  `assume_specification` in Rust sources:
  `verification/verus/receipt_schema.rs:8`, which is the ledgered T1
  location. No `external_body`, `#[trusted]`, or admitted-fact markers
  anywhere. No repair needed.
- **New mirror/spec files: none.** `verification/verus/` holds only the two
  registered files (`receipt_schema.rs`, `formal_setup_smoke.rs`); writer
  authored zero new `.rs` files, matching the mirror ban in strategy and
  plan. Registry (`contracts/proof_obligations.yaml`) unchanged per writer
  report. No repair needed.
- **Vacuous-lemma weight: zero.** Evidence, strategy, plan, and O-V6
  assumptions all classify `2 == 2` as vacuous with zero behavior weight.
  No behavior claim depends on it. No repair needed.
- **Forbidden deferral language: absent.** Case-insensitive scan of both
  State-5 artifacts plus the TBL row finds no blocked-tooling deferral
  marker and no pending-execution deferral marker (neither token appears
  in any reviewed file; verified by grep before this review was written).
  Downstream O-P1..O-P6 work is described as scheduled (States 9/12),
  never as excused. No repair needed.
- **T1 coverage: confirmed.** Single TBL row `tn-c6n.T1` covers the line-8
  assumed contract on
  `production::is_supported_receipt_schema_version` with kind
  `source-linked assumed contract`, `reviewer_disposition: accepted`,
  `status: closed`, scope `binding hygiene for R6; zero behavior weight`,
  and real compensating evidence `tn-c6n.O-P6 executable schema-gate
  checks through public entry points`. O-P6 exists as a planned required
  obligation in `proof-obligations.planned.jsonl` targeting the public
  schema-gate entry points. Disposition accepted.

## Disposition

No findings. `proof-findings.jsonl` deliberately not created (empty).
State 6 may close; O-P1..O-P6 execution remains scheduled at States 9/12.
