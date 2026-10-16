# Trusted-Base Plan — tn-6r36

No trusted markers are introduced by this contract bead. The classifier,
once implemented in follow-up beads, must run with zero `#[trusted]` /
`#[ignore]` / `#[extern_spec]` surface: every spec binds to production via
`#[path]` + `assume_specification` bridge (STRONG binding). Verus/Kani
toolchains themselves (verus binary, cargo-kani 0.67.0) are the ambient
trusted base, pinned by version in obligation tool_metadata. No ledger rows
required at planning state; `trusted-base-ledger.jsonl` lands with executed
evidence at state 12.
