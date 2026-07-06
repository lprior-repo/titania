# cargo-vet Inspection Report: aho-corasick 1.1.4

## Crate Metadata

| Field | Value |
|-------|-------|
| Crate | `aho-corasick` |
| Version | `1.1.4` |
| Publisher | Andrew Gallant (`BurntSushi`), jamslam@gmail.com |
| License | Unlicense OR MIT |
| Edition | 2021 |
| rust-version | 1.60.0 |
| Homepage | https://github.com/BurntSushi/aho-corasick |
| Description | Fast multiple substring searching |

## Commands & Evidence

```
$ cargo vet inspect aho-corasick 1.1.4
# Exit code: 0 (fetched source to ~/.cache/cargo-vet/src/aho-corasick-1.1.4)

$ cd ~/.cache/cargo-vet/src/aho-corasick-1.1.4 && cargo test --lib
# Exit code: 0 — 163 tests passed (1 suite, 0.46s)

$ cd ~/.cache/cargo-vet/src/aho-corasick-1.1.4 && cargo check --all-features
# Exit code: 0 — compiles clean (4 warnings: unused const, lifetime elision)

$ cd ~/.cache/cargo-vet/src/aho-corasick-1.1.4 && cargo check --no-default-features
# Exit code: 0 — no_std build compiles clean (3 warnings)

$ cd /home/lewis/src/titania/.worktrees/v1-combined-dispatch && cargo tree -p aho-corasick
# Exit code: 0
# aho-corasick v1.1.4 └── memchr v2.8.2

$ cd /home/lewis/src/titania/.worktrees/v1-combined-dispatch && cargo tree -i aho-corasick
# Exit code: 0
# aho-corasick v1.1.4
# ├── regex v1.12.4
# │   └── dylint_internal v6.0.1
# │       └── dylint_linting v6.0.1
# │           └── titania-dylint v0.0.0 (workspace member)
# └── regex-automata v0.4.14
#     └── regex v1.12.4 (*)
```

## Files & Areas Inspected

### Package Manifest
- `Cargo.toml` — no build script, optional deps only (`memchr`, `log`)
- `Cargo.toml.orig` — original source form
- `DESIGN.md` — thorough internal design documentation (482 lines)
- `README.md` — public API documentation

### Core Source Modules (all reviewed)
- `src/lib.rs` — public API surface, feature-gated exports
- `src/ahocorasick.rs` — main `AhoCorasick` type, builder, search iterators
- `src/automaton.rs` — `Automaton` trait (sealed, unsafe), match iterators
- `src/dfa.rs` — deterministic finite automaton implementation
- `src/nfa/contiguous.rs` — contiguous NFA (single allocation)
- `src/nfa/noncontiguous.rs` — sparse NFA (per-state allocations)
- `src/transducer.rs` — FST integration (test-only)

### Packed/SIMD Modules
- `src/packed/mod.rs` — packed searcher entry point
- `src/packed/api.rs` — public packed API
- `src/pattern.rs` — pattern types, `is_equal_raw` unsafe comparison
- `src/packed/rabinkarp.rs` — Rabin-Karp fallback for short haystacks
- `src/packed/vector.rs` — SIMD trait abstraction (`Vector`, `FatVector`), x86_64 SSSE3/AVX2, aarch64 NEON impls
- `src/packed/teddy/builder.rs` — Teddy multi-pattern matcher with runtime CPU feature detection
- `src/packed/teddy/generic.rs` — generic SIMD algorithm (Slim/Fat variants)
- `src/packed/teddy/mod.rs` — Teddy module root
- `src/packed/tests.rs` — packed searcher tests

### Utility Modules
- `src/util/prefilter.rs` — prefilter system (byte frequency, Rabin-Karp, SIMD accelerated single-pattern via memchr)
- `src/util/search.rs` — search primitives, match semantics, anchoring
- `src/util/alphabet.rs` — alphabet remapping for DFA
- `src/util/error.rs` — error types
- `src/util/int.rs` — fixed-width integer types
- `src/util/primitives.rs` — `PatternID`, `StateID` newtypes
- `src/util/buffer.rs` — stream buffering
- `src/util/byte_frequencies.rs` — byte frequency analysis
- `src/util/debug.rs` — debug formatting
- `src/util/remapper.rs` — alphabet remapper
- `src/util/special.rs` — state categorization
- `src/util/prefilter.rs` — prefilter implementations

## Behavior Review

### Build Scripts
**None.** No `build.rs` found. Verified via `build = false` in `Cargo.toml`.

### Proc-Macros
**None.** No proc-macro crate dependency or definition.

### Unsafe Code Inventory

| Location | Scope | Assessment |
|----------|-------|------------|
| `src/automaton.rs:198` | `pub unsafe trait Automaton` | Sealed trait (only crate-internal types implement). Safety invariant: `start_state` returns valid state IDs, `next_state` on valid IDs returns valid IDs. No UB in practice. |
| `src/ahocorasick.rs:2661` | `unsafe impl Automaton for Arc<dyn AcAutomaton>` | Delegates to inner `AcAutomaton` trait object. Safe because inner type is one of the sealed impls. |
| `src/automaton.rs:641` | `unsafe impl<'a, A: Automaton> Automaton for &'a A` | Simple delegation, inherits safety from `A`. |
| `src/dfa.rs:190` | `unsafe impl Automaton for DFA` | DFA transition table is fully initialized by builder. Safe. |
| `src/nfa/contiguous.rs:176` | `unsafe impl Automaton for NFA` | Contiguous NFA transition table is fully initialized. Safe. |
| `src/nfa/noncontiguous.rs:591` | `unsafe impl Automaton for NFA` | Sparse NFA transition tables are fully initialized. Safe. |
| `src/packed/ext.rs:8,21,32` | `Pointer::distance` | Wraps `ptr::offset_from` + `unwrap_unchecked`. Sound because `self >= origin` is guaranteed by caller (pointers derived from same slice). |
| `src/packed/pattern.rs:153,265` | `Pattern::get_unchecked`, `Pattern::is_prefix_raw` | Unsafe access from owned/safe `Patterns` struct. Index validated by struct invariants. |
| `src/packed/pattern.rs:368` | `is_equal_raw` | Unsafe raw pointer byte comparison for SIMD-accelerated prefix check. Caller guarantees both pointers derived from valid borrowed slices with length `n`. All loads use `read_unaligned()` — no alignment requirement. Safe. |
| `src/packed/vector.rs` | `Vector`/`FatVector` trait unsafe methods | All methods are intrinsics wrappers (`_mm_*`, `_mm256_*`, `v*_`). Safety contract: caller must ensure target features enabled. Implementation is `#[inline(always)]`, not `#[target_feature]`. Safe because callers guard with runtime CPU detection. |
| `src/packed/teddy/builder.rs` | Teddy `new_unchecked`, `find` | Runtime CPU feature detection via `std::is_x86_feature_detected!`. Fallback to `false` in no_std mode (conservative). Only constructs SIMD searcher if feature present. Safe. |
| `src/packed/teddy/generic.rs` | Generic SIMD `new`, `find`, `find_one`, `candidate` | Uses `Vector` trait methods. Safety inherited from `Vector` impl guards. Safe. |

### File System / Network / Process Access
**None.** This crate performs only memory and CPU operations on byte slices. No `std::fs`, `std::net`, `std::process`, `std::os`, or equivalent `alloc` APIs are used.

### Denial-of-Service / Resource-Bound Concerns

| Concern | Assessment |
|---------|------------|
| **Construction time** | O(p) where p = combined length of all patterns. Documented in DESIGN.md. No unbounded loops. |
| **Construction memory** | Scales linearly with pattern count. Noncontiguous NFA: ~1 byte per pattern byte + state overhead. Contiguous NFA: ~21 MB for 100k Wikipedia titles. DFA: ~1.6 GB (configurable, not default). Construction can fail gracefully via `BuildError`. |
| **Search time** | O(n) where n = haystack length. Each byte of input is visited at most once by the automaton. Prefilters (rare bytes, starting bytes) are also O(n). No super-linear worst case. |
| **Search memory** | Only the pre-constructed automaton is held. Search uses O(1) additional stack space per call. |
| **Pattern count** | Documented construction failure threshold: ~millions of patterns. Safe to use with untrusted pattern lists — builder returns `Result`. |
| **Haystack size** | No upper bound enforcement (not needed; O(n) is linear). Search iterators borrow haystack, preventing reallocation during iteration. |
| **ReDoS** | Not applicable. This is a deterministic finite automaton (Aho-Corasick), not a regex engine. No backtracking possible. |

### Dependency Relationship to Titania

```
titania-dylint (workspace member)
  └── dylint_linting v6.0.1
      └── dylint_internal v6.0.1
          ├── regex v1.12.4  ──┐
          └── regex-automata v0.4.14 ──┤
              └── (both depend on)      │
                  aho-corasick v1.1.4 ←─┘
```

`aho-corasick 1.1.4` is a **transitive dependency** of Titania, reachable through `regex` and `regex-automata` used by `dylint_internal` → `dylint_linting` → `titania-dylint`.

### Publisher Provenance

Andrew Gallant (`BurntSushi`) is one of the most trusted authors in the Rust ecosystem:
- Author of `aho-corasick`, `regex`, `regex-automata`, `memchr`, `bstr`, `globset`, `ignore`, `walkdir`, `lazy_static`
- Mozilla and ByteCode Alliance explicitly trust his publishing identity (as noted by `cargo vet suggest`)
- All dependencies are from the same author, forming a well-known, well-maintained ecosystem

## Safe-to-Deploy Decision: **APPROVE**

### Rationale

1. **No build script, no proc-macros** — no code generation, no compile-time risk.
2. **Minimal, well-documented unsafe code** — every unsafe block has a clear preconditions/contract. SIMD intrinsics are guarded by runtime CPU feature detection with conservative no_std fallbacks.
3. **No ambient capabilities** — zero filesystem, network, or process access. Pure byte-slice computation.
4. **Deterministic, bounded resource usage** — O(n) search, O(p) construction. No backtracking, no exponential worst case.
5. **Graceful failure mode** — construction returns `Result`; no panics on untrusted input (except `unwrap_unchecked` on verified invariants).
6. **All 163 tests pass** across both `--all-features` and `--no-default-features`.
7. **Reputable publisher** — BurntSushi is Mozilla/ByteCode Alliance trusted; the crate is foundational infrastructure in the Rust ecosystem (used by `regex`, `serde_json`, `bstr`, and many others).
8. **Thorough internal documentation** — 482-line DESIGN.md demonstrates mature engineering discipline.

### Caveats

- The `memchr` dependency (optional, enabled by default via `perf-literal` feature) is also by BurntSushi and is itself a foundational, well-audited crate. No separate inspection needed, but it inherits the same trust chain.
- The `std` feature is required for stream-searching APIs (`StreamFindIter`) and the `memchr` memmem prefilter. In `no_std` mode, the Teddy SIMD matcher falls back to `false` for runtime feature detection (SSSE3/AVX2 unavailable at compile time), and the memchr prefilter is unavailable. This is a safe, conservative default — no correctness risk.
- The crate uses `unwrap_unchecked` in two locations (`ext.rs` and `pattern.rs`). Both are on invariants that are provably true: (a) pointer distance where `self >= origin` is caller-guaranteed, and (b) `n >= 4` branch guard after exhaustive match on `n < 4`. No safety risk.
- The `Automaton` trait is `unsafe` and sealed. This is a design choice to allow future trait method additions without breaking compatibility, not an indication of safety concerns in the existing implementations.
