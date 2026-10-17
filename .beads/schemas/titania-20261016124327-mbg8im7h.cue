
package validation

import "list"

// Validation schema for bead: titania-20261016124327-mbg8im7h
// Title: external: bind Velvet Ballistics Verus proofs to production bodies
//
// This schema validates that implementation is complete.
// Use: cue vet titania-20261016124327-mbg8im7h.cue implementation.cue

#BeadImplementation: {
  bead_id: "titania-20261016124327-mbg8im7h"
  title: "external: bind Velvet Ballistics Verus proofs to production bodies"

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
      "record the actual Velvet Ballistics owning issue and target revision before any external implementation claim",
      "distinguish verified production bodies from source-linked copied models in the external handoff",
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
      "Given access to the actual VB target, When handed off, Then its real owning bead and revision are linked.",
      "Given verified real production behavior, When reviewed there, Then raw verifier evidence names the exact implementation.",
    ]

    // Required error path tests
    required_error_tests: [
      "Given no target provenance or inaccessible repo, When this task is reviewed, Then it remains blocked rather than claiming patch integration.",
      "Given an unchanged model beside changed production logic, When VB proof integrity runs, Then the production claim is rejected.",
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
//     timestamp: "2026-10-16T12:43:27Z"
//   }
// }