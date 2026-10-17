# Boundary map — tn-c6n

- Pure core: all six smart constructors and their guard predicates. No I/O,
  no time, no randomness, no global state.
- Text boundary: JSON text enters only through `Deserialize` impls, each of
  which funnels into the matching constructor (or rejects the schema first).
  Callers supply `&str`/`String`/integers; the core never reads files, env, or
  clocks (timestamps arrive as `u64` arguments).
- Action shell (out of scope): assembling target projects, writing receipt
  files, CLI dispatch. This bead tests the core only, through public APIs,
  with real constructors and real JSON text — no mocks, no mirrors.
- Verifier boundary: the registered `verification/verus/receipt_schema.rs`
  binds the schema `const fn` by source path with an assumed contract
  (classified source-linked per `tn-083n`); it cannot observe constructor
  bodies, so constructor behavior is evidenced by executed tests instead.
