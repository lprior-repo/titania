# Truth-serum report — tn-083n (self-audit; single-operator debt applies)

## Claims verified against raw evidence

- CLAIM "10 behavior tests pass": VERIFIED — `cargo test --test
  trust_binding_audit` → `10 passed; 0 failed` (observed).
- CLAIM "touched-crate suite fully green": VERIFIED — `cargo test -p
  titania-lanes --all-features` → 29 suites, 366 passed / 0 failed
  (observed; grep-counted, no `0 failed` exceptions).
- CLAIM "strict clippy clean": VERIFIED — source-only gate per AGENTS.md
  exits 0 (observed); 9 intermediate findings were fixed, not waived.
- CLAIM "all source files < 300 lines": VERIFIED — audit 286, classify 120,
  decide 119, mod 30, report 180, scan 187 (`wc -l` observed).
- CLAIM "fixture mirrors real files": VERIFIED — grep of real
  `receipt_schema.rs` confirms `#[path` L3, `assume_specification` L8,
  `production::` refs, `2 == 2` ×2; smoke marker confirmed at
  `formal_setup_smoke.rs:1` (observed).
- CLAIM "failing-before reproduction exists": VERIFIED — pre-implementation
  run failed with E0432 (observed).

## Claims NOT made

- No Verus/Kani/proof execution is claimed; the module consumes
  caller-supplied evidence records only.
- No `moon ci` pass is claimed (not run; out of scope).
- No validator pass is claimed (`tools/go-skill-v9-validate` absent).
- No independent review is claimed (self-review only).

## Adversarial notes (black-hat pass)

- B1: `contains()`-based scanning can false-positive on markers inside
  trailing comments or string literals → direction is fail-closed (extra
  block), acceptable; recorded.
- B2: `path_binds_production` substring match could over-accept a
  similarly-named wrong target → still gated by trust-marker and evidence
  stages; recorded as residual.
- B3: `trusted` detection covers `#[trusted` / `::trusted` spellings only;
  novel spellings would be missed → recorded as residual.
- B4: Evidence-record provenance is the caller's duty; the audit cannot
  detect forged records → documented boundary, not a defect.

## Verdict

ACCEPT with residuals: tn-083n.F-IND-1 (single-operator review),
validator-absent, B2/B3 scanner-precision notes. No hallucinated evidence;
all numbers above are observed command output.
