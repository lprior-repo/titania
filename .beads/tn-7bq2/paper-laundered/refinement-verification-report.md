reviewer_skill: formal-verifier
reviewer_invocation_id: s12b.tn-7bq2.2.refinement-verifier
writer_invocation_id: s11.tn-7bq2.2.holzman-rust

# Refinement Verification Report (State 12)

## Status
**STATUS: APPROVED**

## Bridge alignment

Every `rust-refinement-obligation/v1` row has:
- source_refs (concrete `path::symbol`)
- behavior_test_refs (independent executable behavior checks)
- refinement_harness_refs (separate file with proof-bearing harness)
- evidence_command captured exit 0
- mapping_status: verified

## Source/test/refinement independence

- No test file imports `kani::*` or `loom::*` or `cargo_fuzz::*`.
- Kani harnesses live under `cfg(kani)`; tests under cargo test default.
- Loom tests use `cfg(loom)` indirection in artifact_writer.
- fuzz targets under `fuzz/fuzz_targets/` are not pulled into the
  regular test binary; they run via `cargo +nightly fuzz run`.

## Bridge rows not behavior-affecting

This milestone: 0 behavior-affecting obligations (every rust-refinement row is `behavior_affecting: false`). All tests are independent of verifier harnesses.

## Approval
PASS for State 12.
