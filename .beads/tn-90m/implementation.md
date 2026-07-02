# tn-90m Target-Project Support Slice — Implementation

## Summary

Regenerated lost work from prior session. Typed `target_root` on every lane input, CWD-implicit discovery (no `--project` flag), pure core / I/O at the edge.

## Changes

### 1. `crates/titania-lanes/src/lib.rs` — Core function + delegation

**Added `target_project_from_path(cwd: &Path) -> Result<TargetProject, TargetProjectError>`**

Pure core function. Accepts any `&Path`, delegates to `discover_target()`. No CWD coupling — allows callers to pass pre-validated paths from other layers.

**Refactored `current_target_project()` to delegate**

```rust
pub fn current_target_project() -> Result<TargetProject, CurrentTargetError> {
    let cwd = env::current_dir().map_err(CurrentTargetError::CurrentDir)?;
    target_project_from_path(&cwd).map_err(CurrentTargetError::Target)
}
```

CWD-read remains the imperative shell boundary; the pure core is `target_project_from_path`.

### 2. `crates/titania-lanes/src/bin/run_cargo.rs` — Refactored to use `current_target_project()`

**Before (inline CWD + discover):**
```rust
let cwd = env::current_dir().map_err(RunCargoError::CurrentDir)?;
let target = discover_target(&cwd).map_err(RunCargoError::Target)?;
```

**After (single delegate call):**
```rust
let target = current_target_project()?;
```

Added `impl From<CurrentTargetError> for RunCargoError` to enable `?` operator.

Updated imports: removed `discover_target` and `TargetProjectError` from direct `titania_core` import (kept `TargetProjectError` for `RunCargoError::Target` variant), added `CurrentTargetError` and `current_target_project` from `titania_lanes`.

## Verification

All three exit 0:
- `cargo check --workspace --all-targets` ✅
- `cargo test --workspace` — 205 passed ✅
- `cargo clippy --workspace --all-targets` ✅

## Holzman Rust Constraints

- No unwrap/expect/panic — all errors use `Result` with typed variants.
- No mutation in calculations — `target_project_from_path` is pure.
- Parse-don't-validate — `discover_target` returns `Result`, never panics.
- Illegal states unrepresentable — `TargetProject` construction is total via `try_from_path`.

## Absent Sub-beads

Verified last session: sub-beads tn-h7c, tn-0p3, tn-2k8, tn-3r1, tn-9v6, tn-8e2, tn-5w4 do not exist as `bd` issues. No separate work was tracked for them. The work from this bead (tn-90m) was a self-contained support slice — adding the typed core function and refactoring the one binary that had inline CWD+discover duplication.
