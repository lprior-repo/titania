# Boundary map — tn-f7ti

Pure core (calculations): registry parsing, content stripping and token
matching, per-entry verdicts, required-row checks, discovery filtering.
No I/O, no clock, no process effects.

Actions shell (thin): audit_workspace reads the registry file, lists the
proof source root, and reads candidate files, then delegates everything to
pure audit(). Discovery is flat (one directory level) and extension-gated
(.rs/.vr); recursion limits and non-UTF8 names are documented residuals.

External input crosses the boundary once: registry text becomes Registry,
file bytes become SourceClass. The core never sees paths it must resolve or
bytes it must decode beyond UTF-8 reads that already failed typed.

Out of boundary: Moon orchestration, verifier execution (verus/kani/flux),
disposition record format, and any change to production schema logic.
