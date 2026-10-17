# Formal verification report — tn-c6n (State 12)

STATUS: PASS

Executed formal lane: `verus` obligation `tn-c6n.O-V6`.

- Command: `verus verification/verus/receipt_schema.rs` (workdir: worktree).
- Tool: `Verus 0.2026.05.05.d03e906, Profile: release`.
- Exit: 0. Raw log: `evidence/verus-s12.log`.
- Output: `verification results:: 3 verified, 0 errors`.

Scope honesty (unchanged since State 5): this run checks the registered
binding file only. Constructor-body proof is explicitly not claimed; the
assumed contract is ledgered (`tn-c6n.T1`, accepted) and all behavior weight
rests on the executed `cargo test` obligations closed in
`verification-ledger.jsonl`. Ruled-out lanes (kani/flux/verus-S1–S5) stand
on their accepted evidence-backed dispositions; they are residuals, not
passes. No waivers exist in this bead.
