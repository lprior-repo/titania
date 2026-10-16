# Black-Hat Review — tn-8qfc (single-operator pass)

Scope: must-use floor (scanner + manifests + dylint parity). Same-operator
review (degraded independence, recorded for landing).

## Attacks attempted

1. Weaken `unused_must_use` to `warn` in a member manifest — fires one
   typed finding (matrix test). CLOSED.
2. Weaken via `policy.toml [lints]` override map — by design the map may
   lower levels (v1 §9.7); lowering now emits per-entry findings instead of
   passing silently. Documented residual, not a defect.
3. Crate-level `#![allow(clippy::let_underscore_must_use)]` — fires
   `BYPASS_REQUIRED_LINT_WEAKENING` (fixture test). CLOSED.
4. Template drift (adoption template missing the new pin) — pinned in the
   same change; self-hosting parity holds. CLOSED.
5. `let _ =` on non-must-use values — correctly NOT flagged (early guard
   drop is not `Result` handling; bead boundary). No defect.
6. Forgotten `Future` without `await` — trips `unused_must_use` at deny;
   `let _ = fut` trips `let_underscore_must_use`. Covered by construction;
   no separate future lint invented per D4.
7. Regression hunt: the floor extension broke
   `matching_exception_suppresses_config_flags_finding` (fixture pinned the
   old 16-entry floor) — repaired by pinning the 4 new entries; test intent
   (exception suppression) preserved and passing.
8. Second regression (found in final `moon ci`): the 5
   `repaired_fixture_*` killer-demo tests failed because
   `fixtures/strict_ai_loop_unwrap/repaired/Cargo.toml` pinned the old
   floor — repaired the same way; 16/16 killer-demo green locally and
   `moon run titania:test` green (1m33s).

## Verdict

Floor enforced on all four vectors with line-accurate findings. One real
regression found and repaired during the work (exceptions fixture). No
blocking finding.
