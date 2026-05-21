
fn validate_ai_release_bead(bead_id: Option<&str>) -> Result<()> {
    match bead_id {
        Some(VB_NF2U) => Ok(()),
        Some(other) => Err(Error::GateFailed {
            gate: format!("unknown ai-release bead id: {other}"),
            exit_code: 2,
            log: PathBuf::from(".evidence")
                .join(other)
                .join("ai-release.log"),
        }),
        None => Err(Error::GateFailed {
            gate: "missing ai-release bead id".to_string(),
            exit_code: 2,
            log: PathBuf::from(".evidence/default/ai-release.log"),
        }),
    }
}

fn profile_name(profile: GateProfile) -> &'static str {
    match profile {
        GateProfile::AiFast => "ai-fast",
        GateProfile::AiDeep => "ai-deep",
        GateProfile::AiRelease => "ai-release",
    }
}

fn synthetic_gate_evidence(gate: &str, output_dir: &Path) -> GateEvidence {
    GateEvidence {
        kind: gate.to_string(),
        gate_name: gate.to_string(),
        command: format!("synthetic fixture-backed gate: {gate}"),
        exit_code: 0,
        log: output_dir.join(format!("{gate}.log")),
        status: GateStatus::Pass,
        why_failed: None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UiSubgateRun {
    name: &'static str,
    command: &'static str,
    origin: SubgateOrigin,
    status: GateStatus,
    diagnostics: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubgateOrigin {
    SnapshotInventory,
    LayoutPredicates,
    RedactionScan,
    NegativeFixtures,
    DeterministicCapture,
    EvidenceShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckOutcome {
    Passed {
        origin: SubgateOrigin,
    },
    Failed {
        origin: SubgateOrigin,
        diagnostic: &'static str,
    },
}

impl CheckOutcome {
    fn passed(origin: SubgateOrigin) -> Self {
        Self::Passed { origin }
    }

    fn is_passed(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }

    fn origin(&self) -> SubgateOrigin {
        match self {
            Self::Passed { origin } | Self::Failed { origin, .. } => *origin,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UiScreenEvidenceRow {
    screen_id: &'static str,
    fixture_id: &'static str,
    artifact_path: String,
    digest: String,
    provenance: ReadArtifactProvenance,
    checks: Vec<UiCheckEvidenceRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UiCheckEvidenceRow {
    kind: &'static str,
    outcome: CheckOutcome,
    diagnostics: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScreenArtifactFacts {
    screen_id: &'static str,
    provenance: ReadArtifactProvenance,
    timestamp: CaptureTimestamp,
    animation_state: HiddenAnimationState,
    clock_source: ClockSource,
    visible_text: String,
    geometry: ScreenGeometry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArtifactPayload {
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceFixtureSet {
    artifacts: Vec<SourceFixtureArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceFixtureArtifact {
    screen_id: &'static str,
    output_path: PathBuf,
    payload: ArtifactPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReadArtifactProvenance {
    path: PathBuf,
    digest: String,
    payload: ArtifactPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScreenGeometry {
    left: Rect,
    right: Rect,
    container: Rect,
    label: Rect,
    viewport: Rect,
    control: Rect,
    chip: Rect,
    selected_indicator: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HiddenAnimationState {
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClockSource {
    FixedFixtureTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptureTimestamp {
    Fixed(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UiReleaseBundle {
    subgates: Vec<UiSubgateRun>,
    screens: Vec<UiScreenEvidenceRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UiReleaseDocument {
    snapshot_report: String,
    ai_release_report: String,
    negative_fixtures: String,
    determinism: String,
    animation_freeze: String,
}

impl UiReleaseDocument {
    fn from_bundle(bundle: &UiReleaseBundle) -> Result<Self> {
        let document = Self {
            snapshot_report: render_snapshot_report(bundle),
            ai_release_report: render_ai_release_report(bundle),
            negative_fixtures: negative_fixture_report()?,
            determinism: render_determinism_report(),
            animation_freeze: render_animation_freeze_report(),
        };
        document.validate()?;
        Ok(document)
    }

    fn validate(&self) -> Result<()> {
        require_document_shape(self)?;
        scan_redaction_text("release_document", &self.text_tree())
    }

    fn text_tree(&self) -> String {
        format!(
            "{}\n{}\n{}\n{}\n{}",
            self.snapshot_report,
            self.ai_release_report,
            self.negative_fixtures,
            self.determinism,
            self.animation_freeze
        )
    }
}

impl UiReleaseBundle {
    fn from_read_artifacts(snapshot_dir: &Path) -> Result<Self> {
        let screens = build_screen_rows(snapshot_dir)?;
        let subgates = build_subgate_rows(&screens)?;
        let bundle = Self { subgates, screens };
        bundle.validate()?;
        Ok(bundle)
    }

    fn from_source_fixtures(source: &SourceFixtureSet) -> Result<Self> {
        let screens = source.screen_rows()?;
        let subgates = build_subgate_rows(&screens)?;
        let bundle = Self { subgates, screens };
        bundle.validate()?;
        Ok(bundle)
    }

    fn validate(&self) -> Result<()> {
        validate_subgates(&self.subgates)?;
        validate_screen_rows(&self.screens)?;
        Ok(())
    }
}

impl SourceFixtureSet {
    fn read_for_output(snapshot_dir: &Path) -> Result<Self> {
        let artifacts = CANONICAL_SCREENS
            .iter()
            .map(|screen| SourceFixtureArtifact::read(screen, snapshot_dir))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { artifacts })
    }

    fn screen_rows(&self) -> Result<Vec<UiScreenEvidenceRow>> {
        self.artifacts
            .iter()
            .map(SourceFixtureArtifact::screen_row)
            .collect()
    }
}

impl SourceFixtureArtifact {
    fn read(screen: &'static str, snapshot_dir: &Path) -> Result<Self> {
        Ok(Self {
            screen_id: screen,
            output_path: snapshot_dir.join(format!("{screen}.fixture.txt")),
            payload: ArtifactPayload::for_screen(screen)?,
        })
    }

    fn screen_row(&self) -> Result<UiScreenEvidenceRow> {
        let facts = ScreenArtifactFacts::from_source_fixture(self)?;
        let checks = build_check_rows(&facts)?;
        Ok(UiScreenEvidenceRow::from_facts(&facts, checks))
    }
}

fn build_screen_rows(snapshot_dir: &Path) -> Result<Vec<UiScreenEvidenceRow>> {
    CANONICAL_SCREENS
        .iter()
        .map(|screen| screen_evidence_row(screen, snapshot_dir))
        .collect()
}
