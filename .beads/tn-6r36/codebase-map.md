# Codebase Map — tn-6r36

Contract bead; files read (no edits):

- v1-spec.md (enforcement chapters), README.md, clippy.toml (ban tables)
- Cargo.toml (workspace lint floor)
- crates/titania-policy/src/lib.rs + exceptions.rs (approval authority shape)
- crates/titania-lanes/src/policy_scan/cargo_lints.rs (manifest weakening floor)
- crates/titania-dylint/src/lib.rs + functional_surface.rs (current
  syntax-level enforcement baseline)

Planned implementation targets (do not exist yet; owned by follow-ups):

- crates/titania-lanes/src/policy_scan/classifier.rs (tn-ebn9/tn-8obr)
- crates/titania-dylint/src/functional_surface.rs resolved-identity pass (tn-pfp1)
- policy_scan/cargo_lints.rs must-use/future floors (tn-8qfc)
