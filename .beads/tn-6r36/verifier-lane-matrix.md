# Verifier Lane Matrix — tn-6r36

45 decisions = 9 seeds x {proptest, verus, kani, flux-rs, cargo-fuzz}.

| Seed | proptest | verus | kani | flux-rs | cargo-fuzz |
|------|----------|-------|------|---------|------------|
| P1 bounded-for | required O-P1-PROP | required O-P1-VERUS | n/a structural | n/a no refinements | n/a no codec |
| P2 approved-recovery | required O-P2-PROP | n/a owned by P5 | n/a no arithmetic | n/a no refinements | n/a no codec |
| P3 combinator-retry | required O-P3-PROP | n/a single guard | n/a no numerics | n/a no refinements | n/a no codec |
| P4 erasure | required O-P4-PROP | n/a dual of P2 | n/a payload-free | n/a no refinements | n/a no codec |
| P5 approval-validation | required O-P5-PROP | n/a total order | n/a derived Ord | n/a no refinements | n/a structured TOML |
| P6 capped-bound | required O-P6-PROP | n/a one guard | required O-P6-KANI | n/a plain u64 | n/a one guard |
| V1 critical-gate | required O-V1-PROP | required O-V1-VERUS | n/a no counters | n/a no refinements | n/a enum matrix |
| P7 critical-source | required O-P7-PROP | n/a closed enum | n/a no payload | n/a no refinements | n/a enum parse |
| F1 hostile-toml | required O-F1-PROP | n/a total return | n/a Box<str> | n/a no refinements | required O-F1-FUZZ |

Loom: excluded, no concurrency surface. Full decision text with evidence
refs lives in verifier-lane-decisions.jsonl; dispositions in
verifier-lane-review.jsonl (45/45 accepted).
