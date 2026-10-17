# tn-ebn9 Trusted-Base Plan

No deductive proof, model checker, refinement type, fuzzer, loom, or miri
lane is required (see `proof-strategy.md`), so the trusted base is the
standard toolchain plus the checked-in detectors:

- `rustc 1.97.0-nightly (ca9a134e0 2026-04-26)` + `cargo` (typecheck,
  DylintWeeks expansion, test execution).
- tree-sitter Rust grammar via `ast-grep-language` (parse boundary).
- No `assume`/`axiom`/`admit`/`trusted`/`external_body`/`ignore`-style
  markers, no stubs, no disabled checks, no model bounds, and no Kani
  `cover!` anywhere in tn-ebn9 proof artifacts. A closed-world word scan
  over `proof-evidence.md` and `proof-writer-report.md` confirms absence;
  `trusted-base-ledger.jsonl` records the single closed-world declaration
  row (no open markers to track).
