# Test Plan Review: tn-pdn — Killer Demo (Second Review)

STATUS: APPROVED

---

## Review Summary

This is a second (re-)review of the repaired test plan (`test-plan.md`) following the first review's four findings (F1–F4). All four findings have been **correctly fixed** with no regressions introduced. The plan is structurally sound, internally consistent, and ready for the test-writer.

---

## Findings — All Resolved

### F1: GateOnly and Mixed RejectKind scenarios — FIXED ✅

**Severity (prior): BLOCKER** → **Disposition: fixed_with_evidence**

The repair added B13 (`report_reject_gate_only`, lines 281–298) and B14 (`report_reject_mixed`, lines 300–316) as integration tests using in-memory `Finding`/`LaneFailure` constructs.

- **B13** asserts: 0 code findings + 1 Dylint gate failure → `reject_kind == "gate_only"`, `code_findings` length 0, `gate_failures` length 1.
- **B14** asserts: 1 AstGrep code finding + 1 Compile gate failure → `reject_kind == "mixed"`, `code_findings` length 1, `gate_failures` length 1.

Both cover the remaining `RejectKind` variants not tested by B5 (`CodeOnly`). ✅

---

### F2: B12 now tests Mixed separation, not duplicating B1 — FIXED ✅

**Severity (prior): ERROR** → **Disposition: fixed_with_evidence**

B12 (lines 260–279) was reframed from a redundant CodeOnly assertion to a distinct Mixed-case lane-separation test:

- Constructs: 2 functional `Finding`s (AstGrep + Clippy) + 1 infrastructure `LaneFailure` (Dylint).
- Asserts: `reject_kind == "mixed"`; functional lanes appear only in `code_findings`; infrastructure lane appears only in `gate_failures`; no cross-contamination.

This is a genuinely distinct scenario from B1 (which tests the bad-fixture CodeOnly case via CLI). ✅

---

### F3: Invalid Vec<u8> proptest removed — FIXED ✅

**Severity (prior): ERROR** → **Disposition: fixed_with_evidence**

P1 (which generated `Vec<u8>` incompatible with `reject_kind_for`'s typed-collection signature) has been removed. P2's exhaustive `(bool, bool)` enumeration of all 4 `(code_empty, gate_empty)` combinations subsumes P1's intent. The proptest section now contains only P2 (exhaustive RejectKind mapping) and P3 (RepairHint JSON round-trip), both with valid strategies. ✅

---

### F4: Dylint mock promoted into Given clauses — FIXED ✅

**Severity (prior): MINOR** → **Disposition: fixed_with_evidence**

The Dylint artifact mock is now present in the Given clauses of both B1 (line 76) and B6 (line 175):

> `Dylint.lane-artifact.json` at `.titania/out/edit/Dylint.lane-artifact.json` with variant `"clean"` (mock artifact to prevent Dylint infra-fail)

Open Questions Q1 and Q4 are marked RESOLVED. ✅

---

### F5 (residual): B11 missing AC reference — STATUS: INFORMATIONAL, no action needed

The contract does not list an explicit AC for the missing-Cargo.toml error path. However, B11 covers an implicit CLI contract behavior (exit code 3, `InputError` on stderr) that is consistent with the bead's error-path expectations. The prior disposition of `owner_approved_no_action` stands. ✅

---

## Mutation Thought Experiment — Updated

| Mutation | Catches By | Caught? |
|----------|-----------|---------|
| Delete `FUNC_LOOPS_FOR` rule_id assertion from B1 | B1 Then clause (explicit `rule_id == "FUNC_LOOPS_FOR"` at line 86) + B2 | ✅ Yes — B1's explicit assertion catches this |
| Delete `CLIPPY_UNWRAP_USED` rule_id assertion from B1 | B1 Then clause (explicit `rule_id == "CLIPPY_UNWRAP_USED"` at line 86) + B2 | ✅ Yes — B1's explicit assertion catches this |
| Move findings from `code_findings` to `gate_failures` | B4 (`gate_failures` empty) + B12 (lane separation) | ✅ Yes |
| Omit receipt digests | B8 (asserts all 4 non-zero 64-char hex strings) | ✅ Yes |
| Flip RejectKind classification (CodeOnly↔GateOnly) | B5 + B13 + B14 (three independent assertions across variants) | ✅ Yes |

**Critical gap closed**: The prior review identified that B1 only asserted `code_findings` length == 2 without checking individual rule IDs. The repair added explicit `rule_id` assertions to B1's Then clause (line 86), closing this gap. B2 remains as a secondary confirmation. ✅

---

## Gate Checklist

| Gate | Status | Notes |
|------|--------|-------|
| 1. Every public behavior has G/W/T scenario | PASS | AC-1 (B1–B2), AC-2 (B3), AC-3 (B6–B7), AC-4 (B4, B12), AC-5 (B5, B13, B14), AC-6 (B8), AC-7 (B9, B10). B11 covers an implicit error path. |
| 2. Every error variant has exact assertion | PASS | `CodeOnly` (B5), `GateOnly` (B13), `Mixed` (B14) — all three RejectKind variants have dedicated tests with exact field assertions. |
| 3. Concrete assertions (no smoke) | PASS | All scenarios use exact counts, exact field values, hex-length checks, lane-name lists. |
| 4. Boundaries named | PASS | B11 (empty-input boundary), P2 (all 4 RejectKind input combinations exhaustive). |
| 5. Property tests for pure behavior | PASS | P2 (RejectKind exhaustive mapping) + P3 (RepairHint round-trip). |
| 6. Fuzz/adversarial input planned | PASS | Not in scope — explicitly deferred to v1.5+. |
| 7. Verifier harnesses don't count as behavior tests | PASS | Kani explicitly scoped out; no verifier harnesses counted as tests. |
| 8. Proof-to-implementation mapped to executable tests | PASS | All proptest invariants test pure production functions (`reject_kind_from_empty`, `RepairHint` serialization). |

---

## Internal Consistency Checks

- **Summary table** (lines 5–14): Behavior tests 14, Integration 3, Property 2, Static 1. Counts match the actual scenarios listed in Sections 1, 3, and 8. ✅
- **Trophy Allocation** (Section 2): ~60% E2E/CLI, ~25% integration, ~10% property, ~5% static — matches the scenario distribution. ✅
- **Evidence Commands** (Section 10): All 14 test names have corresponding `cargo test` commands. ✅
- **Pre-Conditions** (Section 9): RED tests are fixture-dependent (CLI tests); GREEN tests are in-memory (integration tests). Classification is correct. ✅
- **Open Questions** (Section 10): Q1/Q4 resolved. Q2 (Cargo.lock), Q3 (positional path), Q5 (fixtures location), Q6 (policy digest) are legitimate open questions that the test-writer can resolve during implementation. No stale blockers. ✅

---

## Disposition Summary

| Finding | Severity (Prior) | Disposition | Status |
|---------|-----------------|-------------|--------|
| F1: RejectKind::GateOnly/Mixed untested | BLOCKER | fixed_with_evidence | ✅ Resolved |
| F2: B12 redundant with B1 | ERROR | fixed_with_evidence | ✅ Resolved |
| F3: P1 proptest wrong type | ERROR | fixed_with_evidence | ✅ Resolved |
| F4: Dylint mock not in Given clauses | MINOR | fixed_with_evidence | ✅ Resolved |
| F5: B11 missing AC reference | INFORMATIONAL | owner_approved_no_action | ✅ No action needed |

---

## Final Note

The repaired test plan is **approved for execution**. All four prior findings have been addressed with correct, minimal changes. The BDD scenarios are well-structured with concrete assertions, the trophy allocation is reasonable, the mutation thought experiment gap is closed, and the proptest invariants are valid. The test-writer can proceed to implementation.

**Files reviewed only**: `.beads/tn-pdn/test-plan.md`, `.beads/tn-pdn/test-plan-review.md`, `.beads/tn-pdn/contract.md`. No production, test, or fixture files were edited. No gates were run.
