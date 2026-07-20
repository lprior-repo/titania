# Verifier Lane Matrix

| Verifier | Required seeds | Optional seeds | Notes |
|----------|----------------|----------------|-------|
| proptest | 11 (P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11) | none | Pure newtypes + serde round-trip; empirical |
| kani | 4 (P3, K1, K2, and one kani-obligation for v15.P5 shadow) | none | Bounded string/Vec; cgroup-capped |
| verus | 1 (V1) | none | Hot-path closed-set operator |
| loom | 1 (L1) | none | Atomic rename concurrency |
| cargo-fuzz | 2 (F1, F2) | none | Hostile JSON input; 300s budget |
| flux-rs | 0 | 0 | N/A for v1.5 |
| miri | 0 | 0 | v2.5 |

Default Rust profile covers proptest + kani + verus. Loom and cargo-fuzz are conditional additions.

## Lane selection rationale

- **proptest** dominates because most v1.5 obligations are pure-newtype or
  set-difference invariants — well-suited to proptest generation.
- **Kani** targets the four obligations where bounded symbolic execution
  adds clear value over proptest: harness identifier validation, harness
  naming mapping, set-difference zero-negatives, and MutantId validator.
  Kani is heavy per package; we cap at four obligations.
- **Verus** targets exactly one obligation: `MutantId` operator closed-set.
  This is a hot-path safety invariant where deductive proof is the
  strongest evidence.
- **loom** for the atomic baseline load (concurrency).
- **cargo-fuzz** for the two hostile-input parse surface.

## Verifier disable policy
- No `--no-*checks` or `--prove-safety-only` flags permitted on Kani runs.
- No `cargo mutants --check`; full test-mode required.
