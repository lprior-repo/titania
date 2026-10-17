# tn-ebn9 Proof Strategy

Claim shape: detector verdicts over a finite corpus of syntactic shapes
(loop/while-let/for/repeat/cycle/finite-range). The claim is
classification-correctness over an enumerated input space, not a numeric,
inductive, or concurrent property. Therefore:

- Required: `proptest` (executable corpus evidence). In this repo the
  property corpus is realized as exact-assertion detector tests (Dylint
  fixture suite + ast-grep lane suites + killer-demo E2E); each seed maps
  to named tests with sharp assertions, which is the executable form of
  the planned property.
- Not applicable with evidence per seed:
  - `kani`: no counters, indices, arithmetic, or unwind bounds in any
    detector path (two-variant verdict enum; `u64` never appears).
  - `verus`: single-guard presence/absence checks plus one resolved-identity
    predicate; no inductive invariant to discharge deductively.
  - `flux-rs`: no refined indices, lengths, or numeric refinements cross
    any detector path.
  - `cargo-fuzz`: no byte codec, wire format, or persisted payload; inputs
    are parsed Rust sources, covered by the fixture corpus.
  - `loom`/`miri`: no threads, locks, atomics, channels, unsafe, or FFI in
    detector code paths (workspace `forbid(unsafe_code)` holds; chain the
    `cargo geiger` posture by construction — no new unsafe added).

No behavior-affecting lane is waived: every behavior-affecting seed has a
required proptest obligation with executed PASS ledger evidence.
