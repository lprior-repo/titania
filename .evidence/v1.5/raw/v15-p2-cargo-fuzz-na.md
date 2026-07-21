# Non-applicability evidence for v15.P2 verifier=cargo-fuzz

Reason: v1.5 lane profile selects proptest (empirical) + Kani/Verus (deductive bounded).
Verifier 'cargo-fuzz' is not the primary tool for this obligation per spec §4 and proof-coverage-matrix.

Compensation:
- proptest covers the empirical surface
- Kani/Verus covers the bounded/deductive surface where applicable
