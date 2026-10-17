# Regression Diff — tn-bvv7

Touched production behavior (all intended, all pinned by tests):

- Four legacy architecture detectors now also fire on
  fully-qualified paths, renamed imports were already caught and stay
  caught, macro bodies are scanned. Previously silent core effects now
  emit named Reject findings. No previously firing input goes silent
  (full pre-existing lane suites pass unmodified except the
  intentionally extended matrix test).
- Three new Reject rules (PROCESS, ASYNC, STATE) fire only on
  designated core paths; shell code is unaffected (scope filter
  unchanged).
- Repair catalog gains 3 rows (count 80 → 83); `for_rule` mapping
  extended for the async `ReplaceDependency` row.
- v1-spec rule table gains 3 rows plus the fully-qualified paragraph.

No API, CLI, lane-outcome schema, or Moon task changes. `git status`
shows only the files listed in `codebase-map.md` plus bead artifacts.
