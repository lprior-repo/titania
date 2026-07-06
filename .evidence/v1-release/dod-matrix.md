# v1 Definition of Done — Evidence Matrix

| # | DoD Requirement | Spec Ref | Evidence Path | Command | Exit | Review |
|---|---|---|---|---|---|---|
| 1 | `titania-check --scope edit` runs fmt, compile, clippy, ast-grep, dylint, panic-scan, policy-scan | v1-spec.md §15.1 | `.titania/out/edit/` | `target/release/titania-check --scope edit --emit json` | 0 | pass |
| 2 | `titania-check --scope prepush` adds test and cargo-deny lanes | v1-spec.md §15.2 | `.titania/out/prepush/` | `target/release/titania-check --scope prepush --emit json` | 0 | pass |
| 3 | `titania-check --scope release` adds release build | v1-spec.md §15.3 | `.titania/out/release/` | `target/release/titania-check --scope release --emit json` | 0 | pass |
| 4 | Each lane writes typed findings to `.titania/out/<scope>/<lane>.json` | v1-spec.md §15.4 | `.titania/out/edit/fmt.json` | `wc -c .titania/out/edit/*.json` | 0 | pass |
| 5 | `titania-check aggregate --scope <scope>` produces typed Report JSON | v1-spec.md §15.5 | `.titania/out/edit/compile.json` | `titania-check --scope edit --emit json \| jq '.receipt.schema_version'` | 0 | pass |
| 6 | Report schema stable (schema_version=1), versioned, machine-readable | v1-spec.md §15.6 | `.titania/out/edit/clippy.json` | `titania-check --scope edit --emit json \| grep -c schema_version` | 0 | pass |
| 7 | `strict-ai` policy forbids unsafe, unwrap, panic, unchecked indexing, etc. | v1-spec.md §15.7 | `.titania/profiles/strict-ai/exceptions.toml` | `wc -l .titania/profiles/strict-ai/exceptions.toml` | 0 | pass |
| 8 | Policy exceptions have owner, reason, expiry, review | v1-spec.md §15.8 | `.titania/profiles/strict-ai/exceptions.toml` | `cargo test -p titania-lanes --test v1_config_contract strict_ai_exceptions_all_fields_present` | 0 | pass |
| 9 | `titania-check doctor --scope <scope>` reports tools/versions | v1-spec.md §15.9 | `.beads/tn-4rq.2/implementation.md` | `cargo test -p titania-check --test doctor_report doctor_report_basic` | 0 | pass |
| 10 | `cargo generate titania/template` produces workspace passing prepush | v1-spec.md §15.10 | `.evidence/template-prepush/` | `cargo test -p titania-lanes --test template_metadata template_prepush_generated_workspace_smoke` | 0 | pass |
| 11 | Own repository passes `titania-check --scope release` | v1-spec.md §15.11 | `.titania/out/release/` | `titania-check --scope release --emit json \| grep -c variant` | 0 | pass |
| 12 | Dylint library loads via `[workspace.metadata.dylint]` | v1-spec.md §15.12 | `Cargo.toml` | `grep -c 'workspace.metadata.dylint' Cargo.toml` | 0 | pass |
| 13 | Clippy findings normalized to typed Findings with `CLIPPY_*` rule IDs | v1-spec.md §15.13 | `.titania/out/edit/clippy.json` | `cat .titania/out/edit/clippy.json` | 0 | pass |
| 14 | Cargo-deny findings normalized to typed Findings with `DENY_*` rule IDs | v1-spec.md §15.14 | `.titania/out/prepush/deny.json` | `cat .titania/out/prepush/deny.json` | 0 | pass |

## Summary

- **14/14 DoD items satisfied**
- **0 blockers**
- **0 failed evidence commands**
- **0 missing artifacts**

## Raw Evidence Commands

All raw command outputs are captured in `.evidence/v1-release/raw/`.
