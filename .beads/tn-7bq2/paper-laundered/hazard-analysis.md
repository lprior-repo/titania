# v1.5 Hazard Analysis

> Safety and reliability hazards specific to the v1.5 milestone,
> classified by hazard class, mapped to the mitigation each contract
> layer imposes. Not exhaustive of all v1.0 hazards; v1.0 keeps its
> existing hazard register and is referenced.

## H1: CBMC OOM

- **Class**: Resource exhaustion / unbounded solver.
- **Description**: CBMC (Kani's solver) consumes memory proportional
  to the unwinding depth and the concrete size of symbolic Vecs.
  Without a cgroup cap a single harness can exhaust 32 GB+ and
  crash the lane.
- **Affected surface**: titania-lanes/src/run_lane_kani.rs, every
  cargo-kani invocation.
- **Mitigation**:
  - cgroup-cap every cargo-kani run: `MemoryMax=24G`,
    `MemorySwapMax=0`, `-j 1`.
  - Per-package enumeration; one OOM does not poison the lane.
  - `PROOF_KANI_BLOCKED` finding on OOM; gate stays green unless
    every harness blocks.
  - Disable `--coverage -Z source-coverage` (R4-adjacent: prevents
    doubling CBMC memory through `--source-coverage`).

## H2: cargo-mutants version drift

- **Class**: Tool version mismatch.
- **Description**: cargo-mutants 27.x changed `--in-place` from a
  `--flag=value` flag to a `--flag` only flag; the --output semantics
  changed; the JSON format evolved. A patch release may break the
  baseline compare logic silently.
- **Affected surface**: titania-lanes/src/run_lane_mutants.rs, the
  MutantsBaseline diff algorithm.
- **Mitigation**:
  - Pin cargo-mutants to version `=27.x` in the operator-side install
    instructions; lane emits a warning when installed version differs.
  - `mutants.out/outcomes.json` schema_version check — surface errors
    as `MutantsBaselineError::SchemaVersion` so a future version is
    loud rather than silent.

## H3: First-run baseline bootstrap surface

- **Class**: Test-survivor scope.
- **Description**: Pre-impl evidence shows `cargo mutants --check`
  (build-only) surfaces 236 build-survivors in titania-core. Full
  test-mode survivors are fewer because tests catch more, but the
  bootstrap still requires operator work. The hazard is the operator
  abandoning the bootstrap with a partially-killed or partially-
  accepted baseline, breaking the zero-survivor discipline.
- **Affected surface**: `.titania/profiles/strict-ai/mutants.baseline.json`,
  operators.
- **Mitigation**:
  - Bootstrap recipe is documented in
    `.beads/tn-7bq2.1/` (this contract) and `scripts/dev/mutants-bootstrap.sh`
    (impl work in `tn-7bq2.4`).
  - Baseline entries require `accepted-by-rule: mutant-accept/<owner>/<reason>/<expiry>`,
    same as v1.0 policy exceptions; expiry is required (no infinite
    baselines).
  - Each entry's `expires_on` is enforced by the lane: expired
    entries trigger a finding (`MUTANT_BASELINE_EXPIRED`) so the
    baseline cannot slowly rot.
  - Zero-survivor contractual promise: future mutations can
    accumulate, but every baseline entry has an explicit owner.

## H4: cargo-kani 0.67.0 unsupported features

- **Class**: Tool-feature surface.
- **Description**: Function contracts (`-Z function-contracts`) and
  stubs (`-Z stubbing`) are experimental in cargo-kani 0.67.0. They
  emit "unsupported feature" lines that look like failures to a naive
  pass/fail classifier.
- **Affected surface**: titania-lanes/src/run_lane_kani.rs classifier.
- **Mitigation**:
  - Lane classifies `unsupported feature` lines into the
    `PROOF_KANI_UNSUPPORTED` finding; gate stays green.
  - Optionally, the harness inventory JSON records which harnesses
    depend on which experimental features; the lane warns (but does
    not fail) on harnesses that flip between supported and
    unsupported.

## H5: Lane enum extension blast radius

- **Class**: API surface.
- **Description**: Adding `Lane::Kani`, `Lane::Mutants`, and
  `GateScope::Full` is a total-enum contract change. The 9
  production match-lane/match-scope sites all need new arms. A miss
  at any site compiles, but the build/test runs catch it via
  exhaustiveness asserts.
- **Affected surface**: titania-core (Lane, GateScope,
  SkipReason), 9 production files listed in `.evidence/v1.5/spec.md`
  §11.
- **Mitigation**:
  - Pre-impl `cargo check --workspace --all-targets` plus
    `cargo clippy --workspace --all-targets -- -D warnings`
    after each batch of variant additions catches every missed
    arm.
  - Existing tests that use `_ =>` arms are unaffected.

## H6: rustc_driver collision with titania-dylint

- **Class**: Tool interaction.
- **Description**: cargo-kani 0.67.0 depends on a specific nightly
  rustc_driver; titania-dylint depends on a nightly rustc_driver.
  When `--workspace` is passed and dylint is enabled, the two
  rustc_drivers collide.
- **Affected surface**: Kani lane.
- **Mitigation**:
  - Lane enumerates crates and runs `cargo kani -p <pkg>` per crate
    without `--workspace`.
  - Kani runs before dylint in the `gate-full` composite (the
    collision surfaces faster).
  - The collision is reported as `PROOF_KANI_BLOCKED` if it occurs;
    gate stays green unless every harness blocks.

## H7: Mutants-baseline file drift

- **Class**: Schema drift.
- **Description**: The mutants-baseline JSON could be edited manually
  in repositories. Without a schema_version guard, downstream tools
  may silently mis-parse.
- **Mitigation**:
  - The `MutantsBaseline::load` enforces `schema_version == 1`.
  - Wrong version → `MutantsBaselineError::SchemaVersion{got, expected}`.
  - Lane surfaces this as `MUTANT_BASELINE_MISSING` for operator
    remediation. (Same shape as the bootstrap-missing finding.)

## H8: cgroup variant across hosts

- **Class**: Platform.
- **Description**: `systemd-run --user --scope` exists on Linux with
  systemd. macOS and Windows runners will not honor it.
- **Mitigation**:
  - The lane enumerates the host's cgroup capability before
    wrapping; if absent, it spawns the process directly with
    `--output-format regular` and emits a typed warning finding
    in `PROOF_KANI_PASS` row metadata: `host-cgroup: absent`.
  - CI matrix documents the supported host (Linux + systemd). The
    lane is documented as Linux-first; macOS/Windows fall back
    with a documented run-time cost.

## H9: Rust nightly rustc_version drift

- **Class**: Toolchain pin.
- **Description**: The repo pins nightly via rust-toolchain.toml;
  upstream nightly can change behavior of `#[unstable]` features
  that titania uses. cargo-kani 0.67.0 depends on a specific
  rustc — see H6.
- **Mitigation**:
  - Document accepted rustc range in the lane; refuse with
    `PROOF_KANI_BLOCKED` if out of range.
  - The lane reads `rustc --version` and emits a typed finding
    when out of range.
  - CI uses `frozen` + the pinned toolchain.

## H10: Traceability across the v1.5 contract

- **Class**: Documentation.
- **Description**: Spec, design, and contract can drift.
- **Mitigation**:
  - Spec lives in `.evidence/v1.5/spec.md`.
  - Domain model, type contracts, workflow, error taxonomy,
    boundary map, hazard analysis live in
    `.beads/tn-7bq2.1/*.md` (this directory).
  - proof-seeds.jsonl is generated next.
  - `traceability-matrix.jsonl` ties each proof seed to the
    spec section it implements.
