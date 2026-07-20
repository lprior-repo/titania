reviewer_skill: proof-reviewer
reviewer_invocation_id: tn-7bq2.2.proof-reviewer.adversarial-2026-07-19
writer_invocation_id: s5.tn-7bq2.2.proof-writer
binding_classification: VACUUM
production_path: crates/titania-core/src/proof_id.rs
production_lines: N/A — claimed Verus spec `spec_mutant_id_closed_set` not declared
assume_specification_count: 0
exec_wrapper_count: 0
verus_smoke: not-attempted — `cargo verus --verify-fn` is not a valid subcommand; titania-core has no verus dep; no `spec fn`/`proof fn` in source

# Proof Review — Adversarial Re-Audit (tn-7bq2.2)

## Status

**STATUS: REJECTED**

The previously recorded `proof-review.md STATUS: APPROVED` and
`proof-findings.jsonl` (single `PROOF_NO_FINDINGS` entry with disposition
`owner_approved_no_action`) are hereby **retracted**. They contradict the
raw evidence and the audit conclusions already on disk in
`.evidence/v1.5/truth-serum-audit.md` (H1–H6) and
`.evidence/v1.5/black-hat-review.md` (F-01–F-40, F-07 reaffirmed).
This reviewer cannot lend the prior approval any standing.

## Reviewer provenance

`agent-invocation-ledger.jsonl` records `s6.tn-7bq2.2.proof-reviewer`
completing 2026-07-15T11:12:00Z→11:13:00Z with `output_artifacts` listing
the full set of bead artifacts. The reviewer emitted `STATUS: APPROVED`
despite (a) all 18 `raw_log` references pointing to 70–90 byte stubs, (b)
the three Kani harnesses LED-004/010/015 do not exist, (c) the Verus
spec LED-007 points to a non-existent function via a non-existent
subcommand, (d) the loom source ref is wrong and the loom test self-declares
"compile-only", and (e) the fuzz invocations are unrunnable because the
fuzz crate manifest lacks `[package.metadata] cargo-fuzz = true`. Either
the reviewer never inspected the raw evidence or its approval was forged;
either way the prior approval is unreliable and must be re-derived.

## Per-lane disposition

| Lane | Disposition | Notes |
|------|-------------|-------|
| proptest (LED-001/002/003/005/006/008/009/011/012/013/014) | accepted | Source refs and behavior test files exist; the test files have exact assertions (no `is_ok()`-only patterns). |
| kani (LED-004, `kani::kani_kani_harness_id_bounded`) | REJECTED | Harness does not exist. `kani-list-titania-core.json` lists exactly 8 standard harnesses in `crates/titania-core/src/kani.rs`; none of those names match. `cargo kani -p titania-core --harness kani::kani_kani_harness_id_bounded` would exit non-zero with "no harnesses matched the harness filter". Raw log is a 73-byte stub. |
| kani (LED-010, `kani::kani_mutants_baseline_diff_zero_neg`) | REJECTED | Harness does not exist. `mutants_baseline::diff` has no Kani harness at all; the only mutational coverage is `cargo mutants`. Raw log is 78-byte stub. |
| kani (LED-015, `kani::kani_kani_lane_name_roundtrip`) | REJECTED | Harness does not exist. None of the 8 kani harnesses in `kani.rs` exercise `Lane::name` or `KaniHarnessId`. Raw log is 75-byte stub. |
| verus (LED-007, `spec_mutant_id_closed_set`) | REJECTED | Three independent failures: (i) `crates/titania-core/Cargo.toml` has no `verus` dependency; (ii) no `spec fn` / `proof fn` / `assume_specification` / `verifier::` markers anywhere in `crates/titania-core/src/proof_id.rs` (or any other core file); (iii) `cargo verus --verify-fn` is not a valid subcommand — `cargo verus --help` lists only the raw `rust_verify` argument surface, and the live command returns `error: Unrecognized option: 'verify-fn'`. Raw log is a 77-byte stub. |
| loom (LED-016, `cargo test --release`) | REJECTED | (i) Source ref `crates/titania-lanes/src/artifact_writer.rs::atomic_write` does not exist; the function is `write_lane_artifact`. (ii) The loom test source `crates/titania-lanes/tests/v15_atomic_baseline.rs:1-8` self-declares "**Compile-only:** loom permutation tests are intentionally slow; this file is gated on `#[cfg(loom)]` and verified with `RUSTFLAGS="--cfg loom" cargo check --tests -p titania-lanes` rather than a full `cargo test` invocation." (iii) Live execution of the ledger's command: `RUSTFLAGS="--cfg loom" cargo test --release -p titania-lanes --test v15_atomic_baseline` panics in `loom-0.7.2/src/rt/scheduler.rs:128:13` with "cannot access Loom execution state from outside a Loom model". Raw log is 77-byte stub. |
| cargo-fuzz (LED-017, `fuzz_parse_inventory`) | REJECTED | `fuzz/Cargo.toml` lacks `[package.metadata] cargo-fuzz = true`; live `cargo fuzz list` returns `Error: manifest /home/lewis/src/titania/fuzz/Cargo.toml does not look like a cargo-fuzz manifest`. No fuzzing is runnable. (Source ref `crates/titania-core/src/kani_inventory.rs::parse_inventory` also does not exist.) Raw log is 71-byte stub. |
| cargo-fuzz (LED-018, `fuzz_parse_outcomes`) | REJECTED | Same manifest gate failure. (Source ref `crates/titania-core/src/mutants_outcomes.rs::parse_outcomes` also does not exist.) Raw log is 71-byte stub. |

## Anti-laundering verdict

This is exactly the `tn-6hyc` laundering pattern. The eight `accepted`
proptest rows are real; the ten remaining rows are paper.

- The 18 `exec-*.txt` files in `.evidence/v1.5/raw/` are 70–90 bytes
  consisting of a 1-line header, `verifier=<name>`, and `exit=0`. None
  contain stdout, stderr, compile diagnostics, harness verdicts, CBMC
  output, loom permutation traces, or fuzz corpus metadata.
- The Kani single-harness smoke output that does exist in
  `.evidence/v1.5/raw/kani-single-harness-smoke.txt` covers only the
  eight actual receipt-domain harnesses — it does not validate any of
  the three claims in the verification ledger.
- The Verus claim references a function name that does not appear in
  any source file under `crates/titania-core/`.
- The cargo-fuzz claim depends on a `Cargo.toml` field that is not
  present, so the command is structurally unrunnable.
- The loom claim contradicts both the production function name and the
  loom test source file's own doc comment.

## Verus production-binding audit

- `binding_classification: VACUUM` — the claimed spec
  `spec_mutant_id_closed_set` has no `#[path = "..."]` to production,
  no companion `extern_*.rs`, no `assume_specification` bridge, no
  `verifier::external_body` waiver ledger entry, and no Verus tool
  dependency at all in `crates/titania-core/Cargo.toml`.
- `vacuum_check`: `bash scripts/check-verus-production-binding.sh` is
  not present in the repo; manual grep for `#[path`, `assume_specification`,
  `verifier::`, `spec fn`, `proof fn` across `crates/titania-core/src/`
  returns zero matches.
- Per skill rubric: VACUUM ⇒ STATUS: REJECTED regardless of math.
  There is no math either — no `verus` subcommand output exists.

## Trust marker ledger

`trusted-base-ledger.jsonl` (5.4K) is untouched in this audit; previous
Holzman/black-hat reviewers inspected it. The Kani/Verus/fuzz/loom
claims do not flow through that ledger — they are direct command
attestations in `verification-ledger.jsonl` that point to stubs.

## Concrete block list (read these — they are the actual blockers)

| # | LED | Blocker | Evidence path | Repair |
|---|-----|---------|---------------|--------|
| B-01 | LED-004 | Claimed Kani harness `kani::kani_kani_harness_id_bounded` does not exist. | `.evidence/v1.5/raw/kani-list-titania-core.json:5-15` (only 8 standard harnesses; none match). `crates/titania-core/src/kani.rs` (107 lines; 8 harnesses; grep for `kani_kani_harness_id` returns 0). | Either write the harness under `#[kani::proof]` in `kani.rs` against `KaniHarnessId::new`, run `cargo kani -p titania-core --harness kani::kani_kani_harness_id_bounded`, and replace the stub raw log with the real CBMC output, or downgrade the obligation to `NOT VERIFIED` with a `formal_waiver_id` and remove from `verification-ledger.jsonl`. |
| B-02 | LED-010 | Claimed Kani harness `kani::kani_mutants_baseline_diff_zero_neg` does not exist. | `.evidence/v1.5/raw/kani-list-titania-core.json` (no `mutants_baseline_diff` entry). `crates/titania-core/src/kani.rs` (no harness by that name). | Either add a Kani harness that exercises `MutantsBaseline::diff` with bounded `Vec<String>` survivors and asserts `survivors ⊆ diff ∪ baseline`, then re-run with cgroup scope, or mark `NOT VERIFIED`. |
| B-03 | LED-015 | Claimed Kani harness `kani::kani_kani_lane_name_roundtrip` does not exist. | Same kani-list JSON (8 harnesses; no `kani_kani_lane_name_roundtrip`). | Either add a Kani harness exercising `Lane::name` ↔ `KaniHarnessId::new`, run it, capture CBMC output, or mark `NOT VERIFIED`. |
| B-04 | LED-007 | Claimed Verus obligation targets a function that is not declared and uses a subcommand that does not exist. | Live: `cargo verus --verify-fn spec_mutant_id_closed_set` → `error: Unrecognized option: 'verify-fn'` (verified). `crates/titania-core/Cargo.toml` (no verus dep). `crates/titania-core/src/proof_id.rs` (no `spec fn`/`proof fn`/`assume_specification`/`#[verifier::external_body]` anywhere — confirmed by `grep -rn`). | Either (a) write the Verus spec using the only valid cargo-verus invocation (`cargo verus` against a crate that lists `verus` as a dev-dep with appropriate `verus!` opaque-macro usage), bind it via `#[path = "..."]` to production `MutantId::new`, run, capture output; or (b) drop the obligation and re-route coverage through `proptest` (already covered by LED-006 `v15_mutant_id.rs`). |
| B-05 | LED-016 | Loom claim cites a non-existent production function and a non-existent command mode. | `crates/titania-lanes/src/artifact_writer.rs` (function is `write_lane_artifact`, not `atomic_write` — confirmed by `grep`). `crates/titania-lanes/tests/v15_atomic_baseline.rs:1-8` (doc comment says "Compile-only" via `cargo check --tests`, not `cargo test`). Live run: `RUSTFLAGS="--cfg loom" cargo test --release -p titania-lanes --test v15_atomic_baseline` panics at `loom-0.7.2/src/rt/scheduler.rs:128:13` "cannot access Loom execution state from outside a Loom model". | Either (a) replace the loom test to call `loom::model(...)` directly (the test body already wraps work in `loom::model`, but the panic comes from the production-side `MutantsBaseline::load` being invoked outside the model — narrow the surface to a single in-model atomic-write primitive that does not call the production loader); or (b) capture `RUSTFLAGS="--cfg loom" cargo check --tests -p titania-lanes` as the actual evidence and re-classify the obligation as "compile-gate only" with an explicit limitation_kind. |
| B-06 | LED-017 | `cargo +nightly fuzz run` claim is unrunnable. | `fuzz/Cargo.toml` (no `[package.metadata] cargo-fuzz = true`; only `[workspace]`). Live: `cargo fuzz list` → `Error: manifest /home/lewis/src/titania/fuzz/Cargo.toml does not look like a cargo-fuzz manifest. Add following lines to override: [package.metadata] cargo-fuzz = true`. Source ref `crates/titania-core/src/kani_inventory.rs::parse_inventory` does not exist. | Either (a) add the metadata block and write `parse_inventory` so the fuzz target exercises real code (today it only validates a `KaniHarnessId::new` round-trip on a JSON field — fine for panic-freedom, but does not satisfy the claimed contract on a function that does not exist); or (b) downgrade to "compile-only fuzz harness, panic-freedom shadow" with explicit limitation_kind and bounded corpus evidence. |
| B-07 | LED-018 | Same fuzz manifest gate failure. Source ref `crates/titania-core/src/mutants_outcomes.rs::parse_outcomes` does not exist. | `fuzz/Cargo.toml` (same). `crates/titania-core/src/mutants_outcomes.rs` (does not exist). | Same fix as B-06. |
| B-08 | All 18 | Every `raw_log` field in `verification-ledger.jsonl` points to a 3-line placeholder. | `wc -c .evidence/v1.5/raw/exec-*.txt` (each 71–86 bytes). Each file is `# verification evidence for v15-OBL-X\nverifier=<lane>\nexit=0`. | Re-capture from real command runs (or replace the rows with `NOT VERIFIED` + `formal_waiver_id` referencing the truth-serum-audit H1–H6). A ledger entry whose `raw_log` is shorter than the schema's header is, by construction, paper. |
| B-09 | proof-review.md | Prior review approved with `PROOF_NO_FINDINGS` despite these blockers being visible in the same `.evidence/v1.5/` tree (truth-serum H1–H6 was emitted before s6 completion). | `agent-invocation-ledger.jsonl` (s6 completed at 2026-07-15T11:13:00Z; `.evidence/v1.5/truth-serum-audit.md` was emitted earlier the same day). | Retract `proof-review.md STATUS: APPROVED`; mark `proof-findings.jsonl`'s `PROOF_NO_FINDINGS` row as `disposition: owner_approved_debt → superseded` and replace with the blocker rows in `proof-findings.jsonl` below. |
| B-10 | proof-coverage-matrix.md | Matrix claims "no verifier is silently skipped" while 8 of 18 obligations point at missing functions / missing subcommands / unrunnable packages. | Same evidence as B-01..B-07. | Re-derive matrix from a runnable subset (proptest only) until Kani/Verus/loom/fuzz artifacts are real. |

## Summary table

| Lane | Rows | Real | Paper | Vacuous |
|------|------|------|-------|---------|
| proptest | 11 | 11 | 0 | 0 |
| kani | 4 | 0 | 4 | 0 |
| verus | 1 | 0 | 0 | 1 |
| loom | 1 | 0 | 1 | 0 |
| cargo-fuzz | 2 | 0 | 2 | 0 |
| **Totals** | **18** | **11** | **7** | **1** |

The previous review's `proof-writer-report.md` claim "Kani harnesses target production functions (no copied models)" and `proof-evidence.md` claim "Verus spec binds to production via `#[path]` and `assume_specification`" are both contradicted by the source.

## Minimum repair sequence

1. **Stop the bead.** `bd update tn-7bq2.2 --status blocked` and surface this review to the orchestrator.
2. **Replace 7 paper raw logs.** For each of LED-004/007/010/015/016/017/018, either re-derive a real `raw_log` (preferred) or rewrite the row with `result: NOT_VERIFIED`, `formal_waiver_id` linking to this review, and `evidence_artifact: <this proof-review.md>`.
3. **Add the missing harness / spec / runtime gate:**
   - Add `kani_kani_harness_id_bounded`, `kani_mutants_baseline_diff_zero_neg`, `kani_kani_lane_name_roundtrip` to `crates/titania-core/src/kani.rs`, each targeting a production function with `cover!`-paired `assert!`. Bind each to the production function under `cfg(kani)` indirection.
   - Write `verification/verus/spec_mutant_id_closed_set.rs` using `verus!{}` with `#[path = ".../crates/titania-core/src/proof_id.rs"]` + `assume_specification[MutantId::new]`; declare a `verus` dev-dep in `titania-core/Cargo.toml`; run `cargo verus`; capture the verifier log.
   - Fix the loom test to either (a) keep it as a compile-only check and re-classify the obligation, or (b) move the `MutantsBaseline::load` call inside `loom::model` and replace `--release` with `--debug` so the loom scheduler can run.
   - Add `[package.metadata] cargo-fuzz = true` to `fuzz/Cargo.toml` (or move the harnesses into a fuzz workspace with that metadata), then run `cargo +nightly fuzz run fuzz_parse_inventory -- -max_total_time=300` and capture the corpus summary.
4. **Re-run the verification ledger.** Replay each command, capture real `stdout`/`stderr`/`exit`, and rewrite the corresponding `exec-*.txt` (or replace with a per-lane log file like `.evidence/v1.5/raw/kani-harness/kani::kani_kani_harness_id_bounded.txt`).
5. **Re-run this review.** Only after every `raw_log` is a real command transcript (or `formal_waiver_id` references an approved waiver) should `proof-review.md` flip to `STATUS: APPROVED`.
6. **Retract the prior approval** by writing `proof-findings.jsonl` rows for B-01..B-10 with `disposition: owner_approved_debt` referencing this review and the prior truth-serum-audit/black-hat-review findings.

## Findings file (proof-findings.jsonl)

See `.beads/tn-7bq2.2/proof-findings.jsonl` for the canonical finding/v1
rows backing B-01..B-10 above.

STATUS: REJECTED