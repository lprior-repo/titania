# Domain Model — tn-bvv7

Ubiquitous language: pure core, imperative shell, typed port, effect
capability, designated core path, fully-qualified path, renamed import,
macro-hidden effect, manifest dependency, ambient mutable state, frozen
input, write-once memoization.

Entities: `RuleDef` (id, message, effect, repair, scope, detector),
`Detector` (engine vs string), `RuleScope` (ProductionRust,
CoreDomainRust), `ArchitecturePolicy` (core_dirs, infra_crates),
`CatalogRow` (explain mapping).

Value objects: `WorkspacePath` (validated relative path), `RuleId`
(typed identifier), `Finding` (rule + span + effect + hint).

Aggregates: the embedded rule catalog (YAML documents + `RULES` table
must agree id-for-id; `embedded_rule_ids` proves parity at runtime).

Invariants:
- Every YAML architecture id has exactly one `RULES` entry with the
  same id, message family, Reject effect, and `CoreDomainRust` scope.
- A detector fires only on comment/string-stripped code text.
- `LazyLock` memoization and `include_str!` of frozen input are not
  capabilities and never fire.
- The shell (non-core paths, typed ports) never fires architecture rules.
