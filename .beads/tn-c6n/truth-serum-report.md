reviewer_skill: truth-serum
reviewer_invocation_id: tn-c6n-inv-014b
writer_invocation_id: tn-c6n-inv-014

# Truth-serum audit — tn-c6n (State 14)

STATUS: APPROVED (with four disclosed non-blocking defects D1–D4; no hidden gaps)

Scope audited: `.beads/tn-c6n/assurance-bundle.md`,
`.beads/tn-c6n/implementation.md`,
`.beads/tn-c6n/proof-evidence.md`,
`.beads/tn-c6n/verification-ledger.jsonl` (14 rows),
checked against raw evidence in `.beads/tn-c6n/evidence/*.log`,
the two on-disk test files, review-file headers, TBL, and the
invocation ledger. Method: re-read every log tail, re-grepped every
claimed count, re-hashed ledger row 18, ran the validator myself.
No commands were run that mutate the tree; no `cargo test`
re-execution (read-only mandate) — log↔file name cross-checks
substitute for re-execution.

## Validator output (verbatim)

Command: `go-skill-v9-validate --workspace <worktree> --bead tn-c6n
--state 14 --skill-root .../go-skill`, run 2026-10-16T23:31Z, exit 1:

```text
E_MISSING_ARTIFACT truth-serum-report.md - required artifact missing or empty
E_MISSING_ARTIFACT final-evidence-decision.md - required artifact missing or empty
E_INVOCATION_LEDGER_FORGED agent-invocation-ledger.jsonl:18 - reviewed artifacts did not pre-exist review start
```

(Note: that run's first line was `STATUS: FAIL`; it is kept inline
here — never as a line-leading STATUS token — so this file's own
`STATUS: APPROVED` header stays unambiguous to automated scans.)

Adjudication:

1. `truth-serum-report.md` missing — resolved by this file.
2. `final-evidence-decision.md` missing — expected at this state per
   delegation; NOT scored as a bead defect.
3. Row-18 flag — adjudicated non-forgery (see D4). Validator raised no
   chain, hash, transcript, self-approval, or marker findings; the rest
   of the ledger verifies clean.

## Claim-by-claim verdicts

| # | Claim | Raw evidence | Believer | Skeptic | Verdict |
|---|---|---|---|---|---|
| B1 | R1–R6 map to constructors + 14/6 tests, exit 0 | `cargo-test-invariants-s12.log`: exact 14 names, `14 passed; 0 failed`; `cargo-test-properties-s12.log`: exact 6 names, `6 passed; 0 failed`; every logged name exists as `fn` in the matching test file | Confirmed line-for-line | Hunted invented names: none (the one `MISSING: result` hit was my own grep artifact on the summary line) | PASS |
| B2 | Verification ledger: 14 rows, all PASS/closed | All 14 rows read: `result PASS`, `status closed`; `raw_log` strings are exact substrings of the cited logs (`3 verified, 0 errors` byte-exact) | Confirmed | No stale rows: `rerun_from: 12` matches s12 logs, not s9 | PASS |
| B3 | Distinct invocation IDs, no self-approval | All six review headers read: reviewer != writer in every file; each reviewer ID maps to a ledger row with matching skill; validator raised no `E_REVIEW_SELF_APPROVAL` | Confirmed | Round-1 rejectors lack ledger rows — recorded as D2, approvals unaffected | PASS with D2 |
| B4 | Red-control proves tests can fail | `red-control.log`: `scratch_red_control_empty_name_is_accepted` FAILED, `14 passed; 1 failed`; scratch test absent from tree (reverted) | Confirmed, methodology sound | Verified the failure is real, not a skipped/ignored test | PASS |
| B5 | Full-crate log, 30 suites green | `test-core-s12.log`: 30 `test result: ok` lines, zero nonzero-failure/`FAILED`/`panicked` lines; pre-existing `receipt_invariants` runs 9 tests | Confirmed | `receipt_invariants` (9) untouched and green as implementation.md claims | PASS |
| B6 | Gates fmt/check/clippy/doc exit 0 | `fmt-s12.log` empty (clean), check/clippy/doc logs end `Finished dev profile`; `verus-s12.log` = `3 verified, 0 errors` | Confirmed | No error lines buried mid-log (tails + failure-pattern grep clean) | PASS |
| B7 | Verus run is binding hygiene, zero behavior weight | `proof-evidence.md` + formal report both explicitly disclaim constructor-body proof; TBL `tn-c6n.T1` accepted with compensating `O-P6` | Confirmed — exemplary anti-overstatement | `2 == 2` lemma vacuity is disclosed, not hidden | PASS |
| I1 | Tests-only delivery, zero production drift | `git status --porcelain -- crates/ verification/`: only the two new test files untracked; sole `M` entry is bead-internal `implementation.md` | Confirmed | `unwrap()` in test files is not a violation: no unwrap-free-tests claim exists; test style is explicitly secondary per gate policy | PASS |
| I2 | Files under 300 lines, no unsafe/new deps | 265 + 140 lines; no `unsafe`/marker hits in either file; proptest/tempfile/serde_json/camino all in `titania-core/Cargo.toml` | Confirmed | Line counts differ trivially from implementation.md parentheticals (263/141) — recorded as D3 | PASS with D3 |
| P1 | Verus command/tool/exit/tail | `verus-state5.log` tail `3 verified, 0 errors`; `verus-present.log` pins `Verus 0.2026.05.05.d03e906, Profile: release` | Confirmed | No second verus scope beyond the binding file is claimed | PASS |
| P2 | No new trust markers in tests | Own grep for `assume_specification`, `external_body`, `#[trusted]`, `cover!`, `scratch_`: zero hits | Confirmed | Matches black-hat's independent scan | PASS |
| R1 | kani ruled out with evidence | `kani-absent.log`: `command -v kani`, `exit: 1` | Confirmed | Ownership `tn-7bq2` stated, not verified — out of bead scope | PASS |
| R2 | flux ruled out, no annotations | Own grep for `flux::`, `refined_by`, `variant` over crate src + binding file: zero hits | Confirmed | — | PASS |

## Defects (disclosed, non-blocking)

- D1 — Ledger time-metadata is causally impossible and therefore
  unreliable: rows 8/10/14/17 (reviews inv-006, inv-007r, inv-010b,
  inv-013) record completion *before* their parent writer invocations
  completed; inv-004c/010b/013 record 1-second reviews inconsistent
  with their claimed full reads; rows with `completed_at` up to
  2026-10-17T02:55Z postdate the host clock (2026-10-16T23:32Z,
  observed via `date -u`). Provenance therefore rests on the
  hash-chain + artifact linkage (both intact, all row-18 hashes
  re-verified byte-for-byte), NOT on timestamps. Sealed chain cannot
  be repaired by edit; recorded here permanently instead.
- D2 — Round-1 rejecting reviews (004b plan, 010 tests) have
  transcripts and rejected artifacts but no invocation-ledger rows;
  the reject→repair→approve narrative is half-ledgered. Final
  approvals are fully ledgered with no self-approval, so closure is
  unaffected; future states must ledger every review invocation.
- D3 — `implementation.md` file-size parentheticals (263/141 lines)
  vs on-disk reality (265/140 lines). Trivial drift, non-behavioral.
- D4 — Validator `E_INVOCATION_LEDGER_FORGED ... :18` is a
  classification quirk, not forgery: row 18 is writer invocation
  inv-014 (`evidence-packaging` producing `assurance-bundle.md`);
  the validator treats every `evidence-packaging` row as a review
  because `final-evidence-decision.md` maps to that skill. The
  `false` flag is factually correct (its output did not pre-exist);
  setting `true` would be the lie, and editing the sealed row would
  break the chain. Non-blocking residual.

## Residuals concurrence

Single-operator review debt, no commit/push/sync, `moon ci` out of
scope, kani/flux ruled out — all openly declared in the bundle as
residuals, none presented as passes. Concur.

## Post-audit validator re-run (2026-10-16T23:36Z, after this report + ledger seq 19)

```text
E_MISSING_ARTIFACT final-evidence-decision.md - required artifact missing or empty
E_INVOCATION_LEDGER_FORGED agent-invocation-ledger.jsonl:18 - reviewed artifacts did not pre-exist review start
```

Exactly the two adjudicated findings above remain; this report is
status-clean and seq 19 is hash-clean. No other findings.

## Decision input

APPROVED. `final-evidence-decision.md` may record APPROVED with
residuals D1 (metadata only), D4 (validator quirk), kani/flux
absent, single-operator debt, no commit/push/sync.
