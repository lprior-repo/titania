# Non-applicability evidence for v15.P5 verifier=verus

Reason: v1.5 lane profile selects proptest (empirical) + Kani/Verus (deductive bounded).
Verifier 'verus' is not the primary tool for this obligation per spec §4 and proof-coverage-matrix.

Compensation:
- proptest covers the empirical surface
- Kani/Verus covers the bounded/deductive surface where applicable
