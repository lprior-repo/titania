# Trusted-base plan — tn-c6n

This bead introduces zero new trust markers: new test files use only exact
assertions over production constructors, with no assumed contracts, no
body-erasure markers, no admitted facts, and no reachability-only coverage.

One pre-existing marker is consumed, not created, and must be ledgered at
State 5 as `tn-c6n.T1`:

- Location: `verification/verus/receipt_schema.rs:8` (registered file,
  registry-owned, not modified by this bead).
- Marker: the assumed specification on
  `production::is_supported_receipt_schema_version`.
- Kind: source-linked assumed contract (classification per `tn-083n`).
- Compensating evidence: pinned-`verus` exit-0 run (`O-V6`) plus executable
  schema-gate tests (`O-P6`) which carry 100% of the behavior weight.
- Disposition: accepted as binding hygiene with zero behavior weight; the
  vacuous literal lemma in the same file is reported as vacuous.

No other ledger rows are planned. If proof writing surfaces any further
marker, the plan routes back here before State 5 closes.
