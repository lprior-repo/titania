# Proof Review Report — tn-ily.3 (pr-surface)

**Session:** pristine-pass-b
**Scope:** `.evidence/verus/` (3 files found; 2 expected log files missing),
`.evidence/kani-list/` (1 file), Verus specs in `verification/verus/`
**Date:** 2026-07-02
**Reviewer:** proof-reviewer

---

## Summary

| Category | Count |
|---|---|
| CRITICAL | 0 |
| HIGH | 3 |
| MEDIUM | 3 |
| LOW | 2 |
| **Total** | **8** |

---

## CRITICAL

_None._

---

## HIGH

### H1: Missing verus receipt logs — no raw command evidence for formal verification runs

`.evidence/verus/summary.txt:3: HIGH: Expected log files verification_verus_formal_setup_smoke_rs.log and verification_verus_receipt_schema_rs.log are absent. The summary.txt contains only 3 of the expected 5 evidence lines. Without raw CLI command evidence (stdout/stderr of actual `verus` runs), the proof claims cannot be verified as having been executed. The summary.txt states VERUS_TARGET_COUNT 2 and VERUS_TARGETS_OK but provides no trace of the verus binary executing against the two targets. fix: Regenerate the evidence by running `verify_verus` end-to-end and capturing full stdout/stderr into the expected log files. The plan's EARS Unwanted rule ("IF a proof claim has no raw CLI command evidence, THE SYSTEM SHALL NOT consider it verified") is not satisfied.

### H2: assume_specification in receipt_schema.rs — trust boundary assumption on production contract

`verification/verus/receipt_schema.rs:8: HIGH: pub assume_specification[production::is_supported_receipt_schema_version] assumes the contract of the production const function without formal proof. While the production function is a trivial `schema_version == RECEIPT_SCHEMA_VERSION` comparison (line 6–7 of `crates/titania-core/src/receipt/schema.rs`), the assumption still bypasses Verus's contract verification. The waiver in `.evidence/verus/trusted-base-waivers.txt:3` justifies this as "Verus cannot execute the compiled Rust body in this harness," which is a convenience argument, not a correctness proof. For a const fn that is purely an equality check, this is acceptable in practice, but the assumption is still a trust gap. fix: Replace `assume_specification` with an explicit Verus spec that mirrors the const function's contract (`requires schema_version <= 3` or equivalent), then prove it against the production body. Alternatively, accept the waiver with a stronger justification noting the function's trivial purity (single `==` comparison, no side effects, no branches).

### H3: Kani workspace.json shows 0 contract harnesses — no formal contract proof for Kani

`.evidence/kani-list/workspace.json:16-21: HIGH: "contract-harnesses": [] and "functions-under-contract": 0 indicate zero Kani contract-level verification. The workspace only documents 8 standard harnesses in `crates/titania-core/src/kani.rs`, which test constructor invariants (empty lane rejection, null-byte rejection, path validation, scanned/passed ordering). No Kani harnesses verify the contract-level properties of the receipt domain (e.g., LaneDigest invariants across all operations, ReceiptError exhaustiveness, or QualityReceipt serialization/deserialization contracts). fix: Write Kani contract harnesses that verify the domain-level contracts, not just constructor boundaries. The plan's EARS Event-Driven rule states "WHEN a proof model or Kani harness is found, THE SYSTEM SHALL check for vacuous execution paths" — these 8 harnesses are not vacuous (they test real error paths), but their scope is narrow. Expand to cover the full receipt domain contract surface.

---

## MEDIUM

### M1: formal_setup_smoke.rs is vacuous by design but unprovable — smoke fixture only

`verification/verus/formal_setup_smoke.rs:6-11: MEDIUM: proof fn formal_setup_smoke() proves `1 + 1 == 2` — this is a vacuous tautology that proves nothing about the production codebase. It is correctly identified as a "fixture-smoke" (line 1 comment `titania-verus-binding: fixture-smoke`) and is used only to verify that Verus is installed and can execute. The waiver logic in `crates/titania-lanes/src/bin/verify_verus/registry.rs:48-50` correctly excludes it from production proof obligations. fix: No action required — this is intentional. Document this vacuousness explicitly in the waiver ledger or as a known limitation.

### M2: Kani harnesses use `kani::assume` to restrict input space — acceptable but unproven reachability

`crates/titania-core/src/kani.rs:25: MEDIUM: `kani::assume(passed > scanned)` in `lane_digest_rejects_passed_greater_than_scanned` restricts the input space to a specific condition. While the cover statement on line 26 (`kani::cover!(passed > scanned, ...)`) verifies reachability of the assumed condition, the assume itself is unproven. If the assumption cannot be satisfied (e.g., due to constraints on `kani::any()`), the harness is vacuous. fix: Verify that `kani::cover` succeeds in CI. If the cover predicate is never reached, the harness is vacuous and should be restructured (e.g., use `kani::any()` with post-assertion instead of assume).

### M3: Kani harnesses use early return on error paths — may skip assertions

`crates/titania-core/src/kani.rs:49-50: MEDIUM: In `lane_digest_accepts_passed_not_greater_than_scanned`, the early `Err(_) => return` on line 49-50 means that if `LaneName::new("fmt")` fails (which it cannot, since "fmt" is a valid literal), the harness silently exits without verifying anything. This is a minor issue because the literal `"fmt"` cannot produce an error, but the pattern could propagate to other harnesses that use variable input. fix: Replace `Err(_) => return` with `Err(_) => kani::assert(false, "lane creation should not fail for valid input")` to maintain assertion coverage even on unreachable paths.

---

## LOW

### L1: trusted-base-waivers.txt lacks expiration date and reviewer signature

`.evidence/verus/trusted-base-waivers.txt:6: LOW: The waiver record has `review: proof-reviewer finding 2026-07-02` but no explicit reviewer identity, no expiration date, and no linkage to a specific ticket beyond the text. The waiver is self-referential (the review is the finding itself). fix: Add explicit reviewer identity, ticket reference (e.g., `ticket: tn-ily.3`), and an expiration date (e.g., `expires: 2026-10-01`). This makes the waiver auditable and time-bounded.

### L2: No Verus specs exist for the receipt domain contracts

`verification/verus/: LOW: Beyond the smoke fixture and the receipt_schema assume_specification, there are zero Verus specs (`verus::spec`, `verus_external_body`, `verus_replaces`) in the entire codebase. The receipt domain (LaneDigest, LaneName, ReceiptError, RecordedTargetRoot, QualityReceipt) has no formal specifications. The 8 Kani harnesses cover constructors but not invariants, and the assume_specification on receipt_schema.rs is the only Verus-related artifact. fix: Write Verus specs for the receipt domain's core invariants (LaneName validity, LaneDigest consistency, ReceiptError exhaustiveness). Even if the assume_specification approach is retained for the const fn, the domain types should have explicit contract specifications.

---

## Verification Command Evidence

| File | Expected Log | Status |
|---|---|---|
| `verification/verus/formal_setup_smoke.rs` | `verification_verus_formal_setup_smoke_rs.log` | **MISSING** |
| `verification/verus/receipt_schema.rs` | `verification_verus_receipt_schema_rs.log` | **MISSING** |
| `crates/titania-core/src/kani.rs` | Kani harness execution log | MISSING (workspace.json exists but no execution log) |

---

## Conclusion

The proof evidence is **structurally present but execution-evidence deficient**. The summary.txt and trust-scan.txt exist and are internally consistent (1 external marker, 1 waiver, 0 forbidden trust findings). However, the two expected verification log files are missing, meaning there is no raw command evidence that `verus` was actually executed against the targets. The assume_specification in receipt_schema.rs is a documented trust boundary with a waiver, but the waiver justification is convenience-based rather than correctness-based. The Kani coverage is constructor-only with no contract harnesses.

**Recommendation:** Before accepting this proof surface as verified, generate the missing log files by running `verify_verus` end-to-end and capture full output.
