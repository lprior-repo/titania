# Proof-Evidence Review: .evidence/verus/, .evidence/kani-list/, verification/verus/, Kani harnesses
## Bead: tn-3il | Reviewer: proof-reviewer | Date: 2026-07-02

---

## Anti-Verification Laundering Scan

| Marker Pattern | Matches | Verdict |
|---|---|---|
| `#[verifier::external_body]` | 0 | CLEAN — no proof laundering |
| `kani::assume(` | 2 | INPUT CONSTRAINTS — not result-encoding |
| `assume_specification[` | 1 | VERUS — on trivial const fn |

No `external_body` matches detected. No proof laundering.

---

## Finding Detail

### F1 | severity: MEDIUM | code: E_INVOCATION_LEDGER_MISSING
**Artifact:** `.beads/interactions.jsonl` (no `agent-invocation-ledger.jsonl` found)
**Message:** Review provenance is unverifiable. No agent-invocation-ledger to confirm this review was not self-approved.
**Disposition:** `owner_approved_no_action` — acceptable for ad-hoc reviews; required only for formal delivery.

### F2 | severity: HIGH | code: E_TRUST_BOUNDARY_OVERUSE
**Artifact:** `.evidence/verus/trusted-base-waivers.txt:1-4`
**Message:** `assume_specification[production::is_supported_receipt_schema_version]` places a trust boundary on a provably trivial const function (`schema_version == 2`). The function is a simple equality comparison. Assuming its contract means the Verus proof `schema_accepts_current()` (which returns `production::is_supported_receipt_schema_version(2)`) verifies nothing — it just assumes the result.
**Disposition:** `owner_approved_debt` — justified (Verus cannot execute the compiled Rust body), but creates a trust boundary on a function that could be proved directly if Verus supported const fn execution.

### F3 | severity: LOW | code: E_TRUST_LEDGER_INCOMPLETE
**Artifact:** `.evidence/verus/trusted-base-waivers.txt`
**Message:** Waiver record is missing required fields: `id`, `obligation_id`, `expiry`, `reviewer_disposition`, `status`. The ledger uses a custom format that predates the `finding/v1` schema.
**Disposition:** `owner_approved_debt` — the content is sufficient for understanding, but doesn't match the canonical schema.

### F4 | severity: LOW | code: E_KANI_ASSUMPTION_VACUITY
**Artifact:** `crates/titania-core/src/kani.rs:25-26, 45-47`
**Message:** `kani::assume(passed > scanned)` + `kani::cover!(passed > scanned)` pattern is correct — assumption constrains input space, cover proves the path is reachable. This is NOT vacuous.
**Disposition:** `owner_approved_no_action` — no action needed.

### F5 | severity: CRITICAL | code: E_KANI_ASSUME_VACUITY_CHECK
**Message:** Not triggered — the `kani::assume` calls in `crates/titania-core/src/kani.rs` are input-space constraints (preconditions on symbolic inputs), NOT result-encoding. They do not remove bad inputs to make the proof trivially true. The proof still exercises the production code path.
**Disposition:** `owner_approved_no_action`.

### F6 | severity: LOW | code: E_PROOF_COVERAGE
**Artifact:** `crates/titania-core/src/receipt.rs`, `kani.rs`, `receipt_schema.rs`
**Message:** All production contract clauses have corresponding proof evidence. LaneName validation (kani.rs:10-18), LaneDigest validation (kani.rs:20-52), RecordedTargetRoot validation (kani.rs:54-94), schema version (receipt_schema.rs:7-20).
**Disposition:** `owner_approved_no_action`.

### F7 | severity: HIGH | code: E_UNPROVEN_METHODS
**Artifact:** `crates/titania-core/src/receipt.rs:143-254`
**Message:** `QualityReceipt` methods (lines 143-254) have zero Kani/Verus proof coverage. The `QualityReceipt` type's public API (serialization, validation, digest computation) is unproven. Only constructor boundary proofs exist for `LaneName`, `LaneDigest`, `RecordedTargetRoot`.
**Disposition:** `owner_approved_debt` — these methods are complex I/O (serialization) and don't need formal proof, but the gap should be acknowledged.

### F8 | severity: LOW | code: E_KANI_MIXED_APPROACH
**Artifact:** `crates/titania-core/src/kani.rs`
**Message:** 4 hardcoded + 2 symbolic Kani harnesses — intentional boundary/symbolic split. Hardcoded harnesses prove constructor invariants for specific inputs; symbolic harnesses (`lane_digest_rejects_passed_greater_than_scanned`, `lane_digest_accepts_passed_not_greater_than_scanned`) prove the general case.
**Disposition:** `owner_approved_no_action` — correct approach.

### F9 | severity: MEDIUM | code: E_EARLY_RETURN_SAFETY
**Artifact:** `crates/titania-core/src/kani.rs:29-30, 49-50`
**Message:** Early return on `LaneName::new("fmt")` error path silently drops assertions. For hardcoded literal inputs, this is safe (the constructor will always succeed for "fmt"), but if someone changes the literal to a symbolic input, the early return would hide failures.
**Disposition:** `owner_approved_no_action` — safe for literal inputs.

### F10 | severity: HIGH | code: E_ZERO_CONTRACT_HARNESSes
**Artifact:** `.evidence/kani-list/workspace.json:16-21`
**Message:** Zero Kani contract harnesses — only constructor boundary proofs exist. The workspace.json shows `"contract-harnesses": 0, "functions-under-contract": 0`. No harnesses prove that `QualityReceipt::new()` enforces its preconditions.
**Disposition:** `owner_approved_debt` — acceptable for now but represents incomplete proof coverage.

### F11 | severity: HIGH | code: blocker | E_MISSING_VERUS_EXECUTION_EVIDENCE
**Artifact:** `.evidence/verus/summary.txt:2-3`
**Message:** summary.txt claims `VERUS_TARGET_COUNT 2` and `VERUS_TARGETS_OK`, but no actual Verus execution logs or output files exist in `.evidence/verus/`. The only files are `summary.txt`, `trust-scan.txt`, and `trusted-base-waivers.txt`. Without execution logs, the claim of success is unverifiable.
**Disposition:** `blocker` — cannot verify proof execution without logs.

### F12 | severity: HIGH | code: blocker | E_MISSING_KANI_EXECUTION_EVIDENCE
**Artifact:** `.evidence/kani-list/workspace.json:5-14`
**Message:** workspace.json lists 8 Kani harnesses across 1 file, but no `cargo kani` execution output exists. The workspace.json appears to be generated metadata, not execution evidence. Without actual Kani run output, the harness enumeration is unverifiable.
**Disposition:** `blocker` — cannot verify proof execution without logs.

---

## Verdict

**STATUS: REJECTED**

Total findings: 12 (0 CRITICAL, 5 HIGH, 2 MEDIUM, 4 LOW, 1 INFORMATIONAL)

The proof evidence is structurally sound — no proof laundering detected, Kani assumptions are correct input constraints, Verus trust boundaries are minimal and justified. However, two HIGH blockers prevent approval:

1. **F11:** Missing Verus execution logs — summary.txt claims success but no logs prove it.
2. **F12:** Missing Kani execution evidence — workspace.json lists harnesses but no run output proves they executed.

Without execution evidence, the proof claims are unverifiable. The harnesses may be correct, but we have no proof they were actually run and passed.
