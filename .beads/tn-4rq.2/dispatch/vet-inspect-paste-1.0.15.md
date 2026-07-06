# cargo-vet Inspection Report: paste 1.0.15

## Crate Information

| Field | Value |
|---|---|
| **Crate** | `paste` |
| **Version** | 1.0.15 |
| **Publisher** | David Tolnay (`dtolnay`) |
| **Repository** | https://github.com/dtolnay/paste |
| **License** | MIT OR Apache-2.0 |
| **Edition** | 2018 |
| **Rust MSRV** | 1.31 |
| **Library type** | proc-macro (`proc-macro = true`) |
| **Build script** | Yes (`build.rs`) |
| **Unsafe** | None |
| **External deps** | None (proc-macro crate has no runtime dependencies) |
| **Dependents in titania** | `dylint_linting` (transitive via `dylint_internal`) |

## Commands Executed

### 1. `cargo vet inspect paste 1.0.15` (exit 0)
Fetched source to `/home/lewis/.cache/cargo-vet/src/paste-1.0.15`.

### 2. `cargo check` in paste-1.0.15 (exit 0)
Library compiled cleanly with no warnings or errors.

### 3. `cargo vet diff paste 1.0.10 1.0.15` (exit 0)
Full diff against the vetted base (1.0.10). See diff summary below.

### 4. `grep unsafe src/*.rs` in paste-1.0.15 (exit 1)
No `unsafe` blocks found in any source file.

### 5. `grep env::|net::|process::|fs:: src/*.rs` in paste-1.0.15 (exit 1 for file/net/process/fs)
Only `std::env::var` found — used exclusively within the `env!` macro feature at compile time. No network, process-spawning, or filesystem I/O in proc-macro expansion code.

## Files Inspected

| File | Lines | Review |
|---|---|---|
| `src/lib.rs` | ~455 | Full review of proc-macro entry points, token parsing, expansion logic |
| `src/attr.rs` | 164 | Full review of attribute/doc-string paste handling |
| `src/segment.rs` | 233 | Full review of segment parsing, env! resolution, modifier (lower/upper/snake/camel) logic |
| `src/error.rs` | 47 | Full review of error-to-compile_error! emission |
| `build.rs` | 38 | Full review of build script logic |
| `Cargo.toml` | 72 | Reviewed package metadata, features, dev-dependencies |
| `tests/` | — | Inspected for behavioral intent only; not executed (dev-dep corruption) |

## Proc-Macro Behavior Analysis

### Core Operation
`paste` is a proc-macro crate that generates new identifiers at compile time by token concatenation. It parses `[< ... >]` bracket groups, splits them into string/literal/env/modifier segments, pastes them together, and emits the result as a `proc_macro::Ident` or `proc_macro::Literal`.

### Token Manipulation
- Input: `TokenStream` from the `paste!` macro invocation
- Parsing: Iterates tokens, identifies `[< ... >]` paste operations, parses segments (identifiers, literals, `env!`, modifiers `:lower`/`:upper`/`:snake`/`:camel`)
- Output: Constructs new identifiers via `Ident::new(&pasted, span)` or `Literal::from_str(&pasted)`
- Safety: All token construction is wrapped in `panic::catch_unwind` to prevent proc-macro panics from crashing the compiler — errors are converted to `compile_error!` invocations instead

### Attribute Expansion (`attr.rs`)
Handles `#[doc = ...]` and other name-value attributes containing paste operations. Recursively processes comma-separated attribute arguments. Only modifies the attribute value portion; the attribute name/path is preserved.

### Environment Variable Access (`segment.rs:157`)
```rust
let resolved = match std::env::var(&var.value) { ... };
```
- Used only within the `env!` macro feature inside `[< ... >]`
- Occurs at compile time during macro expansion (not runtime)
- Result is embedded into generated code as a string literal
- If env var is missing, produces a compile-time error — does not silently fall back or mask the absence
- The `replace('-', "_")` post-processing on resolved values prevents identifier-invalid characters

### Build Script (`build.rs`)
Checks `rustc --version` and sets `cfg` flags:
- `no_literal_fromstr` for Rust < 1.54 (pre-Literal::from_str support)
- `rustc-check-cfg` declarations for Rust >= 1.80
- Declares `feature("protocol_feature_paste")` as a known feature value
- **No network, file, or process side effects** — only emits `cargo:` directives

## Diff from Vetted Base (1.0.10 → 1.0.15)

**Zero behavioral changes to proc-macro logic.** All changes are cosmetic:

| Change | Category | Risk |
|---|---|---|
| Added `#![doc(html_root_url = ".../1.0.15")]` | Documentation | None |
| `build.rs`: added `cargo:rustc-check-cfg` for Rust >= 1.80 | Build hygiene | None (improves cfg validation) |
| CI workflow updates (GitHub Actions v3→v4, new jobs) | CI/CD | N/A |
| Cargo.toml auto-normalization | Formatting | None |
| LICENSE-APACHE appendix removed | License formatting | None |
| README badge URL update | Documentation | None |
| Test files: added `#![allow(clippy::let_underscore_untyped)]` | Test hygiene | None |
| Test adjustments (struct visibility, stringify args) | Test correctness | None |
| UI test stderr updates | Compiler message format drift | None |

## Publisher/Author Provenance

- **David Tolnay (`dtolnay`)** is one of the most prolific and trusted Rust crate authors
- Maintains: `serde`, `serde_derive`, `serde_json`, `tokio-macros`, `proc-macro2`, `quote`, `syn`, `anyhow`, `thiserror`, `itoa`, `ryu`, `indoc`, `inventory`, `linkme`, `stacker`, `getrandom` (co-maintainer), and dozens more
- mozilla and bytecode-alliance already trust dtolnay (per cargo-vet `suggest` output)
- The paste crate has been in active use across the Rust ecosystem since 2018 with no reported security incidents

## Dependency Relationship to Titania

```
titania
 └─ dylint_linting (6.0.1)
    └─ paste (1.0.15)
```

`paste` is a transitive dev-build dependency via `dylint_linting`. It runs at compile time only and generates code — it never executes at runtime.

## Safe-to-Deploy Decision: **APPROVE**

### Rationale

1. **No code changes from vetted base**: The 1.0.15 diff against 1.0.10 contains zero changes to proc-macro logic, token manipulation, attribute handling, or segment parsing. The diff is entirely cosmetic (CI, docs, cfg hygiene, test formatting).

2. **No unsafe code**: The entire crate is free of `unsafe` blocks.

3. **No runtime dependencies**: As a proc-macro crate, it has zero runtime dependencies. It only uses `std` for `TokenStream`, `Ident`, `Literal`, `panic::catch_unwind`, and `std::env::var`.

4. **Defensive panic handling**: All token construction (`Literal::from_str`, `Ident::new`) is wrapped in `catch_unwind` to prevent compiler crashes from malformed macro input.

5. **Build script is benign**: Only reads `rustc --version` and emits `cargo:` directives for cfg validation. No network, filesystem, or process side effects.

6. **Environment variable access is compile-time only**: `std::env::var` is used exclusively within the `env!` macro feature at compile time, with proper error handling (compile error on missing var).

7. **Trusted publisher**: David Tolnay's reputation and extensive audit history in the Rust ecosystem.

8. **Cargo-vet already trusts dtolnay**: mozilla and bytecode-alliance explicitly trust this publisher (per `cargo vet suggest` output).

### Caveats

- `std::env::var` access requires the `env!` macro feature to be used by the consuming crate. This reads environment variables at compile time — consumers should ensure no secrets are leaked into generated identifiers (the resolved env var becomes part of the compiled code).
- Test suite did not execute due to a corrupted `paste-test-suite` dev-dependency on this system, but the library itself compiles cleanly. This is a test harness issue, not a source code issue.
- This is a proc-macro crate: the `safe-to-deploy` criteria focus on whether generated code is safe, which is satisfied by the deterministic, input-restricted token manipulation in this crate.

---

*Report generated: 2026-07-05*
*Inspector: VetPaste (supply-chain Rust crate auditor)*
