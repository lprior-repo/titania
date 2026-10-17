
package validation

import "list"

// Validation schema for bead: titania-20261016124326-wnmw8wlj
// Title: policy: reject discarded must-use values and futures
//
// This schema validates that implementation is complete.
// Use: cue vet titania-20261016124326-wnmw8wlj.cue implementation.cue

#BeadImplementation: {
  bead_id: "titania-20261016124326-wnmw8wlj"
  title: "policy: reject discarded must-use values and futures"

  // Contract verification
  contracts_verified: {
    preconditions_checked: bool & true
    postconditions_verified: bool & true
    invariants_maintained: bool & true

    // Specific preconditions that must be verified
    precondition_checks: [
      "Read current source and any canonical bead-local contract before editing.",
      "Respect dependency edges and keep changes within the stated control.",
    ]

    // Specific postconditions that must be verified
    postcondition_checks: [
      "deny discarded must-use results and forgotten futures at the judged target boundary",
      "reject missing or weakened required lint configuration rather than only hardening Titania itself",
    ]

    // Specific invariants that must be maintained
    invariant_checks: [
      "Moon is the only CI/CD orchestrator; no GitHub Actions restoration.",
      "No source linkage, trust declaration, listing, coverage percentage, or cover! result is represented as proof of production execution.",
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
      "Given Result handling with ? or exhaustive typed match, When target checks run, Then no discarded-value finding occurs.",
      "Given an awaited future or explicitly owned spawned task with handled result, When checked, Then this disposition control accepts it.",
    ]

    // Required error path tests
    required_error_tests: [
      "Given let _ = fallible_operation(), When target checks run, Then a named discarded-result finding blocks the gate.",
      "Given a constructed future discarded by let _ = or bare statement, When checked, Then the forgotten-future control rejects it.",
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
//     timestamp: "2026-10-16T12:43:26Z"
//   }
// }