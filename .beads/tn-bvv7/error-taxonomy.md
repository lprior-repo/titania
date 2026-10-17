# Error Taxonomy — tn-bvv7

No new error types. This bead adds detections, not fallible operations;
every new production function returns `bool` and every helper is total:

- `split_at_checked` `None` arms map to `false` (no match), never panic.
- `saturating_add` replaces length arithmetic (no overflow path).
- `match_indices` with a non-empty needle always yields valid boundaries.
- Test-only `panic!` appears solely inside `unwrap_or_else` message
  builders in integration tests, matching the pre-existing suite style
  (source-target strict clippy does not lint test targets).

Failure mode of the feature itself: a missed capability (false
negative) or a clean-code flag (false positive). Both are pinned by
paired violation/allowed fixtures at content, detector, and lane level.
