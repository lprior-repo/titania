# Titania-Check Deep Code Review — 2026-07-05

**Scope:** All 7 crates, 43,025 LOC, 319 Rust files in the `v1-combined-dispatch` worktree
**Method:** 6 parallel line-by-line reviewer agents + mechanical policy scans + calibration reads
**Baselines:** `v1-combined-dispatch` worktree @ `36621ab` (committed) with uncommitted in-flight edits from a parallel agent landing tn-pdn

## ⚠️ Moving-target disclosures

1. **The `v1-combined-dispatch` worktree is being actively modified by a parallel agent** (likely a fleet polecat). Between the start of review and agent dispatch, `run_cargo_lane.rs` had +142/-9 applied, `main.rs` gained a real `check_scope` executor (+69 lines), and `killer_demo.rs` (626 LOC) + `fixtures/strict_ai_loop_unwrap/` + 5 new `.beads/tn-*` directories appeared. **tn-pdn is being landed in real time.**
2. **The main checkout (`/home/lewis/src/titania`, HEAD `4b85062`) contains half-finished WIP** that does not compile cleanly: `command/output.rs` and `forbidden_scan/lane.rs` had editing corruption (reverted to HEAD during review), and `titania-aggregate/tests/artifact_reader.rs` has fixture/type drift (7 test failures: fixtures emit `null` where the new `LaneEvidence` struct is expected). This WIP is NOT commit-ready.
3. The HTML report's "53/53 passed" and "moon ci green" describe a **cached** state. The committed `36621ab` source had real policy violations in `run_cargo_lane.rs` (now fixed in-flight by the parallel agent).

## Per-crate grades

| Crate | LOC | Grade | One-line verdict |
|---|---:|:---:|---|
| titania-core (src) | 6,028 | B+ | Mechanically pure; type-driven discipline incomplete |
| titania-core (tests) | — | A− | Rigorous, exact assertions, golden round-trips |
| titania-policy | 1,620 | A− | Clean. Date math rigorous, digest deterministic |
| titania-aggregate | 1,394 | A | Best-in-repo. Findings/failures correctly separated (DoD #6 ✓) |
| titania-check | 2,239 | B+ | Honest architecture, real exit-code-mapping bug |
| titania-lanes (src) | ~6,000 | B+ | Discipline good; orphan files + probe-subprocess gaps |
| titania-lanes (bins) | ~9,000 | A− | All 28 bins functionally correct |
| titania-lanes (tests) | ~15,000 | A− | No `is_ok()`-only assertions; one weak fuzz test |
| titania-output | 292 | B | Manual Error impl violates thiserror mandate |
| titania-dylint | 1,211 | B+ | Lint-registration sound; "type-aware" overstated for 3/5 |

**No CRITICAL findings. No source-level policy violations remain in committed code** (the 3 in `run_cargo_lane.rs` are fixed in-flight).

## Findings by severity

### HIGH (4)

**H1. `titania-check/src/main.rs:143-146` — `run_lane` exit-code mapping bug**
`run_lane` forwards `LaneExit::as_u8()` directly as the process exit code. `LaneExit::Failure` (=3) surfaces as `input_error` (=3), `LaneExit::Usage` (=2) as `policy_error` (=2). The CLI contract is 0=pass/1=reject/2=policy_error/3=input_error/4=internal_error — `run-lane` can never emit exit 4. **Consequence:** the in-flight `check_scope_lanes`'s `.find(is_internal_error)` short-circuit is dead code (no lane disposition ever reaches code 4). Tests at `cli_dispatch.rs:229,263` codify the wrong mapping.
**Fix:** add `const fn map_lane_exit(LaneExit) -> u8` mapping `Failure → 4`, `Usage → 3`; call `execution.exit()` (enum) instead of `.as_u8()`.
**Fold into:** tn-pdn (killer-demo `--scope` path depends on it) and tn-fqd (thin executor).

**H2. `titania-dylint/src/lib.rs:272-345` — lint-group bypass**
`is_required_lint` is a string whitelist. `#![allow(unused)]`, `#![allow(clippy::correctness)]`, `#![allow(clippy::style)]`, `#![allow(clippy::suspicious)]`, `#![allow(clippy::complexity)]`, `#![allow(clippy::perf)]` silently subsume required lints — none are flagged. Real bypass route.
**Fix:** expand whitelist with documented group→lint mappings, or query `cx.lint_store()` at runtime.
**Fold into:** tn-d2h.

**H3. `titania-dylint/src/lib.rs:163-169` — `BYPASS_ATTR_CONTEXT` narrower than spec**
Spec §7:310 frames the rule as "`#[allow]` in proc-macro-expanded code" generally; implementation only catches it on `pub` items (dispatch lives inside `PubAllow` LateLintPass). Private macro-expanded allows slip through.
**Fix:** either tighten spec wording to match implementation (recommended — current behavior is more useful), or add an early-pass hook that fires on every macro-expanded `#[allow]`.
**Fold into:** tn-d2h.

**H4. `titania-dylint` vs `tn-d2h` bead vs spec §7 — three-way drift**
- Spec §7 lists 5 BYPASS rules.
- Implementation delivers those 5.
- tn-d2h bead lists 6 *different* rules (`RESULT_STRING_ERROR_TYPED`, `UNWRAP_IN_MACRO_EXPANSION`, `ASYNC_IN_SYNC_VIA_TRAIT`, …), names the crate `titania-lints` (actual: `titania-dylint`), and cites `rustc_private::Table<Ty>` / `ExprUseVisitor` toolkit absent from source.

DoD #12 cannot be marked "done" until this is reconciled. **Fold into:** tn-d2h (scope reconciliation is the bead's first deliverable).

### MEDIUM (9)

**M5. `titania-check/src/main.rs:148-153` — `missing_doctor` returns `input_error=3`**
"Not implemented" is not user input's fault. Should be `internal_error=4`. Tests codify the wrong value.
**Fold into:** tn-4rq.2.

**M6. `titania-check/src/aggregate.rs:184-186` — `digest_optional_file` conflates `NotFound` with permission-denied**
Substitutes `missing-cargo-toml` marker digest for ANY read failure. Present artifacts + unreadable Cargo.toml still issues a (fraudulent) receipt.
**Fix:** match on `io::Error::kind()`; only substitute marker for `NotFound`; map others to `AggregateError::Read`.
**Fold into:** tn-ed4 (residual risk for v1 sign-off).

**M7. `titania-check/src/args/parse.rs:43-51` — explicit `check` subcommand not recognized**
`titania-check check --scope edit` → `UnknownSubcommand("check")`. v1-spec lists `check` as a subcommand.
**Fold into:** tn-fqd.

**M8. `titania-check` — `--emit` and `--out` flags silently discarded**
Parsed, validated, stored on `CheckOptions`/`AggregateOptions`/`DoctorOptions`, then dropped by `dispatch`. Tests pass the flags but only assert exit code.
**Fold into:** tn-fqd.

**M9. `titania-lanes/src/helpers.rs:62-66` — `walk_rs_files` silently swallows `read_dir` failure**
Permission-denied → zero findings (no typed error). Function is `pub`.
**Fold into:** tn-bdl (finding-domain boundary work).

**M10. `titania-lanes/src/{panic_scan_lane.rs:90, dylint_lane.rs:88, policy_run_lane.rs:48}` — raw `Command::new` probes bypass `CommandIn` contract**
Three `<tool> --version` / `date +%F` probes skip timeout/output-budget/UTF-8 contracts mandated by `command.rs:1-6`.
**Fold into:** tn-d2h (covers dylint probe) + tn-b2w (general lane hygiene).

**M11. `titania-lanes/src/ast_grep_lane/rules/detectors.rs:189-193` — `detect_core_infra_import` skips comment/string stripping**
Uses raw `source.contains(needle)` while all other detectors use `detect_code_line`. Doc-comment mentions of `use tokio::` false-positive.
**Fold into:** tn-5c2 (ast-grep catalog with repair hints).

**M12. `titania-lanes/src/bin/check_hot_cold_forbidden_apis/syntax.rs:122` — `strip_chars` recursive**
One stack frame per char. Stack-overflow risk on long/minified source lines. Sibling bin does identical logic iteratively.
**Fold into:** tn-kpm.

**M13. `titania-lanes/src/bin/check_ignored_fallible_results/source.rs:99-115` — raw-string-blind string stripper**
Doesn't handle `r"..."`/`r#"..."#`. A `"` inside a raw string corrupts state.
**Fold into:** tn-kpm.

### MAJOR design/quality (6, all titania-core)

**Q14.** `Report`, `QualityReceiptV1`, `Location::Span`, `RepairHint::Patch` expose `pub` fields/variants that bypass smart constructors. Illegal states representable; tests demonstrate the bypass.
**Q15.** Two divergent `RECEIPT_SCHEMA_VERSION` constants (`u32 = 2` for `ReceiptEnvelope`, `u16 = 1` for `QualityReceiptV1`).
**Q16.** `Report::pass` doesn't validate that `per_lane` outcomes are pass-shaped.
**Q17.** `crates/titania-core/src/finding.rs:1-5` — module doc-block malformed (`// !` instead of `//!`); 4 lines silently became ordinary comments.
**Q18.** `RuleId::MAX_LEN = 96` declared but never enforced.
**Q19.** `titania-output` uses hand-rolled `impl std::error::Error` instead of `thiserror` (AGENTS.md mandate).
**Fold into:** tn-bdl (finding-domain) for Q14-Q18; tn-4rq (output) for Q19.

### Notable LOW / NIT

- Orphan files `ast_grep_lane/{pathing,source_clean}.rs` (dead code) → tn-5c2
- `forbidden_scan` doesn't detect `unwrap_or`/`unwrap_or_default`/`unwrap_or_else` (AGENTS.md §5 bans these) → tn-5c2
- `fuzz_minimization` inline test returns `ExitCode::FAILURE` (doesn't fail `#[test]` — always passes) → tn-kpm
- `check_error_exhaustiveness`/`check_stepstate_matrix` miss single-line tuple variants → tn-kpm
- `REQUIRED_LINTS` not synced with `[workspace.lints]` via test → tn-d2h
- `check_test_integrity/scan.rs:252` count-based `weakened_assertions` fires false-positive on consolidation → tn-kpm
- `dylint` tests Linux-only (`.so` hardcoded); spec claims macOS/Windows → tn-d2h

## tn-kpm reality check

The bead's "295 strict-clippy lints in lane binaries" is **mislabeled**. The bins have **zero** real policy violations (no unwrap/expect/panic/for-loop/unsafe in production paths). The 295 is **pedantic/nursery style debt** (format!, clone, to_owned, missing docs). Recommend re-labeling the bead to "clear pedantic/nursery lint debt" so it isn't confused with a correctness/safety issue.

## Killer-demo functionality verdict

**Wired end-to-end at the CLI boundary (in-flight).** `tests/killer_demo.rs` (626 LOC) + `fixtures/strict_ai_loop_unwrap/{bad,repaired}` exist as untracked tn-pdn work. The in-flight `main.rs` diff adds a real `check_scope` executor that runs lanes (not just aggregates). The chain `killer_demo.rs → check_scope → execute_scope_lanes → run_lane → execute_lane → aggregate` runs real lanes, writes real artifacts, emits real JSON.

**Caveat:** H1 (exit-code mapping bug) means `check_scope_lanes`'s internal-error short-circuit is dead code. The killer-demo E2E doesn't trip this because the bad fixture produces `LaneExit::Violations` (exit 1) → `Reject` (1), not `Failure`. The bug is latent on direct `run-lane` paths.

## Verified-clean files (read in full, zero issues)

titania-core: `digest.rs`, `error.rs`, `text_range.rs`, `lane.rs`, `diagnostic.rs`, `discover.rs`, `target_project.rs`, all `receipt/` submodules.
titania-policy: `lib.rs`, `exceptions.rs`.
titania-aggregate: `lib.rs`, `receipt_builder.rs`, `report_assembly.rs`, `artifact_reader.rs` (source).
titania-check: `explain.rs`, `args.rs` Default impls, `main.rs` exit/write helpers, `aggregate.rs::ReportStatus::from_report`.
titania-lanes bins: `kani_list`, `check_beads_server_mode`, `policy_scan`, `verify_lean`, `run_cargo`, `run_tlc_checks`, `rust_verification_gauntlet`, `loom_list`, `flux_check_package`, `bench_instruction_counts`, `check_agent_cli_contract`, `check_workspace_assertions`, `check_verus_production_binding`, `check_source_length`, `check_production_inner_drift`, `hotpath_scan`.
titania-dylint: full `src/lib.rs` policy-clean (panic-free, iterator-only, documented unsafe waivers).
