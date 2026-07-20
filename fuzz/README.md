# titania-fuzz

cargo-fuzz harnesses for the v1.5 titania-check pure-core parsers
(`titania_core::KaniInventory`, `titania_core::MutantsOutcomes`,
`titania_core::MutantsRecords`).

The package is a **stub**: the harnesses compile cleanly with plain
`cargo check` against
[`libfuzzer-sys`](https://docs.rs/libfuzzer-sys) but are not wired to
a real libFuzzer runtime. Operators opt into a real fuzzing campaign
by enabling the default `libfuzzer` feature on `libfuzzer-sys` and
running with `cargo +nightly fuzz`.

## Layout

```
fuzz/
├── Cargo.toml                                # standalone package, not in workspace
├── README.md                                 # this file
└── fuzz_targets/
    ├── fuzz_parse_inventory.rs               # harness: KaniInventory::parse_str
    ├── fuzz_parse_outcomes.rs                # harness: MutantsOutcomes::parse_str
    ├── fuzz_parse_records.rs                 # harness: MutantsRecords::parse_str
    ├── corpus_parse_inventory/
    │   └── seed1.json                        # minimal valid kani-list.json
    ├── corpus_parse_outcomes/
    │   └── seed1.json                        # minimal valid outcomes.json
    └── corpus_parse_records/
        └── seed1.json                        # minimal valid mutants.json
```

Each harness takes the canonical libFuzzer entry-point signature:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn LLVMFuzzerTestOneInput(data: *const u8, size: usize) -> i32;
```

The three targets share an identical oracle surface, only the parser
API under test differs.

## Oracles

Every `LLVMFuzzerTestOneInput` invocation runs the same three bounded
oracles against the parser under test.

### 1. Parse-or-typed-error

The arbitrary byte slice is converted to UTF-8 (a non-UTF8 slice is
**not** a violation — the parser contract is `&str → Result`, not
`&[u8] → Result`) and passed through the pure-core parser. The
parser must surface either a successful `Result::Ok` value or one
of the parser's typed error variants. Panics, silent truncation,
non-UTF8-tolerant input, or any other failure shape is a violation.

### 2. Deterministic reparse/roundtrip when successful

When the parse step returns `Ok`, the value is serialised back to
JSON via `serde_json::to_string` and re-parsed. The two values must
compare equal under the parser's `PartialEq` implementation. Any
drift — missing field, lost precision, different ordering — is a
violation. The roundtrip path uses a distinct path label
(`<fuzz-...-roundtrip>`) so error diagnostics remain unambiguous.

### 3. Max entry caps

A synthetic payload of exactly
`KANI_INVENTORY_MAX_HARNESSES + 1` /
`MUTANTS_OUTCOMES_MAX_ENTRIES + 1` /
`MUTANTS_RECORDS_MAX_ENTRIES + 1` minimal entries is built once at
process startup and parsed. The parser must return the matching
`TooManyHarnesses` / `TooManyOutcomes` / `TooManyRecords` variant.
The check is cached in a `OnceLock<bool>` so subsequent fuzz calls
pay only an atomic load — the synthetic construction and 1M+1-entry
parse run exactly once per process.

The cap construction is bounded:

- `KaniInventory`: 1M+1 empty-string harness names → ~3 MiB payload,
  ~150 MiB peak parse.
- `MutantsOutcomes`: 1M+1 baseline-success entries → ~50 MiB
  payload, ~200 MiB peak parse.
- `MutantsRecords`: 1M+1 empty-record entries → ~38 MiB payload,
  ~115 MiB peak parse.

All three stay well inside libFuzzer's default 2048 MiB RSS limit.

## Crash encoding

`LLVMFuzzerTestOneInput` returns `0` on success and `-1` on any
oracle violation. libFuzzer treats negative returns as crashes and
records the offending input for triage.

## No-panic / no-unsafe discipline

The harnesses contain no `unwrap`, `expect`, `panic!`, `todo!`,
`unimplemented!`, `unreachable!`, production `assert!`, or other
panic paths. The single `unsafe` block in each harness is the
libFuzzer ABI slice reconstruction (`slice::from_raw_parts(data,
size)`); it is the smallest unsafe surface required by the FFI
contract and carries a `// SAFETY:` comment justifying the libFuzzer
lifetime guarantee.

## Running the fuzzer (operator opt-in)

The fuzzer is **not** run automatically by the workspace CI. To
execute a real fuzzing campaign, install `cargo-fuzz` on the nightly
toolchain and run a single target with the bundled libFuzzer linked
in:

```bash
# One-time install (nightly).
cargo +nightly install cargo-fuzz

# From the workspace root, point cargo-fuzz at the standalone fuzz package.
cargo +nightly fuzz run \
    --manifest-path fuzz/Cargo.toml \
    --fuzz-dir fuzz \
    fuzz_parse_inventory \
    -- -max_total_time=60
```

Replace `fuzz_parse_inventory` with `fuzz_parse_outcomes` or
`fuzz_parse_records` to fuzz the mutant lanes. The bundled
`corpus_parse_*` directories act as the initial seed corpora;
libFuzzer will grow them under `fuzz/fuzz_targets/corpus_parse_*/`
as it discovers new coverage.

## When NOT to run

- The v1.5 contract is still bootstrapping (see
  `.evidence/v1.5/spec.md` §10): Miri, sanitizers, and cargo-fuzz
  are explicitly **deferred to v2.5**. Treat any fuzz run as
  exploratory, not gating.
- Do **not** enable the default `libfuzzer` feature on
  `libfuzzer-sys` from `fuzz/Cargo.toml` unless you have a real
  fuzzer driver wired up. Without `cargo-fuzz` providing `main()`,
  the link step will fail.

## Type-checking the stub

```bash
cargo check --manifest-path fuzz/Cargo.toml
```

This is the canonical local verification command for the stub — it
should exit 0 on the stable toolchain and compile every
`[[bin]]` target declared in `fuzz/Cargo.toml` without pulling in
clang's `-fsanitize=fuzzer`.