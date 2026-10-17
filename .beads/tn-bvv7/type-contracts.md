# Type Contracts — tn-bvv7

All new production code is total synchronous predicates:

- `detect_core_process_import(&str) -> bool`
- `detect_core_async_effect(&str) -> bool`
- `detect_core_state_effect(&str) -> bool`
- `code_contains_token(&str, &str) -> bool` (private)
- `leading_boundary_ok(&str, usize) -> bool` (private)
- `trailing_boundary_ok(&str, usize, &str) -> bool` (private)
- `is_ident_char(char) -> bool` (private)

Contracts:
- Inputs are untrusted source text; outputs are booleans. No I/O, no
  clock, no allocation beyond the stripped-code buffer, no panics
  (checked splits, saturating arithmetic, iterator pipelines only).
- Boundary rule: a marker ending in `::` needs only a leading
  identifier boundary; any other marker needs both, so `rand::Rng`
  rejects `rand::RngCore` and `std::time::Instant` rejects
  `std::time::Instantaneous`.
- Extended detectors preserve all previously accepted inputs (alias
  forms were already caught; clean inputs stay silent — pinned by
  `tn_bvv7_aliased_fs_import_stays_violation`,
  `tn_bvv7_boundary_names_do_not_fire`, and the frozen-catalog test).
