# Proof Strategy — tn-6r36

Profile: default Rust behavior lanes (verus, kani, flux-rs, proptest) plus
cargo-fuzz for the parser boundary. Loom excluded: the classifier is pure
sync with no concurrency surface.

Shape of the argument, per seed:

- P1 (bounded-for): proptest corpus over input shapes + Verus postcondition
  (permitted iff claimed-bound and cause preserved). Kani/Flux n/a with evidence.
- P2/P4 (recovery/erasure dual): proptest approval matrix; validation proof
  owned by P5. Kani/Verus/Flux n/a with evidence.
- P3 (combinator retry): proptest negative-universal over syntax forms with
  per-form presence assertion (non-vacuity). Deductive lanes n/a.
- P5 (approval validation): proptest date/field boundary matrix against
  titania-policy Exception shape.
- P6 (capped bound): proptest boundaries + Kani exhaustive u8 edges with
  cover(0)/cover(MAX) witnesses.
- V1 (critical gate): Verus unshippability over closed enums + full 2x3
  proptest matrix.
- P7 (critical source): proptest closed-source resolution + rejection corpus.
- F1 (hostile TOML): proptest malformed corpus now; libFuzzer harness
  (O-F1-FUZZ) once cargo-fuzz lands as dev-dep.

Obligations target the planned classifier (`policy_scan/classifier.rs`) to
be built by tn-ebn9/tn-8obr/tn-pfp1; no obligation executes in this bead.
