
package validation

import "list"

// Validation schema for bead: titania-20260701160808-grmgcfkq
// Title: docs: align README and public docs with v1 contract
//
// This schema validates that implementation is complete.
// Use: cue vet titania-20260701160808-grmgcfkq.cue implementation.cue

#BeadImplementation: {
  bead_id: "titania-20260701160808-grmgcfkq"
  title: "docs: align README and public docs with v1 contract"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "The canonical v1-spec.md sections named by this bead are readable in the repository.",
      "The current Cargo workspace can be inspected before source edits are planned.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "The named source or configuration paths contain the behavior described by this bead.",
      "The bead evidence names every command run and every command that is blocked by a missing tool.",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "All new Rust production code remains unsafe-free and panic-free under workspace lints.",
      "All externally consumed JSON shapes are serde round-tripped by tests before implementation is closed.",
    ]
  }

  // Test verification
  tests_passing: {
    all_tests_pass: bool & true

    happy_path_tests: [...string] & list.MinItems(2)
    error_path_tests: [...string] & list.MinItems(2)

    // Note: Actual test names provided by implementer, must include all required tests

    // Required happy path tests
    required_happy_tests: [
      "README quickstart uses titania-check --scope edit and titania-check doctor",
      "README does not advertise `titania init`, `titania ci --scope full`, or vb-fmt-0012 for v1",
    ]

    // Required error path tests
    required_error_tests: [
      "Docs do not claim v1 proves panic-freedom or functional correctness",
      "Docs do not list deferred tools as v1 blockers",
    ]
  }

  // Code completion
  code_complete: {
    implementation_exists: string  // Path to implementation file
    tests_exist: string  // Path to test file
    ci_passing: bool & true
    no_unwrap_calls: bool & true  // Rust/functional constraint
    no_panics: bool & true  // Rust constraint
  }

  // Completion criteria
  completion: {
    all_sections_complete: bool & true
    documentation_updated: bool
    beads_closed: bool
    timestamp: string  // ISO8601 completion timestamp
  }
}

// Example implementation proof - create this file to validate completion:
//
// implementation.cue:
// package validation
//
// implementation: #BeadImplementation & {
//   contracts_verified: {
//     preconditions_checked: true
//     postconditions_verified: true
//     invariants_maintained: true
//     precondition_checks: [/* documented checks */]
//     postcondition_checks: [/* documented verifications */]
//     invariant_checks: [/* documented invariants */]
//   }
//   tests_passing: {
//     all_tests_pass: true
//     happy_path_tests: ["test_version_flag_works", "test_version_format", "test_exit_code_zero"]
//     error_path_tests: ["test_invalid_flag_errors", "test_no_flags_normal_behavior"]
//   }
//   code_complete: {
//     implementation_exists: "src/main.rs"
//     tests_exist: "tests/cli_test.rs"
//     ci_passing: true
//     no_unwrap_calls: true
//     no_panics: true
//   }
//   completion: {
//     all_sections_complete: true
//     documentation_updated: true
//     beads_closed: false
//     timestamp: "2026-07-01T16:08:08Z"
//   }
// }