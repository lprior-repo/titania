# Proof coverage matrix — tn-vr2n

| Requirement | Seed | Lane | Obligation | Evidence command |
|---|---|---|---|---|
| tnvr2n-REQ-CHOKEPOINT | tnvr2n.P1 | proptest required | tnvr2n.O-P1-BEH | cargo test -p titania-lanes --test run_cargo_invocation |
| tnvr2n-REQ-STATIC-ARGV | tnvr2n.P2 | proptest required | tnvr2n.O-P2-BEH | cargo test -p titania-lanes --lib run_cargo |
| tnvr2n-REQ-CHANNEL-LOCKS | tnvr2n.P3 | proptest required | tnvr2n.O-P3-BEH | cargo test -p titania-lanes (env/config/scrub suites) + titania-check cli_dispatch |

verus/kani/flux-rs: not-applicable per lane decisions (evidence refs in
verifier-lane-decisions.jsonl).
