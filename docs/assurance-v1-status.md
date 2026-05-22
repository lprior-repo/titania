# Assurance V1 Status

## Current State

The current `assure` implementation is a TenantAccess-v1 scaffold, not a finished assurance gate.

It currently provides:

- `cargo xtask assure contract-lint`
- `cargo xtask assure oracle-check`
- `cargo xtask assure path-check`
- typed finite decision-table data structures
- TenantAccess fixture for `JwtVerified + tenant claim relation + membership fact`
- finite path totality and overlap checking
- oracle, evidence, assumption, and claim-ceiling data models
- tests for overlap, uncovered valuation, unreachable paths, generated-oracle rejection, claim ceilings, and evidence status

## Explicit Non-Claims

The current scaffold does not yet provide:

- real code generation
- real Kani harness generation
- Moon CI target integration
- real landing gate closure
- generated artifact digest validation
- Git/signature verification for oracle provenance
- real oracle replay validation against inputs and expected outcomes
- full authentication correctness

## Known Review Findings

The hostile review is accepted. The current implementation must not be treated as a trustworthy TenantAccess assurance compiler until these are fixed:

- `oracle-check` must verify artifact lineage instead of trusting self-attested fields.
- `oracle-check` must replay oracle inputs through the independent evaluator and compare expected outcomes.
- `contract-lint` must stop being a pass-only scaffold and must validate concrete contract artifacts.
- Evidence acceptance must require trusted runner identity, non-empty tool versions, digest freshness, and Moon/CI provenance.
- Claim ceilings and the JwtVerified assumption ledger must participate in pass/fail, not just exist as data.
- Moon targets must exist before any CI/landing claim is made.
- Typed expression validation must reject unknown variables and invalid enum variants before path evaluation.

## Next Build Line

Do not broaden scope beyond TenantAccess-v1 until the deterministic gates above are real. Flux, Verus, TLA+, Red Queen, manual QA, and broad auth correctness remain extension slots only.
