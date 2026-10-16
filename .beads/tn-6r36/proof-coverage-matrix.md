# Proof Coverage Matrix — tn-6r36

13 planned obligations cover all 13 required lane decisions; 32
not_applicable lanes carry evidence refs. No seed left without an
executable lane: every behavior-affecting seed has at least one proptest
obligation, and the two deductive anchors (P1-Verus, V1-Verus) plus the
Kani edge obligation (P6) are planned.

| Obligation | Seed | Verifier | Status |
|------------|------|----------|--------|
| O-P1-PROP/VERUS | P1 | proptest/verus | planned |
| O-P2-PROP | P2 | proptest | planned |
| O-P3-PROP | P3 | proptest | planned |
| O-P4-PROP | P4 | proptest | planned |
| O-P5-PROP | P5 | proptest | planned |
| O-P6-PROP/KANI | P6 | proptest/kani | planned |
| O-V1-VERUS/PROP | V1 | verus/proptest | planned |
| O-P7-PROP | P7 | proptest | planned |
| O-F1-PROP/FUZZ | F1 | proptest/cargo-fuzz | planned |

Execution is deferred to follow-up implementation beads (classifier does
not exist yet); mapping status stays `planned` until state 12.
