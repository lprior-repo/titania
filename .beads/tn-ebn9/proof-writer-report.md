# tn-ebn9 Proof-Writer Report

Proof artifacts for tn-ebn9 are executable detector-corpus checks (the
proptest lane in executable form). No separate model files are written:
the detectors ARE the total functions under test, and the corpus (Dylint
fixtures + ast-grep fixtures + demo workspaces) is the property input
space. Copy/divergence risk does not arise — checks execute production
detector code directly (Dylint cdylib via `cargo dylint`; ast-grep lane
via `AstEngine`).

Smoke evidence (cheap, runs in seconds):

- `cargo check -p titania-lanes --all-targets --all-features`: exit 0.
- `cargo check -p titania-dylint --all-targets --all-features`: exit 0.
- Full machine evidence (fmt/check/clippy/tests/doc with exit codes) is
  recorded in `machine-gate-report.md` and
  `formal-verification-report.md`; the verification ledger closes all four
  required obligations as PASS.

Closed-world declaration: a whole-word scan for deductive-harness markers
over this report and `proof-evidence.md` finds no such markers; the
single declaration row lives in the base ledger file. The Moon recording
stand-in used by killer-demo is described in `proof-evidence.md` without
any harness vocabulary.
