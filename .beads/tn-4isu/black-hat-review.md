# Black-hat review — tn-4isu

reviewer_skill: black-hat-reviewer
reviewer_invocation_id: inv-s13-blackhat
writer_invocation_id: inv-s12-formalverifier
review_state: 13
reviewed_artifacts: crates/titania-core/src/kani.rs, verification-ledger.jsonl, proof-test-source-alignment.md, trusted-base-ledger.jsonl, evidence/kani-*.log

## Proof/Test/Source Parity Matrix

| Claim | Proof evidence | Test evidence | Source state | Parity |
|---|---|---|---|---|
| 9/12 harnesses bounded-verified | 9 SUCCESSFUL raw logs w/ versions+bounds | cargo suite 96/96 green, no drift | attributes-only diff | HOLDS |
| diff blocked (u32/u16 timeouts) | EXIT 124 logs, 7.5k/13.6k unwind profiles | n/a (proof-only) | bound 16 kept, no weakening | HOLDS (blocked, numbered) |
| wire blocked (u64/u24/u12 timeouts) | EXIT 124 logs, 356-copy float profile | outcome_serde 22/22 green (no drift) | bound 12 kept, no weakening | HOLDS (blocked, numbered) |
| id blocked (u100 timeout) | EXIT 124 log, 2147 steps | unit tests 8/8 green | bound 100 kept | HOLDS (blocked, numbered) |
| package infeasible | floor-sum arithmetic >1290s vs 600s | n/a | lane constants untouched | HOLDS |

## Attacks attempted

1. Property reduction? `git diff` = 13+/3-, every line a `#[kani::unwind]` attribute
   or prose comment. Asserts/assumes/covers/inputs/production all byte-identical.
   ATTACK FAILS (no reduction).
2. Theater? No new tests written to hide behind; no cover!-as-proof (every property
   keeps production asserts); timeouts reported as FAIL_LOCAL, never passes.
   ATTACK FAILS (no theater).
3. Laundering? No stubs of serde, no domain shrinkage (8-byte wire domain, 32-byte
   id domain, 8-candidate diff domain all intact), no weakening flags in any
   command (`--no-unwinding-checks` etc. absent from all logs). ATTACK FAILS.
4. Weak unwinds? Every shipped bound is either passing with assertions on (9/12)
   or a least-bad bound with timeout evidence and no unwinding failure (3/12).
   No bound is asserted sufficient without a SUCCESSFUL log. ATTACK FAILS.
5. Hidden behavior change? cargo suite green + outcome_serde negative control
   green + diff inspection. ATTACK FAILS.
6. Ledger/provenance forgery? Hash chain validates (validator PASS at 12);
   one mechanical re-chain after a disclosed content evolution, recorded for
   the cleanup report. Same-agent review limits disclosed, never presented as
   independent. ATTACK FAILS (limits disclosed, math checks).

## Residuals (not defects)

- R1: same-agent review chain (delegation environment lacks named-specialist
  child types). Carried openly; validator judges.
- R2: machine shared with fleet processes; wall-clock numbers are noisy upper
  bounds on an unloaded machine — timeouts are robust to noise (orders of
  magnitude over budget), passes are exact.
- R3: three structural blockers stand; bead must stay OPEN.

## STATUS: APPROVED
