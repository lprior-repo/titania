# Regression diff — tn-4isu (State 12)

`git diff --stat`: 1 file changed (`crates/titania-core/src/kani.rs`).

Full diff (13 insertions, 3 deletions — 10 attribute lines + 1 comment line):

- 8× `+#[kani::unwind(32)]` on the previously unannotated fixed-input/u32
  harnesses (resolves the default-bound ≥5107-iteration grind; 2 lights went
  120s-TIMEOUT → 0.63s/2.27s SUCCESSFUL).
- `#[kani::unwind(32)]` → `#[kani::unwind(16)]` (diff harness; narrowed, still blocked).
- `#[kani::unwind(64)]` → `#[kani::unwind(12)]` via 24 (wire harness; narrowed, still blocked).
- 1 comment sync (`unwind(64)` → `unwind(12)` prose).

Behavior drift: NONE. Every changed line is a `#[kani::unwind]` attribute
(evaluated only by the Kani toolchain under `--cfg kani`) or a prose comment.
No assert/assume/cover/input/production/lane/config/test line touched — verified
by `git diff` inspection (13+/3-, all inside `#[...]` attribute lines + 1 comment).
`cargo test -p titania-core` green (96/96) confirms zero behavior drift;
`outcome_serde.rs` (22 tests) is the paired negative control for the wire domain.
