# Template Prepush — Generated Workspace Smoke Test Report

**Bead:** tn-rld.4  
**Date:** 2026-07-05  
**Disposition:** `PASSED`

## Blocker resolution

`cargo-generate` was missing. Installed via:

```bash
cargo install cargo-generate
```

Exit status: **0**  
Installed version: **cargo-generate v0.23.12**

Verified:

```bash
cargo generate --version
```

Result: `cargo generate-generate 0.23.12` — exit **0**.

## Acceptance command

```bash
cargo test -p titania-check template_prepush
```

Exit status: **0** (test passed)

### What the test does

1. Resolves the template directory at `<worktree>/titania/template/` from `CARGO_MANIFEST_DIR`.
2. Generates a fresh workspace using `cargo generate --path <template> --name <unique-name>` into `/tmp/` with a nanosecond-unique name.
3. Runs `target/debug/titania-check --scope prepush --emit json` in the generated workspace.
4. Asserts:
   - The generated workspace contains `Cargo.toml` and `deny.toml`.
   - The JSON report has `"variant": "reject"` (no lane artifacts exist in a fresh template).
   - The JSON report has `"gate_failures"` with ≥1 entry (all lanes report "output file missing" since no lane artifacts were run).
5. Cleans up the generated workspace.

## Generated workspace details

- **Temp dir strategy:** `/tmp/<test-name>-<nanosecond-timestamp>/`
- **Template:** `titania/template/` (empty workspace with strict Rust config)
- **titania-check output:** `reject` with 9 gate failures (Fmt, Compile, Clippy, AstGrep, Dylint, PanicScan, PolicyScan, Test, Deny — all "output file missing")
- **Exit code:** 1 (reject — expected for a workspace with no prepush artifacts)

## Files changed

- `crates/titania-check/tests/template_prepush.rs` — rewritten: replaced blocker-only test with full generated-workspace smoke test.
- `.evidence/template-prepush/disposition.txt` — updated from `BLOCKED_MISSING_CARGO_GENERATE` to `PASSED`.
- `.evidence/template-prepush/probe-report.md` — updated with full command evidence.

## Residual blockers

None. The acceptance command passes. The test proves that a freshly generated Titania workspace can run `titania-check --scope prepush --emit json` and produce a valid JSON report.
