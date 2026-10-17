# Titania goal 1 — apply Holzman Rust to Titania's own code

## Scope and status

The goal is bounded, reviewable, idiomatic Rust in Titania itself, plus enforcement of that coding standard. It is not expansion of Verus/Kani/Flux, not a new language, and not a Velvet Ballistics change.

This bundle is a DRAFT against `lprior-repo/titania` commit
`9a5c936e93551a778a17ccb32b671f0f8058deef`.
It has not been compiled, tested with Rust, linted, accepted by Moon, committed, or pushed. Do not report the implementation as complete until the canonical gates pass. The numerical limits below are proposed policy defaults for review, not measured workload requirements.

Canonical supplied reference: `nasa-jpl-standards(3).md`, “NASA/JPL Power of Ten PLUS For Rust.” The rule on fixed upper bounds takes precedence over treating iterator syntax as evidence of bounded work. The repository's pinned toolchain and Moon remain authoritative; do not replace the pin with floating `+nightly`.

## Concrete application in this patch

`crates/titania-lanes/src/run_lane_sources.rs` currently performs indirect recursion through `collect_rust_sources_into` and `visit_source_entry`. Its `ReadDir::try_for_each` traversal has no configured work or depth maximum. Rewriting the same traversal as a functional iterator did not establish boundedness.

The patch replaces that traversal with a depth-first stack owned by the scanner and one fixed-range driver. Each driver step polls at most one directory entry. Non-Rust entries, excluded directories, and end-of-directory polls consume work. The result is returned only after the root traversal has actually completed; exhaustion returns `SourceWalkError::LimitExceeded` instead of a successful prefix.

Proposed fixed production maxima:

| Resource | Maximum | Meaning |
|---|---:|---|
| Traversal steps | 250,000 | At most this many application-level `ReadDir::next` calls, including rejected entries and EOF |
| Directory depth | 64 | At most 64 simultaneously owned directory frames; root is level 1 |
| Rust source files | 50,000 | Maximum returned path count |
| Per-path bytes | 4,096 | OS-encoded bytes, not Unicode character count |
| Admitted path bytes | 67,108,864 | Cumulative bytes charged for directory paths and returned relative paths; no refund on directory exit |

Checked subtraction accounts for path storage. Collection growth uses `try_reserve` and preserves allocation errors. Existing sorting, build-script exclusions, directory exclusions, and behavior for stable symlink entries are preserved. No dependencies are added; tests use the existing `tempfile` dependency.

The private driver receives a limits value to permit small boundary tests. Its production entrypoint always supplies the fixed constants. This structure makes the Rust-level polling bound straightforward to inspect, but this patch does not implement a general static bound analyzer.

## Explicit limits of the guarantee

A directory read or metadata syscall can still block. A bounded number of syscalls is not a wall-clock completion proof. OS and library internals are outside the application-level polling bound.

The storage counters bound admitted path bytes, file count, and traversal depth. They do not certify total process RSS, allocator overhead, every temporary allocation, or allocation-failure freedom. This source scanner is a cold I/O boundary, not an allocation-free safety-critical hot path.

The scanner is not a filesystem sandbox or a consistent snapshot. A concurrent directory-to-symlink replacement remains a separate race; the stable-symlink test does not establish resistance to hostile concurrent mutation. The patch preserves existing source exclusions rather than claiming they discover every generated/compiled production file.

Adding error variants can expose exhaustive matches elsewhere in the full workspace. Full-workspace compilation is required to discover and repair those integrations; no partial source snapshot can establish that they all compile.

## Enforcement contract for the remainder of goal 1

For each production-reachable loop, iterator consumer, recursive call, retry, traversal, and drain, identify the maximum work and exhaustion behavior. Bounds must remain valid under nested work, failing predicates, retries, and cancellation. A timeout or cancellation token alone is operational containment, not a static iteration bound.

An accepted constant range or validated length supplies a bound only for the operations actually counted. In particular:

- `filter(predicate).take(N)` bounds matching outputs, not predicate evaluations.
- `take(N).filter(predicate)` bounds outer input pulls, but each input iterator's `next` and the predicate can still do unbounded work or block.
- A finite `Vec` with an unrestricted length is not a chosen resource ceiling.
- A bounded queue can be refilled forever; draining it needs its own per-operation bound.
- Replacing a loop with `fold` or `try_for_each` is not evidence of progress.
- Silently taking the first N files and returning success would hide unscanned code. Exceeding the admitted scope must return an explicit incomplete/limit failure.

Conservative bound recognition should fail closed for unsupported cases. A human annotation is a claim, not a proof. Avoid implementing an unsound AST heuristic that marks any `.take` or uppercase constant name as verified. Resolved identities, admitted input bounds, macro expansion, and bounds on called work matter.

The existing `FUNC_LOOPS_*` Dylint family still bans syntax. This patch does NOT remove those rules, replace them with a bound analyzer, or declare Titania wholly compliant. The actual driver uses a known finite range to remain compatible with the current style policy while removing the real recursion/unbounded traversal. Replacing the blanket syntax rule with tested, type-aware boundedness checks remains an explicit acceptance requirement for complete goal 1.

Other Holzman rules remain in scope for Titania: explicit errors and checked numeric semantics, small functions and narrow mutable state, validated domain types, safe APIs by default, bounded subprocess/output handling, generated-code review, and the repository's zero-warning gates. Apply the no-post-initialization-allocation requirement only to paths explicitly classified as safety/mission-critical; do not falsely label this cold scanner allocation-free.

## Acceptance evidence required

The added 14 Rust test cases call the production implementation with boundary-sized limits. Cases cover exact budgets, one-over rejection, nonmatching entries consuming budget, EOF/completeness, source count, directory depth, path lengths, cumulative path bytes, preserved I/O errors, exclusions, and stable symlinks (Unix-only). They were WRITTEN but NOT RUN.

Run the repository's canonical `moon ci` with the pinned toolchain, including source formatting/linting, source-scanner tests, and existing lane integration tests. Confirm that `SourceWalkError` propagates to a failed lane, never `Clean`. Use a disposable small workspace for resource-failure integration tests, not a real production checkout with stateful writes.

The working environment had no `rustc`, `cargo`, `moon`, or `bd`, and could not resolve github.com for a checkout. Source was retrieved through the connected GitHub reader. The original file's exact Git blob hash was verified before generating the patch. Patch application was checked against that exact file. These packaging checks do not establish Rust correctness.

## Source locations inspected

- `crates/titania-lanes/src/run_lane_sources.rs`, original blob `db99df5c469d3278623176947607afdeb60b03b7`
- `crates/titania-dylint/src/functional_surface.rs`, original blob `193edc05180ed123ebeb92bf8662b08402254f80`
- `crates/titania-lanes/Cargo.toml`, original blob `32375ced2e5fa275485efde71ed63ceb9cb4bfbd`
- `AGENTS.md`

No existing proof gates or prior patch bundle are modified by this bundle.
