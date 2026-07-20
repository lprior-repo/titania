# Proof Coverage Matrix

| Req-ID | Contract Clause | Seed | proptest | kani | verus | loom | cargo-fuzz |
|--------|-----------------|------|----------|------|-------|------|------------|
| v15-REQ-LANE-ROUNDTRIP | 3.1 Lane round-trip | v15.P1 | ✓ (P1) | - | - | - | - |
| v15-REQ-GATESCOPE-ROUNDTRIP | 3.2 GateScope round-trip | v15.P2 | ✓ (P2) | - | - | - | - |
| v15-REQ-KANI-HARNESS-ID | 3.3 KaniHarnessId validation | v15.P3 | ✓ (P3-proptest) | ✓ (P3-kani) | - | - | - |
| v15-REQ-KANI-HARNESS-SERDE | 3.3 KaniHarnessId serde | v15.P4 | ✓ (P4) | - | - | - | - |
| v15-REQ-MUTANT-ID | 3.4 MutantId invariants | v15.P5 | ✓ (P5-proptest) | - | - | - | - |
| v15-REQ-MUTANT-ID-OPERATOR | 3.4 operator closed-set | v15.V1 | - | - | ✓ (V1-verus) | - | - |
| v15-REQ-MUTANTS-BASELINE-LOAD | 5 load errors | v15.P6 | ✓ (P6) | - | - | - | - |
| v15-REQ-MUTANTS-DIFF | 4.3 set difference | v15.P7 | ✓ (P7-proptest) | - | - | - | - |
| v15-REQ-MUTANTS-DIFF-ZERO-NEG | 4.3 zero-negatives | v15.K2 | - | ✓ (K2-kani) | - | - | - |
| v15-REQ-MUTANTS-EXPIRY | 12-H3 expired | v15.P8 | ✓ (P8) | - | - | - | - |
| v15-REQ-SKIP-REASON-TOOL | 7 skip_state | v15.P9 | ✓ (P9) | - | - | - | - |
| v15-REQ-LANE-NAME | 3.1 name uniqueness | v15.P10 | ✓ (P10) | - | - | - | - |
| v15-REQ-LANE-SERDE | 11 serde round-trip | v15.P11 | ✓ (P11) | - | - | - | - |
| v15-REQ-KANI-LANE-NAME | 4.2 harness naming | v15.K1 | - | ✓ (K1-kani) | - | - | - |
| v15-REQ-ATOMIC-BASELINE-LOAD | 8 atomic load | v15.L1 | - | - | - | ✓ (L1-loom) | - |
| v15-REQ-INVENTORY-PARSE | 12-H9 inventory parse | v15.F1 | - | - | - | - | ✓ (F1-fuzz) |
| v15-REQ-OUTCOMES-PARSE | 12-H2 outcomes parse | v15.F2 | - | - | - | - | ✓ (F2-fuzz) |

## Coverage

- 17/17 requirements have at least one lane decision.
- Default Rust profile (proptest, kani, verus) covers 16/17 obligations.
- Loom covers 1, cargo-fuzz covers 2.
- No verifier is silently skipped: every seed has explicit applicability.

## Cross-lane balance
- proptest: 11 obligations (pure core)
- kani: 4 obligations (bounded symbolic execution)
- verus: 1 obligation (deductive hot-path)
- loom: 1 obligation (concurrency)
- cargo-fuzz: 2 obligations (hostile input)

Total planned obligations: 18 lane decisions, 18 proofs.
