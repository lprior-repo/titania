# Proof strategy — tn-c6n

Defense in depth for six constructor-level requirements with zero new trust
markers and zero mirror types.

## Lane posture

- `proptest` (required, all 6 seeds): acceptance/rejection properties over the
  full input classes (`String`, `u32 × u32`, `u64 × u64`, path strings, `u32`
  versions) calling production constructors directly. This is the behavior
  evidence lane.
- `verus` (required for S6 only): pinned-verifier execution of the registered
  `verification/verus/receipt_schema.rs` binding. Claimed as binding hygiene
  (file verifies clean), never as constructor-body proof. The `2 == 2` lemma
  is classified vacuous with zero behavior weight.
- `verus` (not applicable, S1–S5): constructors take heap-allocated `String`
  inputs through serde-derived impls, outside the verifier fragment; no
  production-body spec is attempted, so no laundering is possible.
- `kani` (not applicable, all seeds): bounded-proof execution is owned by
  `tn-7bq2` per delegation, and the tool is absent on this host
  (`evidence/kani-absent.log`, exit 1). Rejection behavior is covered by
  exhaustive proptest input classes instead.
- `flux-rs` (not applicable, all seeds): the crate carries no refinement
  annotations and the serde-backed constructors sit outside the refinement
  fragment; annotating production signatures would be an API change outside
  this bead's scope.

## Obligations

Seven planned obligations: `tn-c6n.O-P1`…`tn-c6n.O-P6` (proptest) and
`tn-c6n.O-V6` (verus binding run). All `required: true`, all
behavior-affecting, all with executable commands and expected evidence. No
waivers (behavior-affecting waivers are invalid; the single waiver-candidate
row is the explicit no-waiver record).

## What closes this bead

Green `cargo test` runs for both new test targets with exact assertions,
pinned-`verus` exit 0 on the registered binding file, strict gates in State
12, and accepted independent reviews. Anything less stays open.
