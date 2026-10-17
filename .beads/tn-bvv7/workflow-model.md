# Workflow Model — tn-bvv7

1. Source file enters `ast_grep_lane::scan_path` with its workspace path.
2. `rule_applies` gates on Rust source, non-ignored production path,
   non-meta path, and `CoreDomainRust` scope (designated core paths only).
3. The string detector scans comment/string-stripped code for `use`
   imports (direct, grouped, multiline, renamed), fully-qualified paths,
   and macro bodies.
4. On match, the lane emits a typed `Finding` (Reject effect + repair
   hint) with a span; on no match across all rules, a clean outcome.
5. `titania-core` manifest dependencies are pinned by contract test
   (`ast_grep_architecture_core_manifest_has_no_effect_dependencies`),
   closing the dependency bypass outside the lane.

Shell path: file reads, lane execution, report emission, and process
spawning stay in shell crates behind typed ports; core computes over
explicit inputs only.
