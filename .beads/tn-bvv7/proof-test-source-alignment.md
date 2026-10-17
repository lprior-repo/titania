# Alignment — tn-bvv7

| Requirement | Proof ID | Refinement ID | Source Refs | Behavior Test Refs | Refinement Harness Refs | Commands Run | Ledger Result | Status |
|-------------|----------|---------------|-------------|--------------------|-------------------------|--------------|---------------|--------|
| REQ-3 | O1 | R1 | detectors.rs::detect_core_fs_import et al. | ast_grep_architecture + ast_grep_lane FQ tests | tn_bvv7_fq_bypass_property | cargo test -p titania-lanes --lib tn_bvv7_property; cargo test -p titania-lanes --test ast_grep_architecture --test ast_grep_lane | PASS | closed |
| REQ-1 | O2 | R2 | detectors.rs::detect_core_process_import et al. | ast_grep_lane new-boundaries test; manifest test; clean-controls test | tn_bvv7_effect_boundary_property | cargo test -p titania-lanes --lib tn_bvv7_property; cargo test suites | PASS | closed |
| REQ-2 | none (happy-path contract) | none | filters.rs scope | scope tests; allowed fixtures; lane clean-controls | clean-padding controls | cargo test -p titania-lanes | PASS | closed |
| REQ-4 | O1 (macro) + contract test | R1 | detectors.rs::detect_core_fs_import; titania-core/Cargo.toml | macro fixture test; manifest test | tn_bvv7_fq_bypass_property | cargo test suites | PASS | closed |
