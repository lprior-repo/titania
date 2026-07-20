# Changelog

All notable changes to titania-check are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- CLI help and input-error guidance now list the `full` scope and the `kani`
  and `mutants` lane spellings accepted by the parser.

## [0.1.0] - 2026-07-12

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
- **Distribution**: `cargo binstall titania-check` support
  (`[package.metadata.binstall]`) and a GitHub Actions release workflow
  (`.github/workflows/release.yml`) building the `titania-check` binary and
  the `titania-dylint` cdylib across a Linux/macOS/Windows × x86_64/aarch64
  matrix.
- **Co-located `titania-dylint` cdylib** (`libtitania_dylint.so` / `.dylib` /
    `titania_dylint.dll`) for type-aware lint scans.

### Changed

- Aligned lane exit codes with v1-spec §12: `0` Pass, `1` Reject, `3`
  InputError, `>=4` Internal error.
- Non-clippy cargo lanes (`fmt`/`compile`/`test`/`build`) now emit
  `LaneOutcome::Failed { LaneFailure::Tool }` (gate failure, exit 1) on
  nonzero exit instead of synthetic findings, per v1-spec §5. Clippy remains
  the only cargo lane that normalizes output into typed `CLIPPY_*` findings.

[Unreleased]: https://github.com/lprior-repo/titania/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/lprior-repo/titania/releases/tag/v0.1.0
