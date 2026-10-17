# Black-hat review — tn-vr2n

reviewer_skill: black-hat-reviewer
reviewer_invocation_id: s13.tn-vr2n.black-hat-reviewer

STATUS: APPROVED

## Proof/Test/Source Parity Matrix

| Claim | Source | Test | Verdict |
|---|---|---|---|
| C1 guard rejects ceiling extras | `invocation.rs::validate_extra_args` + `run_lane` guard | `run_cargo_invocation` (8) + wiring test | parity holds |
| C2 static argv ceiling-free | `args.rs` const tables | negative unit test + exact-match pins | parity holds |
| C3 established paths locked | unchanged scanners/scrub/CLI | 5 ceiling regressions, green pre/post | parity holds |
| Reject maps to Violations | `error_exit` Policy arm | wiring test asserts variant | parity holds |

## Attacks run

- A1 case/unicode/position variants (`--CAP-LINTS`, lookalikes, post-`--`,
  missing level): matcher is presence-based; unknown flags fail closed in
  rustc. No bypass. (Lookalike accept test pins no over-blocking.)
- A2 guard removal / matcher weakening / variant remap: each is killed by a
  named test (mutation checkpoints in test-plan.md). No silent hole.
- A3 second spawn path: `cargo_output` has exactly one caller (`run_lane`,
  guarded); `run_lane_dylint` args are driver-internal, not compiler denies;
  `run_lane.rs` deny args are static. Single enforcement point confirmed.
- A4 receipt forgery: `Clean` evidence is built only inside `clean_outcome`
  after a guarded spawn; no target-controlled input reaches it. No forgery.
- A5 env/config smuggling (`CARGO_BUILD_*`, nested configs, wrappers):
  scrub drops all `CARGO_*` at spawn (proved); configs reject all rustflags
  (proved); wrappers sccache-only (unchanged). No live bypass.
- A6 laundering: zero verifier artifacts, zero trust markers in src
  (`rg` exit 1), behavior tests call production APIs directly. Nothing to
  launder.

## Findings and dispositions

- F1 (note): `-A`/`--allow` weakening is not screened — out of bead scope
  (acceptance names cap-lints only). Tracked as follow-up candidate, not a
  defect. Disposition: owner_approved_no_action (this bead).
- F2 (note): `String` token instead of newtype — deliberate; a newtype
  around one error field is ceremony without safety gain. Disposition:
  owner_approved_no_action.
- F3 (residual): reviews inline-only; independent review pending.
  Disposition: blocker for bead closure, not for handoff.

No contract-parity failure, no panic surface in production code (all
`assert!` matches are below `mod tests` lines 127/184; new module has zero),
no deleted tests (diff is additive; 3 removed lines are fmt reflows of added
lines), no scope creep (no manifest/Moon/scanner changes).
