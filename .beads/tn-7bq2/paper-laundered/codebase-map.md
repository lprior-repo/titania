# Codebase Map (State 2 — Explore)

> Retroactively produced at State 3 closure; the contract work
> inherently covered the explore survey. Future v1.5 work references
> this map.

## Crate layout
- `crates/titania-core`: pure domain types (Lane, GateScope, RuleId,
  SkipReason, LaneOutcome, Finding, Report, Receipt, TargetProject,
  existing proof-seeds at `crates/titania-core/src/kani.rs`).
- `crates/titania-lanes`: lane shells; match-lane dispatch in
  `src/run_lane.rs`, `src/run_cargo_lane.rs`, `src/run_cargo/args.rs`,
  `src/run_lane_outcome.rs`, `src/artifact_writer.rs`.
- `crates/titania-aggregate`: lane artifact reader/writer; match-lane
  in `src/artifact_reader.rs::stem_to_lane`.
- `crates/titania-check`: CLI; match-lane in
  `src/main.rs::lane_stem`; match-scope in `scope_dir`. Args parser in
  `src/args.rs`.
- `crates/titania-policy`: rule explain catalog; recipients of new
  `PROOF_KANI_*` and `MUTANT_SURVIVED` entries.
- `crates/titania-dylint`: cdylib; rustc-driver collision risk with
  cargo-kani 0.67.0 (H6).
- `crates/titania-output`: shared output types.

## Files that touch the Lane enum
Production match-lane/match-scope sites identified (9):
1. `crates/titania-core/src/lane.rs` — enum definition + name/from_str.
2. `crates/titania-core/src/gate_scope.rs` — `*_LANES` consts + from_str.
3. `crates/titania-lanes/src/run_lane.rs` — dispatch table.
4. `crates/titania-lanes/src/run_cargo_lane.rs` — cargo sub-lane.
5. `crates/titania-lanes/src/run_cargo/args.rs` — `CargoLane::parse`.
6. `crates/titania-lanes/src/run_lane_outcome.rs` — outcome mapping.
7. `crates/titania-lanes/src/artifact_writer.rs` — artifact filename.
8. `crates/titania-aggregate/src/artifact_reader.rs` — `stem_to_lane`.
9. `crates/titania-check/src/main.rs` — `lane_stem`, `scope_dir`, run-lane.

Test files using `_ =>` arms: not exhaustive, no update needed.

## Tools & versions
- `cargo` (workspace pinned via rust-toolchain.toml): nightly-2026-04-27
- `cargo-kani` 0.67.0 (installed; reject `--package`/`--output-format` on `list`)
- `cargo-mutants` 27.0.0 (installed)
- `cargo-fuzz` (TBD; needed for v15.F1/v15.F2 fuzz lanes)
- `cargo-verus` (TBD; needed for v15.V1)
- `cargo-loom` (TBD; needed for v15.L1)
- `systemd-run` (host cgroup support; required for Kani)

## Risks captured in v1.5 spec §12
- H1 CBMC OOM, H2 cargo-mutants version drift, H3 bootstrap scope,
  H4 Kani unsupported-feature classification, H5 9-site match-lane
  blast radius, H6 rustc_driver/titania-dylint collision,
  H7 baseline schema drift, H8 cgroup variant across hosts,
  H9 nightly rustc version drift, H10 contract traceability.
