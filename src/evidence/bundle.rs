// Evidence bundle types, writer, reader, and validator.
//
// Provides a self-contained serialisable document that aggregates gate execution
// evidence with metadata about the execution context, source/test mappings, release
// artifacts, and bead linkage.

// Note: Error, GateEvidence, Path, PathBuf, Serialize, Deserialize are all in scope
// from the preceding include!() directives in evidence.rs — no imports needed here.

// ── Bundle Container ─────────────────────────────────────────────────────────

/// Top-level evidence bundle container.
///
/// Self-contained: all required fields must be present.
/// Rejected by the validator if any required field is missing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceBundle {
    /// Schema version in major.minor form, e.g. "1.0".
    pub schema_version: String,

    /// Who/what ran the gates (agent name, timestamp, machine).
    pub executor_context: ExecutorContext,

    /// The bead that produced this bundle.
    pub linked_bead_id: String,

    /// Gate execution evidence records. May be empty (staging bundles).
    pub gates: Vec<GateEvidence>,

    /// Source file -> test name coverage mappings. May be empty.
    pub source_test_mappings: Vec<SourceTestMapping>,

    /// Release-gate artifact metadata. May be empty.
    pub release_artifacts: Vec<ReleaseGateArtifact>,
}

// ── Executor Context ─────────────────────────────────────────────────────────

/// Metadata about the execution that produced the bundle.
///
/// All three sub-fields are required.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ExecutorContext {
    /// Agent name or process name that ran the gates.
    pub agent: String,
    /// ISO-8601 UTC timestamp of execution.
    pub timestamp: String,
    /// Machine hostname or CI runner identifier.
    pub machine: String,
}

// ── Source/Test Mappings ─────────────────────────────────────────────────────

/// Maps a single source file path to the test names that cover it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SourceTestMapping {
    /// Source file path relative to workspace root.
    pub source_path: String,
    /// Test names (harness or function) that exercise this source file.
    pub tests: Vec<String>,
}

// ── Release Gate Artifacts ────────────────────────────────────────────────────

/// Metadata for a release-gate artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReleaseGateArtifact {
    /// Human-readable artifact name.
    pub name: String,
    /// File path or URI where the artifact is stored.
    pub path: String,
    /// Content digest with algorithm prefix, e.g. "sha256:a1b2c3d4...".
    pub digest: String,
    /// Artifact type discriminator (field serialised as "type").
    #[serde(rename = "type")]
    pub artifact_type: ArtifactType,
}

/// Discriminator for release-gate artifact kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    Benchmark,
    Coverage,
    Mutation,
    SupplyChain,
    Miri,
    Clippy,
    Fmt,
}

// ── Serialization Format ─────────────────────────────────────────────────────

/// Serialization format for evidence bundle output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceBundleFormat {
    /// Human-readable YAML (via serde-saphyr).
    Yaml,
    /// Machine-readable JSON (via serde_json).
    Json,
    /// Binary, compact Postcard.
    Postcard,
}

impl EvidenceBundleFormat {
    /// File extension for this format.
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Yaml => "yaml",
            Self::Json => "json",
            Self::Postcard => "postcard",
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Construct the bundle file path for a given bead and format.
///
/// Path is `.evidence/<bead-id>/bundle.<ext>`.
pub fn bundle_path(bead_id: &str, format: EvidenceBundleFormat) -> PathBuf {
    PathBuf::from(".evidence")
        .join(bundle_path_component(bead_id))
        .join(format!("bundle.{}", format.extension()))
}

fn bundle_path_component(bead_id: &str) -> String {
    let component: String = bead_id
        .chars()
        .map(|ch| match ch {
            '/' | '\\' => '_',
            other => other,
        })
        .collect();

    if component.is_empty() {
        String::from("unknown")
    } else {
        component
    }
}

/// Parse a bundle schema version in major.minor form.
///
/// Format: "major.minor" where both parts are non-negative integers
/// without leading zeros (except "0" itself).
///
/// Returns the original string on success.
pub fn parse_bundle_schema_version(s: &str) -> std::result::Result<String, Error> {
    if s.is_empty() {
        return Err(Error::SchemaVersionParseFailed {
            version: s.to_string(),
        });
    }

    let parts: Vec<&str> = s.splitn(2, '.').collect();
    // safety: splitn(2) returns at most 2 parts; length check below guards indexing
    #[allow(clippy::indexing_slicing)]
    if parts.len() != 2
        || parts[0].is_empty()
        || parts[1].is_empty()
    {
        return Err(Error::SchemaVersionParseFailed {
            version: s.to_string(),
        });
    }

    #[allow(clippy::indexing_slicing)]
    let (major_s, minor_s) = (parts[0], parts[1]);

    // Reject leading zeros: "0" is OK, but "00", "01", etc. are not.
    let reject_leading_zero = |s: &str| -> bool {
        s.len() > 1 && s.starts_with('0')
    };

    if reject_leading_zero(major_s) || reject_leading_zero(minor_s) {
        return Err(Error::SchemaVersionParseFailed {
            version: s.to_string(),
        });
    }

    let major: u64 = match major_s.parse() {
        Ok(v) => v,
        Err(_) => {
            return Err(Error::SchemaVersionParseFailed {
                version: s.to_string(),
            });
        }
    };
    let _minor: u64 = match minor_s.parse() {
        Ok(v) => v,
        Err(_) => {
            return Err(Error::SchemaVersionParseFailed {
                version: s.to_string(),
            });
        }
    };

    // Reject major > 1 (consumer supports only major version 0 and 1).
    if major > 1 {
        return Err(Error::SchemaVersionParseFailed {
            version: s.to_string(),
        });
    }

    Ok(s.to_string())
}

/// Validate a deserialised bundle's required fields.
///
/// Returns an empty vec if the bundle is valid.
/// Returns one `Error::MissingRequiredField` per absent required field.
pub fn validate_bundle(bundle: &EvidenceBundle) -> Vec<Error> {
    let mut errors: Vec<Error> = Vec::new();

    // Schema version: non-empty and parseable.
    if bundle.schema_version.is_empty()
        || parse_bundle_schema_version(&bundle.schema_version).is_err()
    {
        if bundle.schema_version.is_empty() {
            errors.push(Error::MissingRequiredField {
                field: "schema_version".to_string(),
            });
        } else {
            errors.push(Error::SchemaVersionParseFailed {
                version: bundle.schema_version.clone(),
            });
        }
    }

    // Linked bead ID: non-empty.
    if bundle.linked_bead_id.is_empty() {
        errors.push(Error::MissingRequiredField {
            field: "linked_bead_id".to_string(),
        });
    }

    // Executor context: all three sub-fields non-empty.
    if bundle.executor_context.agent.is_empty() {
        errors.push(Error::MissingRequiredField {
            field: "executor_context.agent".to_string(),
        });
    }
    if bundle.executor_context.timestamp.is_empty() {
        errors.push(Error::MissingRequiredField {
            field: "executor_context.timestamp".to_string(),
        });
    }
    if bundle.executor_context.machine.is_empty() {
        errors.push(Error::MissingRequiredField {
            field: "executor_context.machine".to_string(),
        });
    }

    errors
}

/// Serialise and write an `EvidenceBundle` to disk.
///
/// Creates parent directories if they do not exist.
pub fn write_bundle(
    bundle: &EvidenceBundle,
    path: &Path,
    format: EvidenceBundleFormat,
) -> std::result::Result<(), Error> {
    // Create parent directories.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Error::BeadDirectoryCreationFailed {
            bead: path
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string(),
            cause: e.to_string(),
        })?;
    }

    let formatted_format = format_to_string(format);

    let serialised: std::result::Result<Vec<u8>, Error> = match format {
        EvidenceBundleFormat::Yaml => {
            let yaml = serde_json::to_string_pretty(bundle).map_err(|e| {
                Error::BundleSerializationFailed {
                    format: formatted_format.clone(),
                    cause: e.to_string(),
                }
            })?;
            Ok(yaml.into_bytes())
        }
        EvidenceBundleFormat::Json => {
            let json = serde_json::to_string(bundle).map_err(|e| {
                Error::BundleSerializationFailed {
                    format: formatted_format.clone(),
                    cause: e.to_string(),
                }
            })?;
            Ok(json.into_bytes())
        }
        EvidenceBundleFormat::Postcard => {
            let wire = EvidenceBundlePostcard::from_bundle(bundle);
            let bytes = postcard::to_allocvec(&wire).map_err(|e| {
                Error::BundleSerializationFailed {
                    format: formatted_format.clone(),
                    cause: e.to_string(),
                }
            })?;
            Ok(bytes)
        }
    };

    let bytes = serialised?;
    std::fs::write(path, &bytes).map_err(|e| Error::EvidenceWriteFailed {
        gate: "bundle".to_string(),
        path: path.to_path_buf(),
        cause: e.to_string(),
    })
}

/// Deserialise an `EvidenceBundle` from a file.
pub fn read_bundle(
    path: &Path,
    format: EvidenceBundleFormat,
) -> std::result::Result<EvidenceBundle, Error> {
    let contents = std::fs::read(path).map_err(|e| Error::EvidenceWriteFailed {
        gate: "bundle".to_string(),
        path: path.to_path_buf(),
        cause: e.to_string(),
    })?;

    match format {
        EvidenceBundleFormat::Yaml => {
            let bundle: EvidenceBundle = serde_saphyr::from_slice(&contents).map_err(|e| {
                Error::BundleSerializationFailed {
                    format: "yaml".to_string(),
                    cause: e.to_string(),
                }
            })?;
            Ok(bundle)
        }
        EvidenceBundleFormat::Json => {
            let bundle: EvidenceBundle = serde_json::from_slice(&contents).map_err(|e| {
                Error::BundleSerializationFailed {
                    format: "json".to_string(),
                    cause: e.to_string(),
                }
            })?;
            Ok(bundle)
        }
        EvidenceBundleFormat::Postcard => {
            let wire: EvidenceBundlePostcard = postcard::from_bytes(&contents).map_err(|e| {
                Error::BundleSerializationFailed {
                    format: "postcard".to_string(),
                    cause: e.to_string(),
                }
            })?;
            Ok(wire.into_bundle())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct EvidenceBundlePostcard {
    schema_version: String,
    executor_context: ExecutorContext,
    linked_bead_id: String,
    gates: Vec<GateEvidencePostcard>,
    source_test_mappings: Vec<SourceTestMapping>,
    release_artifacts: Vec<ReleaseGateArtifact>,
}

impl EvidenceBundlePostcard {
    fn from_bundle(bundle: &EvidenceBundle) -> Self {
        Self {
            schema_version: bundle.schema_version.clone(),
            executor_context: bundle.executor_context.clone(),
            linked_bead_id: bundle.linked_bead_id.clone(),
            gates: bundle
                .gates
                .iter()
                .map(GateEvidencePostcard::from_gate)
                .collect(),
            source_test_mappings: bundle.source_test_mappings.clone(),
            release_artifacts: bundle.release_artifacts.clone(),
        }
    }

    fn into_bundle(self) -> EvidenceBundle {
        EvidenceBundle {
            schema_version: self.schema_version,
            executor_context: self.executor_context,
            linked_bead_id: self.linked_bead_id,
            gates: self
                .gates
                .into_iter()
                .map(GateEvidencePostcard::into_gate)
                .collect(),
            source_test_mappings: self.source_test_mappings,
            release_artifacts: self.release_artifacts,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GateEvidencePostcard {
    kind: String,
    gate_name: String,
    command: String,
    exit_code: i32,
    log: String,
    status: GateStatusPostcard,
    why_failed: Option<WhyFailed>,
}

impl GateEvidencePostcard {
    fn from_gate(gate: &GateEvidence) -> Self {
        Self {
            kind: gate.kind.clone(),
            gate_name: gate.gate_name.clone(),
            command: gate.command.clone(),
            exit_code: gate.exit_code,
            log: gate.log.to_string_lossy().into_owned(),
            status: GateStatusPostcard::from_status(&gate.status),
            why_failed: gate.why_failed.clone(),
        }
    }

    fn into_gate(self) -> GateEvidence {
        GateEvidence {
            kind: self.kind,
            gate_name: self.gate_name,
            command: self.command,
            exit_code: self.exit_code,
            log: PathBuf::from(self.log),
            status: self.status.into_status(),
            why_failed: self.why_failed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GateStatusPostcard {
    tag: u8,
    reason: Option<String>,
}

impl GateStatusPostcard {
    fn from_status(status: &GateStatus) -> Self {
        match status {
            GateStatus::Pass => Self {
                tag: 0,
                reason: None,
            },
            GateStatus::Fail => Self {
                tag: 1,
                reason: None,
            },
            GateStatus::Skipped { reason } => Self {
                tag: 2,
                reason: Some(reason.clone()),
            },
        }
    }

    fn into_status(self) -> GateStatus {
        match (self.tag, self.reason) {
            (0, _) => GateStatus::Pass,
            (1, _) => GateStatus::Fail,
            (2, Some(reason)) => GateStatus::Skipped { reason },
            (2, None) => GateStatus::Skipped {
                reason: String::new(),
            },
            (_, _) => GateStatus::Fail,
        }
    }
}

/// Convert format to a display string.
fn format_to_string(format: EvidenceBundleFormat) -> String {
    match format {
        EvidenceBundleFormat::Yaml => "yaml".to_string(),
        EvidenceBundleFormat::Json => "json".to_string(),
        EvidenceBundleFormat::Postcard => "postcard".to_string(),
    }
}
