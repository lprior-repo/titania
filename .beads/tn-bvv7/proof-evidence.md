# Proof Evidence — tn-bvv7

O1 (SEED-S1, proptest): `tn_bvv7_fq_bypass_property`, 64 cases, PASS.
Command: `cargo test -p titania-lanes --lib tn_bvv7_property`
(exit 0; raw log `evidence/proptest-obligations.txt`).

O2 (SEED-S2, proptest): `tn_bvv7_effect_boundary_property`, 64 cases,
PASS. Same command and log.

Both harnesses execute directly against production detector functions
(`detect_core_*` in `rules/detectors.rs`); no logic is duplicated into
a model, so divergence between model and production is impossible by
construction. Exact-assertion companions (13 detector tests, 2 scope
tests) and lane-level fixtures provide the paired negative/positive
controls. Marker scan over this file, the writer report, and the proof
review found zero proof-debt markers; the base ledger file holds the
attesting row with grep evidence.
