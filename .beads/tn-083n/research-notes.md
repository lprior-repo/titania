# Research notes — tn-083n

## Files read (exact refs)

- `contracts/proof_obligations.yaml:1-4` — registry v1; exactly two targets:
  `verification/verus/formal_setup_smoke.rs`, `verification/verus/receipt_schema.rs`.
- `verification/verus/receipt_schema.rs:3-4` — `#[path =
  "../../crates/titania-core/src/receipt/schema.rs"] mod production;`
  (source linkage import, lines 3-4).
- `verification/verus/receipt_schema.rs:9-12` —
  `pub assume_specification[production::is_supported_receipt_schema_version]`
  with `ensures supported == (schema_version == 2)` (assumed contract, not a
  verified body).
- `verification/verus/receipt_schema.rs:14-25` — `schema_accepts_current`
  / `schema_rejects_future` call the assumed spec; still downstream of the
  assumption.
- `verification/verus/receipt_schema.rs:27-31` —
  `proof fn schema_version_literal_is_stable() ensures 2 == 2` (tautology;
  proves nothing about the production body).
- `verification/verus/formal_setup_smoke.rs:1` —
  `// titania-verus-binding: fixture-smoke` (explicit smoke marker);
  lines 7-12 prove `1 + 1 == 2` only.
- `crates/titania-core/src/receipt/schema.rs:24-26` —
  `pub(super) const fn is_supported_receipt_schema_version` production body
  (`schema_version == RECEIPT_ENVELOPE_SCHEMA_VERSION`). Note: `pub(super)`
  — the `assume_specification` names a non-`pub` item, reinforcing that the
  bridge is source linkage, not an executable-behavior proof.
- `crates/titania-lanes/src/lib.rs:60-194` — `Finding::new(rule, path, line,
  message)` + `LaneReport::{push, extend_finding, findings, is_clean}`.
- `crates/titania-lanes/src/policy_scan.rs` + `policy_scan/exceptions.rs` —
  scanner composition + waiver/exception filtering conventions reused by the
  new audit (`scan_*` pure fns, `RuleIdError` errors, findings into
  `LaneReport`).
- `crates/titania-core/src/finding/repair_catalog.rs:138-146` — unknown rule
  ids map to `requires_human_review`; new `TRUST_*` rule ids are safe without
  catalog edits.

## Patterns searched

- `proof.?registr|ProofRegistr|obligation` in `crates/` — no hits: tn-f7ti's
  registry audit is not present in this checkout (uncommitted elsewhere), so
  this bead takes obligations as typed caller input instead of re-reading
  YAML. No duplication.
- `tools/go-skill-v9-validate` — absent (no `tools/` dir in checkout).
  Blocker recorded; manual gate checks substitute.

## Research questions

- Consumer-visible regression proving this control: integration behavior
  tests in `crates/titania-lanes/tests/trust_binding_audit.rs` drive the
  public `trust_binding` API with fixtures mirroring the real
  `receipt_schema.rs` / `formal_setup_smoke.rs` content. No second
  convention added (no CLI, no YAML reader).

## Scope reduction

Smallest correct change: one new pure module `titania-lanes::trust_binding`
(`mod.rs`, `classify.rs`, `scan.rs`, `audit.rs`, each < 300 lines) +
one integration test file. Consumers pass obligations, scanned file
contents, waivers, evidence records, and current production hashes; the
audit classifies and fails closed. Verus execution itself is out of scope
(tool-absent lane; evidence records are caller-supplied and hash-checked).
