# Proof evidence — tn-c6n (State 5, proof-writer)

Executed lane: `verus` for obligation `tn-c6n.O-V6` (R6 schema gate, binding
hygiene only). No new spec files were authored: the plan forbids mirror
types, and the registered binding file is registry-owned and untouched.

## Executed command (raw)

- Command: `verus verification/verus/receipt_schema.rs`
- Workdir: isolated worktree root (see STATE.md)
- Tool: `Verus 0.2026.05.05.d03e906, Profile: release`
  (`evidence/verus-present.log`)
- Exit status: 0. Full log: `evidence/verus-state5.log`.
- Output tail: `verification results:: 3 verified, 0 errors`

## Classification (binding, per tn-083n)

The run proves the binding file is well-formed and its three claims check
out under the pinned verifier. It does not verify any constructor body: the
contract on the production `const fn` is source linkage recorded in TBL row
`tn-c6n.T1`, and the literal-equality lemma in the same file is vacuous
(`2 == 2` proves nothing about behavior). Zero behavior weight rests on this
run; 100% of R6 behavior weight rests on obligations `O-P6` (executed at
States 9/12 through public entry points).

## Marker inventory for this bead

- Pre-existing marker in the registered file (line 8, assumed contract on
  the production const fn): recorded as TBL row `tn-c6n.T1` with
  compensating evidence `O-P6`. No new markers introduced anywhere.
- New test/property files (States 9–12) will use exact assertions over
  production constructors with no assumed contracts and no body-erasure
  markers by construction; the State 6/10/13 reviews re-scan for them.

## Pending downstream evidence (not claimed here)

`O-P1`..`O-P6` execute at States 9/12 via `cargo test` over the two new
test targets; their ledger rows close at State 12 with raw logs.
