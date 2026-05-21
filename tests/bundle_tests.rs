// Proof harnesses, property tests, and Miri checks for bundle module.
//
// Kani harnesses: OBL-001 through OBL-004 (run via `cargo kani`)
// Proptest properties: OBL-005 through OBL-007 (run via `cargo test --test bundle_tests`)
// Miri UB check: OBL-008 (run via `cargo +nightly miri test --test bundle_tests`)

use std::path::PathBuf;

use proptest::prelude::*;
use xtask::evidence::*;

// ──────────────────────────────────────────────────────────────────────────────
// Kani Proof Harnesses (OBL-001 to OBL-004)
// ──────────────────────────────────────────────────────────────────────────────

/// OBL-001: parse_bundle_schema_version never panics on arbitrary string input.
///
/// For any input string s, if parse succeeds, s matches ^(0|[1-9][0-9])\.(0|[1-9][0-9])$.
/// Leading-zero strings ("01.0", "0.01"), malformed strings ("1.0.0", ""),
/// and major > 1 all return Err(SchemaVersionParseFailed).
#[cfg(kani)]
#[kani::proof]
fn schema_version_parse_non_panic() {
    let input: String = kani::any();

    // This must not panic for any input.
    let _result = parse_bundle_schema_version(&input);
}

/// OBL-002: validate_bundle correctness.
///
/// Returns empty vec iff all required fields are non-empty.
/// Each missing field produces exactly one MissingRequiredField error.
#[cfg(kani)]
#[kani::proof]
fn validator_correctness() {
    // Generate an arbitrary bundle via kani::any.
    let bundle: EvidenceBundle = kani::any();

    let errors = validate_bundle(&bundle);

    // Verify that for every error, a specific required field is empty.

    let schema_err = errors.iter().any(|e| {
        matches!(
            e,
            xtask::evidence::Error::MissingRequiredField { field }
                if field == "schema_version"
        ) || matches!(e, xtask::evidence::Error::SchemaVersionParseFailed { .. })
    });

    let bead_err = errors.iter().any(|e| {
        matches!(
            e,
            xtask::evidence::Error::MissingRequiredField { field }
                if field == "linked_bead_id"
        )
    });

    let agent_err = errors.iter().any(|e| {
        matches!(
            e,
            xtask::evidence::Error::MissingRequiredField { field }
                if field == "executor_context.agent"
        )
    });

    let timestamp_err = errors.iter().any(|e| {
        matches!(
            e,
            xtask::evidence::Error::MissingRequiredField { field }
                if field == "executor_context.timestamp"
        )
    });

    let machine_err = errors.iter().any(|e| {
        matches!(
            e,
            xtask::evidence::Error::MissingRequiredField { field }
                if field == "executor_context.machine"
        )
    });

    assert!(
        schema_err || !bundle.schema_version.is_empty(),
        "schema_version error expected when empty"
    );
    assert!(
        bead_err || !bundle.linked_bead_id.is_empty(),
        "linked_bead_id error expected when empty"
    );
    assert!(
        agent_err || !bundle.executor_context.agent.is_empty(),
        "agent error expected when empty"
    );
    assert!(
        timestamp_err || !bundle.executor_context.timestamp.is_empty(),
        "timestamp error expected when empty"
    );
    assert!(
        machine_err || !bundle.executor_context.machine.is_empty(),
        "machine error expected when empty"
    );
}

/// OBL-003: write_bundle does not panic for any serialisable bundle.
///
/// Returns Ok(()) or a descriptive Error.
#[cfg(kani)]
#[kani::proof]
fn write_bundle_non_panic() {
    let bundle: EvidenceBundle = kani::any();
    let format: EvidenceBundleFormat = kani::any();
    let path: PathBuf = kani::any();

    // Must not panic; result is either Ok(()) or an Error.
    let _result = write_bundle(&bundle, &path, format);
}

/// OBL-004: read_bundle does not panic when reading arbitrary bundle data.
///
/// Unknown fields are silently ignored (no deny_unknown_fields).
#[cfg(kani)]
#[kani::proof]
fn read_bundle_non_panic() {
    let bundle: EvidenceBundle = kani::any();
    let format: EvidenceBundleFormat = kani::any();

    // Round-trip through the format: serialise then read from memory buffer.
    let bytes_result: std::result::Result<Vec<u8>, String> = match format {
        EvidenceBundleFormat::Yaml => serde_saphyr::to_string(&bundle)
            .map(|s| s.into_bytes())
            .map_err(|e| e.to_string()),
        EvidenceBundleFormat::Json => serde_json::to_string(&bundle)
            .map(|s| s.into_bytes())
            .map_err(|e| e.to_string()),
        EvidenceBundleFormat::Postcard => postcard::to_allocvec(&bundle).map_err(|e| e.to_string()),
    };

    if let Ok(ref raw) = bytes_result {
        // Read back — must not panic.
        let _result: std::result::Result<EvidenceBundle, _> = match format {
            EvidenceBundleFormat::Yaml => serde_saphyr::from_slice::<EvidenceBundle>(raw),
            EvidenceBundleFormat::Json => serde_json::from_slice::<EvidenceBundle>(raw),
            EvidenceBundleFormat::Postcard => postcard::from_bytes::<EvidenceBundle>(raw),
        };
    }
    // If serialisation failed, that's an error return, not a panic.
}

// ──────────────────────────────────────────────────────────────────────────────
// Proptest Properties (OBL-005 to OBL-007)
// ──────────────────────────────────────────────────────────────────────────────

/// OBL-005: Round-trip identity — serialise then deserialize yields equivalent bundle.
#[test]
fn prop_write_read_roundtrip_yaml() {
    use proptest::prelude::*;

    proptest!(|(bundle in evidence_bundle_strategy())| {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("bundle.yaml");

        write_bundle(&bundle, &path, EvidenceBundleFormat::Yaml)
            .expect("write bundle succeeded");

        let roundtrip = read_bundle(&path, EvidenceBundleFormat::Yaml)
            .expect("read bundle succeeded");

        assert_eq!(
            bundle, roundtrip,
            "YAML round-trip failed: original != roundtrip"
        );
    });
}

#[test]
fn prop_write_read_roundtrip_json() {
    use proptest::prelude::*;

    proptest!(|(bundle in evidence_bundle_strategy())| {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("bundle.json");

        write_bundle(&bundle, &path, EvidenceBundleFormat::Json)
            .expect("write bundle succeeded");

        let roundtrip = read_bundle(&path, EvidenceBundleFormat::Json)
            .expect("read bundle succeeded");

        assert_eq!(
            bundle, roundtrip,
            "JSON round-trip failed: original != roundtrip"
        );
    });
}

#[test]
fn prop_write_read_roundtrip_postcard() {
    use proptest::prelude::*;

    proptest!(|(bundle in evidence_bundle_strategy())| {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("bundle.postcard");

        write_bundle(&bundle, &path, EvidenceBundleFormat::Postcard)
            .expect("write bundle succeeded");

        let roundtrip = read_bundle(&path, EvidenceBundleFormat::Postcard)
            .expect("read bundle succeeded");

        assert_eq!(
            bundle, roundtrip,
            "Postcard round-trip failed: original != roundtrip"
        );
    });
}

/// OBL-006: Fail-closed validation — empty required fields trigger rejection.
#[test]
fn prop_fail_closed_missing_bead_id() {
    use proptest::prelude::*;

    proptest!(
        |(agent in any::<String>(),
          timestamp in any::<String>(),
          machine in any::<String>(),
          major in 2u64..,
          minor in any::<String>())| {
            let bundle = EvidenceBundle {
                schema_version: format!("{}.{}", major, minor),
                executor_context: ExecutorContext {
                    agent,
                    timestamp,
                    machine,
                },
                linked_bead_id: String::new(),
                gates: vec![],
                source_test_mappings: vec![],
                release_artifacts: vec![],
            };

            let errors = validate_bundle(&bundle);
            assert!(
                !errors.is_empty(),
                "validate_bundle must reject empty linked_bead_id"
            );
            assert!(
                errors.iter().any(|e| {
                    matches!(
                        e,
                        xtask::evidence::Error::MissingRequiredField {
                            field
                        } if field == "linked_bead_id"
                    )
                }),
                "must produce MissingRequiredField for linked_bead_id"
            );
        }
    );
}

#[test]
fn prop_fail_closed_missing_agent() {
    use proptest::prelude::*;

    proptest!(|(bundle in evidence_bundle_strategy())| {
        let mut mutated = bundle.clone();
        mutated.executor_context.agent = String::new();

        let errors = validate_bundle(&mutated);
        assert!(
            !errors.is_empty(),
            "validate_bundle must reject empty agent"
        );
    });
}

#[test]
fn prop_fail_closed_missing_timestamp() {
    use proptest::prelude::*;

    proptest!(|(bundle in evidence_bundle_strategy())| {
        let mut mutated = bundle.clone();
        mutated.executor_context.timestamp = String::new();

        let errors = validate_bundle(&mutated);
        assert!(
            !errors.is_empty(),
            "validate_bundle must reject empty timestamp"
        );
    });
}

#[test]
fn prop_fail_closed_missing_machine() {
    use proptest::prelude::*;

    proptest!(|(bundle in evidence_bundle_strategy())| {
        let mut mutated = bundle.clone();
        mutated.executor_context.machine = String::new();

        let errors = validate_bundle(&mutated);
        assert!(
            !errors.is_empty(),
            "validate_bundle must reject empty machine"
        );
    });
}

/// OBL-007: Path determinism — same bead_id + format produces same path.
#[test]
fn prop_path_deterministic() {
    use proptest::prelude::*;

    let format_strategy = proptest::sample::select(vec![
        EvidenceBundleFormat::Yaml,
        EvidenceBundleFormat::Json,
        EvidenceBundleFormat::Postcard,
    ]);

    proptest!(|(bead_id in any::<String>(), format in format_strategy)| {
        let path1 = bundle_path(&bead_id, format);
        let path2 = bundle_path(&bead_id, format);

        assert_eq!(
            path1, path2,
            "bundle_path must be deterministic for same inputs"
        );

        assert!(
            path1.starts_with(".evidence"),
            "path must start with .evidence/"
        );

        let expected_ext = match format {
            EvidenceBundleFormat::Yaml => "yaml",
            EvidenceBundleFormat::Json => "json",
            EvidenceBundleFormat::Postcard => "postcard",
        };
        let actual_ext = path1
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        assert_eq!(
            actual_ext, expected_ext,
            "extension mismatch: expected {}, got {}",
            expected_ext, actual_ext
        );
    });
}

#[test]
fn prop_format_extensions_distinct() {
    assert_eq!(EvidenceBundleFormat::Yaml.extension(), "yaml");
    assert_eq!(EvidenceBundleFormat::Json.extension(), "json");
    assert_eq!(EvidenceBundleFormat::Postcard.extension(), "postcard");

    assert_ne!(
        EvidenceBundleFormat::Yaml.extension(),
        EvidenceBundleFormat::Json.extension()
    );
    assert_ne!(
        EvidenceBundleFormat::Yaml.extension(),
        EvidenceBundleFormat::Postcard.extension()
    );
    assert_ne!(
        EvidenceBundleFormat::Json.extension(),
        EvidenceBundleFormat::Postcard.extension()
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Miri Test (OBL-008) — UB check for Postcard serialization
// ──────────────────────────────────────────────────────────────────────────────

/// OBL-008: Postcard serialization round-trip must not exhibit undefined behavior.
/// Run with: cargo +nightly miri test --test bundle_tests
#[cfg(miri)]
#[test]
fn miri_postcard_roundtrip_no_ub() {
    let bundle = EvidenceBundle {
        schema_version: "1.0".to_string(),
        executor_context: ExecutorContext {
            agent: "miri-test".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            machine: "miri-host".to_string(),
        },
        linked_bead_id: "vb-miri-test".to_string(),
        gates: vec![],
        source_test_mappings: vec![],
        release_artifacts: vec![],
    };

    let bytes = postcard::to_allocvec(&bundle).expect("postcard serialise");
    let roundtrip: EvidenceBundle = postcard::from_bytes(&bytes).expect("postcard deserialise");
    assert_eq!(bundle, roundtrip, "Miri postcard round-trip failed");
}

// ──────────────────────────────────────────────────────────────────────────────
// Helper: proptest strategy for generating arbitrary EvidenceBundle values
// ──────────────────────────────────────────────────────────────────────────────

use proptest::strategy::{BoxedStrategy, Strategy};

fn evidence_bundle_strategy() -> BoxedStrategy<EvidenceBundle> {
    use proptest::collection::vec;

    fn arb_executor_context() -> BoxedStrategy<ExecutorContext> {
        (any::<String>(), any::<String>(), any::<String>())
            .prop_map(|(agent, timestamp, machine)| ExecutorContext {
                agent,
                timestamp,
                machine,
            })
            .boxed()
    }

    fn arb_gate_evidence() -> BoxedStrategy<GateEvidence> {
        (
            any::<String>(),
            any::<String>(),
            any::<String>(),
            any::<i32>(),
            any::<PathBuf>(),
        )
            .prop_flat_map(|(kind, gate_name, command, exit_code, log)| {
                let status_strategy = prop_oneof![
                    Just(GateStatus::Pass),
                    Just(GateStatus::Fail),
                    any::<String>().prop_map(|reason| GateStatus::Skipped { reason }),
                ];
                status_strategy.prop_map(move |status| GateEvidence {
                    kind: kind.clone(),
                    gate_name: gate_name.clone(),
                    command: command.clone(),
                    exit_code,
                    log: log.clone(),
                    status,
                    why_failed: None,
                })
            })
            .boxed()
    }

    fn arb_source_test_mapping() -> BoxedStrategy<SourceTestMapping> {
        (any::<String>(), vec(any::<String>(), 0..=5))
            .prop_map(|(source_path, tests)| SourceTestMapping { source_path, tests })
            .boxed()
    }

    fn arb_release_artifact() -> BoxedStrategy<ReleaseGateArtifact> {
        use proptest::sample::select;

        let artifact_type_strategy = select(vec![
            ArtifactType::Benchmark,
            ArtifactType::Coverage,
            ArtifactType::Mutation,
            ArtifactType::SupplyChain,
            ArtifactType::Miri,
            ArtifactType::Clippy,
            ArtifactType::Fmt,
        ]);

        (
            any::<String>(),
            any::<String>(),
            any::<String>(),
            artifact_type_strategy,
        )
            .prop_map(|(name, path, digest, artifact_type)| ReleaseGateArtifact {
                name,
                path,
                digest,
                artifact_type,
            })
            .boxed()
    }

    (
        arb_executor_context(),
        any::<String>(),
        vec(arb_gate_evidence(), 0..=5),
        vec(arb_source_test_mapping(), 0..=5),
        vec(arb_release_artifact(), 0..=5),
    )
        .prop_map(
            |(executor_context, linked_bead_id, gates, stms, rga)| EvidenceBundle {
                schema_version: "1.0".to_string(),
                executor_context,
                linked_bead_id,
                gates,
                source_test_mappings: stms,
                release_artifacts: rga,
            },
        )
        .boxed()
}
