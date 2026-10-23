# Changelog

All notable changes to titania-check are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-10-21

First release verified end to end against the v1 Definition of Done
(v1-spec §15): this repository now passes `titania-check --scope release`,
and all three Moon-orchestrated gate scopes emit typed Pass reports.

### Added

- **v1.0.0 release packaging**: every first-party crate bumped 0.1.0 →
  1.0.0; README install section updated from "coming with v1.0". Releases
  are Moon-built (`moon run :titania-build`); Moon is the only CI/release
  substrate — no GitHub Actions workflow ships with v1.0.0.
- **Release-scope dogfood evidence**: `titania:gate-edit`, `gate-prepush`,
  and `gate-release` each emit a typed Pass `Report` JSON plus a
  `QualityReceipt` with `schema_version=1` and source/lock/policy/
  toolchain digests (v1-spec §11.1).
- **`KaniVersionError` taxonomy** (`NonUtf8Output`, `UnparseableOutput`):
  malformed `cargo kani` version probes now fail the kani lane loudly
  instead of degrading to `Skipped`.
- Policy exception for `build_kani_diff_candidate` registered in
  `.titania/profiles/strict-ai/exceptions.toml` with owner, reason, expiry
  (2027-04-16), and review, per the exceptions mechanism (v1-spec §15.8).

### Changed

- Eliminated all `Result::ok()` erasures from the tool's own code (33 sites
  in 17 files): real failures now propagate as typed domain errors
  (`LaneFailure::Infra`, `KaniVersionError`, `MutantsLaneError`); genuine
  absence signals use explicit `let-else`, the unique form satisfying both
  the dylint `FUNC_RESULT_OK` rule and clippy pedantic's `manual_ok`
  family. Dylint lane green, 1186 tests / 0 failed (88 binaries).
- `CLIPPY_ARGS` now carries `-A clippy::missing_{errors,panics,safety}_doc`,
  matching the operative comment-free lint policy.
- cargo-deny lane runs `--offline` for a hermetic, deterministic gate;
  template `.moon/toolchains.yml` sets `bins: []` so Moon does not
  source-build `cargo-binstall`.
- Test-contract updates: `CANONICAL_LINT_COUNT` 20→17, doc-lint
  deny→allow per operative policy, exception counts 3→4, `cli_dispatch`
  `ENOTDIR` failure injection made root-proof.

### Fixed

- **Process-group kill bug** (`titania-lanes/src/command/process.rs`):
  timeout kills now pass `--` to `/bin/kill` so `-<pgid>` is not misparsed
  as a signal spec — it previously degraded to `kill(-1, SIGTERM)`.
- rustfmt re-layout of `deny_outcome` (pre-existing fmt drift); dropped a
  redundant `#![allow(missing_docs)]` in `titania-output`.

## [0.1.0] - 2026-10-16

First typed, evidence-bearing release of titania-check — the Moon-orchestrated
Rust QA gate for the `strict-ai` coding standard. Single binary
`titania-check` plus a co-located `titania-dylint` dynamic library.

### Added

- **Typed Rust lanes** behind the Moon DAG in `.moon/tasks/all.yml`:
  - Cargo-native: `fmt`, `compile`, `check`/`clippy`, `test`, `build`.
  - Supply chain: `cargo-deny` (advisories, bans, licenses, sources, dupes).
  - Specialized: `ast-grep` structural rules, `dylint` type-aware bypass /
    functional / panic-surface rules, `policy-scan` (TOML/env bypass
    detection), `panic-scan` (clean compatibility artifact), `source-length`
    (function/file line limits), `mutants` (residual mutation coverage).
- **`QualityReceipt`** with four content digests (source, lock, policy,
  toolchain) for reproducible, diffable, gateable evidence.
- **`strict-ai` policy** as the only profile: no `unwrap`/`expect`/`panic`/
  `todo`/`unimplemented`/`unreachable` in production, `forbid(unsafe_code)`
  workspace-wide, no unchecked indexing/slicing/casts/arithmetic, typed
  errors only, unowned suppressions rejected.
- **Three gate scopes**: `edit` (inner loop), `prepush` (PR expectation),
  `release` (on tag).
- **`titania-check setup-hermetic`** subcommand creating hermetic
  `CARGO_HOME`/`RUSTUP_HOME` symlinks (v1-spec §9.5).
- **`titania-check doctor`**, **`explain`**, and **`run-lane`** subcommands.
- **`cargo generate titania/template`** workspace adoption template with a
  pinned `nightly-2026-04-27` toolchain and the strict-ai gate pre-wired.
- **Moon DAG** (`setup-hermetic` → lane tasks → aggregate) with content-hash
  caching.
- **Distribution**: version tags (`v*`) with Moon-built release binaries
  (`moon run :titania-build --release`) for the `titania-check` binary and
  the `titania-dylint` cdylib.
- **Co-located `titania-dylint` cdylib** (`libtitania_dylint.so` / `.dylib` /
    `titania_dylint.dll`) for type-aware lint scans.

### Changed

- Aligned lane exit codes with v1-spec §12: `0` Pass, `1` Reject, `3`
  InputError, `>=4` Internal error.
- Non-clippy cargo lanes (`fmt`/`compile`/`test`/`build`) now emit
  `LaneOutcome::Failed { LaneFailure::Tool }` (gate failure, exit 1) on
  nonzero exit instead of synthetic findings, per v1-spec §5. Clippy remains
  the only cargo lane that normalizes output into typed `CLIPPY_*` findings.

### Fixed

- CLI help and input-error guidance now list the `full` scope and the `kani`
  and `mutants` lane spellings accepted by the parser.
- `OutcomeError::Argv0Mismatch` display is bounded (200 bytes per payload,
  control characters sanitized), so hostile `argv[0]`/executable values can
  no longer produce multi-kilobyte unbounded error strings.

[Unreleased]: https://github.com/lprior-repo/titania/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/lprior-repo/titania/compare/v0.1.0...v1.0.0
[0.1.0]: https://github.com/lprior-repo/titania/tree/v0.1.0
