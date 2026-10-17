reviewer_skill: proof-plan-reviewer
reviewer_invocation_id: tn-c6n-inv-004b
writer_invocation_id: tn-c6n-inv-004
# Proof plan review — tn-c6n (State 4b)

STATUS: REJECTED

23 of 24 lane decisions are accepted. One lane is rejected with a blocking
finding in `proof-plan-findings.jsonl` (`tn-c6n-F01`); see the repair guide
below. The initial accept-all draft was corrected after the validator
corroborated the unevidenced rule-out.

## Adversarial checks performed

- **Vacuous obligations: none found.** `tn-c6n.O-P1`..`O-P6` each assert
  exact accept/reject equivalence over a full input class (empty/NUL
  strings, `u32 x u32` with `(0,0)`/`(MAX,MAX)`/`(1,0)`, nine-variant
  enumeration, `u64 x u64` with equal/`(MAX,0)` stamps, path corpus,
  sampled-plus-arbitrary versions) calling the production constructor
  directly, with exact error-variant assertions. The only vacuous item in
  scope (the `2 == 2` lemma in `verification/verus/receipt_schema.rs`) is
  explicitly classified vacuous with zero behavior weight in
  `proof-strategy.md`, `O-V6` assumptions, and `trusted-base-plan.md`.
- **Mirror-type laundering: none.** No new spec files are planned. The verus
  binding uses `#[path = "../../crates/titania-core/src/receipt/schema.rs"]`
  plus `assume_specification[...]` (STRONG production binding), targets a
  registry-owned file the bead does not modify, and is claimed as binding
  hygiene only — never as constructor-body proof, per `tn-083n`, stated in
  three separate artifacts.
- **Kani cover-only reasoning: none.** Zero kani obligations exist, so no
  `cover!`-as-proof is possible. All six kani lanes are ruled
  `not_applicable` with concrete evidence
  (`.beads/tn-c6n/evidence/kani-absent.log`, exit 1, plus delegation of
  bounded-proof execution to `tn-7bq2`); rejection behavior is covered by
  exhaustive proptest input classes instead.
- **Behavior-affecting waivers: none.** `waiver-candidates.jsonl` holds the
  single explicit no-waiver record (`tn-c6n-WAIVER-NONE`,
  `behavior_affecting: false`, `review_status: approved`, expiry
  2027-01-01). Formal gaps are residuals owned by `tn-7bq2`, not waivers.
- **Unevidenced lane rule-outs: ONE FAILURE (`tn-c6n-F01`).**
  `tn-c6n.D-S6-flux-rs` rules the R6 flux-rs lane `not_applicable` with an
  empty `non_applicability_evidence_refs` list — reason text only, no cited
  artifact — unlike its five sibling flux-rs rows. Rejected.
- **Verus-binding overclaim: absent.** The plan denies constructor-body
  proof value for `O-V6` explicitly; 100% of behavior weight rests on the
  executed `cargo test` obligations.

## Dispositions

`verifier-lane-review.jsonl`: 23 rows `accepted`, 1 row (`tn-c6n.D-S6-flux-rs`)
`rejected` under independent invocation `tn-c6n-inv-004b` (planner
`tn-c6n-inv-004`; IDs differ, no self-approval).

## Repair guide (for proof-planner, State 4)

1. In `verifier-lane-decisions.jsonl`, row `tn-c6n.D-S6-flux-rs`: add
   concrete `non_applicability_evidence_refs`, e.g.
   `crates/titania-core/src/receipt/schema.rs` (const-equality gate, no
   flux annotations in crate) plus the boundary/registry ref used by the
   S1–S5 sibling flux-rs rows. Change nothing else.
2. Notify the reviewer to re-run this State 4b review: the reviewer will
   flip that single lane to `accepted`, clear `proof-plan-findings.jsonl`,
   and set `STATUS: APPROVED`.
3. Out of scope for this bead: the validator also reports a pre-existing
   `E_INVOCATION_LEDGER_FORGED` hash mismatch on ledger row 1
   (orchestrator `STATE.md`, recorded in the source checkout; this
   worktree holds an untracked copy). Owned by States 1–3, not the
   State-4 plan; flagged here for the orchestrator.

Proof writing must not start until this review flips to APPROVED.
