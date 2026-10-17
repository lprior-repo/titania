reviewer_skill: test-reviewer
reviewer_invocation_id: tn-c6n-inv-010
writer_invocation_id: tn-c6n-inv-009

# Test-plan review — tn-c6n (State 10)

STATUS: APPROVED

## Scope

Reviewed `.beads/tn-c6n/test-plan.md` against the contract clauses (R1–R6),
`.beads/tn-c6n/rust-refinement-obligations.jsonl` (bridge IDs tn-c6n.B-P1…
B-P6, B-V6), and the as-built suite
(`crates/titania-core/tests/receipt_domain_invariants.rs`,
`crates/titania-core/tests/receipt_domain_properties.rs`).

## Checks

- Behavior inventory covers all contract clauses: lane name (empty / NUL /
  preserve), lane digest (reject above-scanned direct + JSON / accept
  boundaries), error exhaustiveness (9 variants, exact messages), period
  (equal accept / reversed reject), target root (empty / relative / NUL /
  manifest join), exit round-trip (4 variants), schema gate (v2 accept /
  0, 1, 3, MAX reject). Pass.
- Trophy allocation is honest: pure validation core, every check an
  integration check through real constructors with real JSON text; static
  layer via compile-time exhaustiveness. Pass.
- BDD scenarios name Given/When/Then with exact expected variants. Pass.
- Proptest plan asserts accept/reject equivalence plus payload preservation;
  no `is_ok()`-only plan entries. Pass.
- Mutation checkpoints (guard flip must fail closed; new variant must break
  the exhaustive match) are stated. Pass.
- Combinatorial and proof/refinement matrices list every bridge ID with the
  correct behavior tests and harnesses. Pass.

## Corrections to apply with the suite repair (no independent plan rework)

1. `§4` / matrix harness cell `prop_receipt_error_messages_stable` does not
   resolve: the built check is the plain `#[test]`
   `error_messages_stable_across_all_variants`. Update the name when the
   suite repair lands (see test-suite-review.md, finding F1).
2. "Proptest invariants: 6" overstates: as built there are 5 proptest
   properties plus 1 plain stability check. Reword to "5 proptest + 1
   plain".
3. "13 integration" undercounts test fns: 13 behaviors are covered by 14
   test fns (digest reject is split constructor + JSON). Reword to "13
   behaviors in 14 test fns".

None of these is missing coverage; the plan itself is approved.
