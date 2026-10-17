# Implementation — tn-083n

## Files changed

- `crates/titania-lanes/src/trust_binding/mod.rs` (new, 30 lines): module
  docs + facade re-exports.
- `crates/titania-lanes/src/trust_binding/classify.rs` (new, 120 lines):
  `TrustMarker` (5 markers + `all()`/`name()`), `BlockReason` (6 reasons +
  `rule_id()`), `BindingVerdict` (`VerifiedProduction` / `SourceLinked` /
  `Blocked` + predicates).
- `crates/titania-lanes/src/trust_binding/scan.rs` (new, 187 lines):
  `FileSignals` + `ScannedFile` + `scan_proof_content()` pure line scanner
  (path attrs + target extraction, per-marker line lists, production-ref /
  proof-fn counts, smoke detection via `fixture-smoke` or smoke path).
- `crates/titania-lanes/src/trust_binding/decide.rs` (new, 119 lines):
  `audit_one` decision table, evidence staging, waiver coverage,
  file/evidence lookup.
- `crates/titania-lanes/src/trust_binding/report.rs` (new, 180 lines):
  `finding_for_blocked` + per-reason messages + `BlockedCtx` bundling.
- `crates/titania-lanes/src/trust_binding/audit.rs` (new, 286 lines):
  `Obligation`, `VerifierEvidence` (+`trusted_base()`), `Waiver`
  (+`is_explicit()`), `AuditInputs`, `AuditResult`, `audit_all`.
- `crates/titania-lanes/src/lib.rs` (+1 line): `pub mod trust_binding;`.
- `crates/titania-lanes/tests/trust_binding_audit.rs` (new, 10 behavior
  tests mapping 1:1 to acceptance criteria).

All source files < 300 lines (audit.rs was split at 557 → audit/decide/report).

## Contract clauses satisfied

- Report source linkage independently from executable-behavior
  verification: distinct `BindingVerdict` variants by construction.
- Block unsupported claims / unapproved markers / drift: `Blocked` +
  `TRUST_*` findings; waivers keep `SourceLinked`, never verify.
- `#[path]` + `assume_specification` = linkage, not body proof (tested on a
  fixture mirroring the real `receipt_schema.rs:3-12,27-31`).
- Tautology (`2 == 2`) discharges nothing (`VacuousProof`); smoke fixtures
  never satisfy obligations (`SmokeOnly`); stale revisions are drift
  (`MirrorDrift`); failed runs prove nothing (`MissingEvidence`).

## Design notes

- Pure core (no I/O, no verifier execution, no YAML parsing): callers supply
  typed inputs; absent tools/evidence fail closed by construction.
- Holzman discipline: zero `unwrap`/`expect`/`panic`/`todo`, no indexing or
  string slicing, saturating counters, `#[must_use]` on value-returning
  public functions, `# Errors` docs on every fallible function including
  private ones (`check-private-items`).
- Unknown `TRUST_*` rule ids resolve to `requires_human_review` hints via
  the existing catalog fallback; no catalog edit needed.
