# Test plan — tn-083n (behavior tests first)

## Strategy

Integration behavior tests only (`crates/titania-lanes/tests/trust_binding_audit.rs`)
against the public `trust_binding` API. Fixtures mirror the real
`verification/verus/receipt_schema.rs` (path + assume_specification +
tautology) and `formal_setup_smoke.rs` (fixture-smoke). No Verus execution;
evidence records are caller-supplied typed values whose revision is
hash-checked (tool-absent lane discipline).

## Cases (each maps to one acceptance criterion)

1. Verified body + passing evidence at current revision → `VerifiedProduction`
   naming binding + trusted base; report clean. (AC1)
2. Path-linked `assume_specification` without evidence → `SourceLinked`,
   never `VerifiedProduction`. (AC2)
3. No `#[path]` and zero `production::` refs → `Blocked(MissingBinding)`
   with `TRUST_MISSING_BINDING` finding. (AC3a)
4. Evidence revision != current production hash → `Blocked(MirrorDrift)`
   with `TRUST_MIRROR_DRIFT` finding. (AC3b)
5. Each marker (`assume(`, `external_body`, `admit`, `trusted`) without
   waiver → `Blocked(UnapprovedTrust)` + `TRUST_UNAPPROVED_MARKER`; with an
   explicit waiver → `SourceLinked` (still not verified). (AC4)
6. Tautology-only bound file (`2 == 2`, no `production::` refs) →
   `Blocked(VacuousProof)` + `TRUST_VACUOUS_PROOF`. (AC5)
7. `fixture-smoke` file as proof file → `Blocked(SmokeOnly)` +
   `TRUST_SMOKE_COVERAGE`, even with passing evidence. (AC6)
8. Evidence with nonzero exit → `Blocked(MissingEvidence)` +
   `TRUST_MISSING_EVIDENCE`. (contract step 5)
9. Waiver with empty approver/reason is not explicit → marker stays
   uncovered. (independent-approval strictness)

## Exactness

Every case asserts the exact verdict variant, exact rule-id string, and
exact finding count — no `is_ok()`-only assertions.
