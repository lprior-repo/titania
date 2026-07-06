# tn-pdn — Truth-Serum Final Acceptance Audit

**Bead:** `tn-pdn` — Killer demo: prove for-loop plus unwrap rejection
**Auditor:** TnPdnTruthSerumFinal (Truth-Serum Evidence Auditor)
**Date:** 2026-07-05
**Workspace:** `/home/lewis/src/titania/.worktrees/v1-combined-dispatch`

---

## STATUS: APPROVED

---

## 1. Execution Evidence

### Killer demo test run (fresh, 2026-07-05)

```
$ cd /home/lewis/src/titania/.worktrees/v1-combined-dispatch
$ cargo test -p titania-check --test killer_demo
```

**Result:** 15/15 passed, 0 failures, 0.32s. Exit code: 0.

### Cross-check: bad-stdout.json vs evidence-bundle claims

| Claim | Actual (raw JSON) | Match |
|-------|-------------------|-------|
| `variant == "reject"` | `"reject"` | YES |
| `code_findings` contains `CLIPPY_UNWRAP_USED` | Found, lane=Clippy, effect=reject | YES |
| `code_findings` contains `FUNC_LOOPS_FOR` | Found, lane=AstGrep, effect=reject | YES |
| `gate_failures` is empty | `[]` (0 entries) | YES |
| `per_lane` has 7 lanes | 7 lanes: Fmt, Compile, Clippy, AstGrep, Dylint, PanicScan, PolicyScan | YES |
| CLIPPY_UNWRAP_USED repair hint | `requires_human_review` | YES |
| FUNC_LOOPS_FOR repair hint | `use_iterator_pipeline` | YES |

### Cross-check: repaired-stdout.json vs evidence-bundle claims

| Claim | Actual (raw JSON) | Match |
|-------|-------------------|-------|
| `variant == "pass"` | `"pass"` | YES |
| `receipt.schema_version == 1` | `1` | YES |
| `receipt.scope == "Edit"` | `"Edit"` | YES |
| `source_digest` present, 64-char hex | `b47c92...` (len=64, all lowercase hex) | YES |
| `cargo_lock_digest` present, 64-char hex | `87be94...` (len=64, all lowercase hex) | YES |
| `policy_digest` present, 64-char hex | `a18455...` (len=64, all lowercase hex) | YES |
| `toolchain_digest` present, 64-char hex | `2f12da...` (len=64, all lowercase hex) | YES |
| All 4 digests distinct | All 4 unique | YES |
| `per_lane` has 7 entries | 7 entries, all `clean` | YES |

### Cross-check: direct-cli-summary.json vs raw JSONs

All 10 fields (case, tempdir, exit_code, stderr_empty, variant, code_findings, gate_failures, per_lane, receipt_schema, stdout_path, stderr_path) verified consistent across all three raw JSON files. All referenced file paths exist on disk.

---

## 2. Hallucination Check

| Check | Result |
|-------|--------|
| Hallucinated file paths | NONE — all paths in evidence-bundle.md and contract.md verified existing |
| Hallucinated line numbers | NONE — all source refs verified at claimed lines (functional.yml:5, clippy_normalizer.rs:176, report.rs:33/60, v1_receipt.rs:78/120, gate_scope.rs:31, outcome.rs:122, repair_hint.rs:25/52) |
| Stale/contradictory text | NONE — all evidence-bundle claims match raw JSON content |

**Minor note:** Contract source ref for `clippy_normalizer.rs` line 176 says "`CLIPPY_UNWRAP_USED` mapping" — the actual code is a generic rule constructor `RuleId::new(&format!("CLIPPY_{}", lint.to_ascii_uppercase()))`. This is a description-level imprecision (the mapping exists dynamically, not via hardcoded match arm), not a hallucination. Behavior is correct.

---

## 3. Verification Laundering Check

| Check | Result |
|-------|--------|
| `#[verifier::external_body]` found | NONE |
| `assume(` / `axiom` found | NONE |
| Tests use real binary execution | YES — E2E tests (AC-1 through AC-7) spawn `titania-check` binary against temp-copied fixtures |
| Mock JSON used inappropriately | NO — mock JSON tests (B12-B14) are legitimate deserialization/invariant integration tests |
| Evidence is fresh | YES — killer_demo test re-run: 15/15 passed |

---

## 4. Adversarial Checks

| Anti-pattern | Found? |
|-------------|--------|
| Ellipsis laziness (`...`) | NO |
| Hallucinated paths | NO |
| Deleted tests | NO — all 15 tests present and passing |
| Contract parity | YES — all 8 ACs satisfied |
| Scope integrity | YES — only 5 allowed files created |
| Zero runtime panic surface | YES — zero `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`, `unreachable!()` in production code |
| Lazy error handling | NO |

---

## 5. Known Contamination Reconciliation

Both contamination sources are acknowledged and handled in the evidence-bundle:

1. **Parent `clippy.toml` contamination** — in-repo fixture path sees extra `CLIPPY_DISALLOWED_METHODS` diagnostic. Acceptance path uses fresh temp copies, avoiding this.
2. **Missing `/cache/cargo-shared/bin`** — Dylint lane infra failure without correct PATH. Acceptance evidence was captured with this PATH set.

These are environment issues, not implementation bugs. The acceptance evidence is clean.

---

## 6. Sibling Audit Cross-Reference

- **Black-hat final** (`black-hat-final.md`): STATUS: APPROVED, 8/8 ACs pass, 15/15 tests, no findings. Consistent with my audit.

---

## 7. UNVERIFIED Items

| Item | Reason |
|------|--------|
| `strace` evidence (evidence-bundle.md line 15) | Run by sibling agent; `cargo-dylint` PATH availability confirmed via direct test execution instead |

---

## 8. Residual Blockers

None. The `tn-fqd` thin-executor work is a separate P1 bead, not a blocker for `tn-pdn`.

---

## Final Verdict

**STATUS: APPROVED**

All acceptance criteria verified. No hallucinated claims, no stale contradictory text, no missing raw command evidence, no mismatch between raw JSON and evidence-bundle summary. Evidence is fresh, untainted, and consistent across all three raw JSON files. The killer demo proves exactly what it claims: bad code (for-loop + unwrap) is rejected with the correct findings, and repaired code (iterator pipeline) passes with a v1 receipt.
