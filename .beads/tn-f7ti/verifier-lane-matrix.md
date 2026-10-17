# Verifier lane matrix — tn-f7ti

| Seed | verus | kani | flux-rs | proptest |
|------|-------|------|---------|----------|
| S1 registered fail-closed (REQ-REGISTRY-COMPLETE / C1) | required (PO-RC-verus) | required (PO-RC-kani) | required (PO-RC-flux) | required (PO-RC-proptest) |
| S2 smoke/disposition (REQ-SMOKE-DISPOSITION / C2) | required (PO-SD-verus) | required (PO-SD-kani) | required (PO-SD-flux) | required (PO-SD-proptest) |

All eight lanes are required. Every lane names its planned obligation;
every obligation targets production files or the behavior suite directly;
no duplicated harness models are used, so no copy/reality bridge risk
exists. Lane detail lives in verifier-lane-decisions.jsonl; independent
dispositions live in verifier-lane-review.jsonl (all accepted).
