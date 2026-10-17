# Transcript — 007r State-7 bridge review (tn-c6n)

Verdict: APPROVED. The State-7 bridge maps all 7 behavior-affecting
proof obligations (`tn-c6n.O-P1`..`tn-c6n.O-P6`, `tn-c6n.O-V6`) to 7
bridge rows (`tn-c6n.B-P1`..`tn-c6n.B-P6`, `tn-c6n.B-V6`) with no
orphans, identical requirement/contract/verifier triples per row,
`behavior_affecting: true` throughout, independent behavior tests
disjoint from harness refs, `path::symbol` source refs matching
obligation targets, and `B-V6` held to binding-hygiene with behavior
weight on the paired invariants tests. One non-blocking note for
State-12: run both test binaries (plus verus) before flipping any
`mapping_status` to `verified`. Full criterion-by-criterion record in
`.beads/tn-c6n/proof-to-rust-review.md`.
