# Proof coverage matrix — tn-f7ti

| Requirement | Clause | Seed | Obligations | Bridge | Ledger |
|-------------|--------|------|-------------|--------|--------|
| REQ-REGISTRY-COMPLETE | C1-registered-fail-closed | S1 | PO-RC-verus, PO-RC-kani, PO-RC-flux, PO-RC-proptest | RRO-RC-verus, RRO-RC-kani, RRO-RC-flux, RRO-RC-proptest | VL-PO-RC-*, VL-RRO-RC-* (all PASS) |
| REQ-SMOKE-DISPOSITION | C2-smoke-never-proof | S2 | PO-SD-verus, PO-SD-kani, PO-SD-flux, PO-SD-proptest | RRO-SD-verus, RRO-SD-kani, RRO-SD-flux, RRO-SD-proptest | VL-PO-SD-*, VL-RRO-SD-* (all PASS) |

Every behavior-affecting proof obligation has a verified Rust refinement
bridge with concrete source refs, independent behavior tests, disjoint
refinement harness refs, and executed command evidence.
