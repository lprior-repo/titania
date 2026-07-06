# Research Notes — tn-4rq.2 Doctor Implementation

## Spec Requirements (v1-spec §12)
1. `titania-check doctor [--scope <scope>] [--emit json]` reports tools and versions
2. Human output: columns Tool, Required, Installed, Version, Path + Status line
3. JSON output: scope, tools[], missing_required[], status
4. Missing required tools => exit 3 (InputError) + status "MissingRequiredTools"
5. Optional sccache missing must NOT cause MissingRequiredTools

## Tool Matrix by Scope
| Tool | Edit | Prepush | Release |
|------|------|---------|---------|
| cargo | required | required | required |
| rustfmt | required | required | required |
| clippy/clippy-driver | required | required | required |
| rg | required | required | required |
| ast-grep (embedded) | required | required | required |
| cargo-dylint | required | required | required |
| cargo-deny | optional | required | required |
| sccache | optional | optional | optional |

## Dylint Rows
- `cargo-dylint` binary: PATH probe, version, path
- `libtitania_dylint`: co-located library check; ABI compatibility `unknown` when library missing

## Embedded ast-grep
- No filesystem path, no version — identified by `embedded` row with null version/path
- Always required (part of the binary via tn-37r.4 bead)

## Design Decisions
- Pure Rust PATH lookup (no external `which` crate dependency)
- `ToolRow::name` is `&'static str` to avoid ownership overhead
- `DoctorOptions.emit()` accessor replaces private field access
- `EmitFormat` made `pub` with docs for cross-module use
- `doctor::render` is `pub(crate)` to respect `CliDisposition` visibility
- Forward-compatible wildcard arms on `GateScope` matches
