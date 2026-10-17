# Verifier lane matrix — tn-vr2n

| Seed | proptest (behavior) | verus | kani | flux-rs |
|---|---|---|---|---|
| P1 chokepoint | required (O-P1-BEH) | not-applicable | not-applicable | not-applicable |
| P2 static argv | required (O-P2-BEH) | not-applicable | not-applicable | not-applicable |
| P3 channel locks | required (O-P3-BEH) | not-applicable | not-applicable | not-applicable |

`proptest` here denotes the executable Rust behavior-test lane
(`cargo test` suites with exact assertions), matching prior-bead convention.
