
impl RawSubgateDocument {
    fn into_subgate(self) -> std::result::Result<UiSubgateName, String> {
        if self.status != "passed" || self.command != "cargo xtask ai-release --bead vb-nf2u" {
            return Err("invalid subgate row".to_string());
        }
        if !self.diagnostics.is_empty()
            || self.origin.is_empty()
            || self.execution_marker.is_empty()
        {
            return Err("invalid subgate diagnostics/origin".to_string());
        }
        UiSubgateName::parse(self.name)
    }
}

impl RawRedactionDocument {
    fn into_screens(self) -> std::result::Result<Vec<ParsedRedactionScreen>, String> {
        if self.status != "passed" || self.checked_artifacts.is_empty() {
            return Err("invalid redaction document".to_string());
        }
        self.screens
            .into_iter()
            .map(ParsedRedactionScreen::try_from)
            .collect()
    }
}

impl TryFrom<RawRedactionScreen> for ParsedRedactionScreen {
    type Error = String;

    fn try_from(raw: RawRedactionScreen) -> std::result::Result<Self, Self::Error> {
        if raw.status != "passed" || !raw.diagnostics.is_empty() || raw.execution_marker.is_empty()
        {
            return Err("invalid redaction screen".to_string());
        }
        Ok(Self {
            screen_id: CanonicalScreenId::parse(raw.screen_id)?,
            classes: raw.class_coverage.into_classes()?,
        })
    }
}

impl RawClassCoverage {
    fn into_classes(self) -> std::result::Result<Vec<RedactionClass>, String> {
        [
            ("sentinel", self.sentinel),
            ("api_key", self.api_key),
            ("token", self.token),
            ("password", self.password),
            ("idempotency_key", self.idempotency_key),
            ("tainted_fixture_value", self.tainted_fixture_value),
        ]
        .into_iter()
        .map(|(name, evidence)| evidence.into_class(name))
        .collect()
    }
}

impl RawClassEvidence {
    fn into_class(self, name: &str) -> std::result::Result<RedactionClass, String> {
        if self.detectors == 0 || self.raw_matches != 0 || self.approved_placeholders_seen == 0 {
            return Err(format!("invalid redaction evidence for {name}"));
        }
        let class = RedactionClass::parse(name)?;
        if self.placeholder != format!("[REDACTED:{}]", class.as_str()) {
            return Err(format!("invalid redaction placeholder for {name}"));
        }
        Ok(class)
    }
}

impl TryFrom<RawNegativeFixtureDocument> for ParsedNegativeFixtureDocument {
    type Error = String;

    fn try_from(raw: RawNegativeFixtureDocument) -> std::result::Result<Self, Self::Error> {
        validate_raw_contract_audit(raw.contract_audit)?;
        let mut overlap = None;
        let mut secret = None;
        for entry in raw.negative_fixtures {
            match entry.fixture_id.as_str() {
                "intentional_overlap_fixture" if overlap.is_none() => {
                    overlap = Some(entry.into_overlap()?)
                }
                "intentional_secret_fixture" if secret.is_none() => {
                    secret = Some(entry.into_secret()?)
                }
                _ => return Err("duplicate or unknown negative fixture entry".to_string()),
            }
        }
        Ok(Self {
            overlap: overlap.ok_or_else(|| "missing overlap fixture entry".to_string())?,
            secret: secret.ok_or_else(|| "missing secret fixture entry".to_string())?,
        })
    }
}

fn validate_raw_contract_audit(raw: RawContractAudit) -> std::result::Result<(), String> {
    require_fixture_backed(raw.fixture_backed)?;
    require_unsupported_parity(raw.core_runtime_parity_claim)?;
    if raw.false_pass_detectors.len() == 2 {
        Ok(())
    } else {
        Err("invalid negative fixture contract audit".to_string())
    }
}

impl RawNegativeFixtureEntry {
    fn into_overlap(self) -> std::result::Result<ParsedOverlapFixtureEvidence, String> {
        if self.status == "rejected" {
            return self.into_rejected_overlap();
        }
        self.into_expected_overlap()
    }

    fn into_expected_overlap(self) -> std::result::Result<ParsedOverlapFixtureEvidence, String> {
        let evidence = ParsedOverlapExpectedFailure {
            fixture_id: FixtureId::parse(self.fixture_id, "overlap fixture_id")?,
            status: FixtureStatus::parse_expected(self.status, "overlap status")?,
            gate: parse_required_gate(self.gate, FixtureGate::LayoutReadability, "overlap gate")?,
            diagnostic_code: parse_required_code(
                self.diagnostic_code,
                DiagnosticCode::Layout,
                "overlap diagnostic",
            )?,
            screen_id: parse_required_screen(self.screen_id, "overlap screen_id")?,
            control_id: parse_required_control(self.control_id, "overlap control_id")?,
            second_control_id: parse_required_control(
                self.second_control_id,
                "overlap second_control_id",
            )?,
            overlap_area_px: parse_required_overlap_area(self.overlap_area_px)?,
            bounds: parse_required_bounds(self.bounds, "overlap bounds")?,
            predicate: parse_overlap_predicate(self.predicate)?,
            fixture_nonce: parse_optional_nonce(self.fixture_nonce)?,
        };
        Ok(ParsedOverlapFixtureEvidence::ExpectedFailed(evidence))
    }

    fn into_rejected_overlap(self) -> std::result::Result<ParsedOverlapFixtureEvidence, String> {
        let rejected = ParsedRejectedFixtureEvidence::Overlap(self.into_overlap_rejection()?);
        Ok(ParsedOverlapFixtureEvidence::Rejected(rejected))
    }

    fn into_secret(self) -> std::result::Result<ParsedSecretFixtureEvidence, String> {
        if self.status == "rejected" {
            return self.into_rejected_secret();
        }
        self.into_expected_secret()
    }

    fn into_expected_secret(self) -> std::result::Result<ParsedSecretFixtureEvidence, String> {
        let evidence = ParsedSecretExpectedFailure {
            fixture_id: FixtureId::parse(self.fixture_id, "secret fixture_id")?,
            status: FixtureStatus::parse_expected(self.status, "secret status")?,
            gate: parse_required_gate(self.gate, FixtureGate::Redaction, "secret gate")?,
            diagnostic_code: parse_required_code(
                self.diagnostic_code,
                DiagnosticCode::Redaction,
                "secret diagnostic",
            )?,
            screen_id: parse_required_screen(self.screen_id, "secret screen_id")?,
            secret_class: RedactionClass::parse(&require_some(self.secret_class, "secret class")?)?,
            redacted_sample: parse_required_redacted_sample(self.redacted_sample)?,
            fixture_nonce: parse_optional_nonce(self.fixture_nonce)?,
        };
        Ok(ParsedSecretFixtureEvidence::ExpectedFailed(evidence))
    }

    fn into_rejected_secret(self) -> std::result::Result<ParsedSecretFixtureEvidence, String> {
        let rejected = ParsedRejectedFixtureEvidence::Secret(self.into_secret_rejection()?);
        Ok(ParsedSecretFixtureEvidence::Rejected(rejected))
    }

    fn into_overlap_rejection(self) -> std::result::Result<ParsedOverlapRejection, String> {
        validate_false_pass_variant(self.variant, self.code)?;
        Ok(ParsedOverlapRejection {
            fixture_id: FixtureId::parse(self.fixture_id, "rejected fixture_id")?,
            status: FixtureStatus::parse_rejected(self.status, "rejected status")?,
            error: parse_required_error(self.error)?,
            expected_gate: parse_required_gate(self.expected_gate, FixtureGate::Layout, "gate")?,
            actual_status: parse_required_passed(self.actual_status)?,
            action: self.action,
            fixture_nonce: parse_optional_nonce(self.fixture_nonce)?,
        })
    }

    fn into_secret_rejection(self) -> std::result::Result<ParsedSecretRejection, String> {
        validate_false_pass_variant(self.variant, self.code)?;
        Ok(ParsedSecretRejection {
            fixture_id: FixtureId::parse(self.fixture_id, "rejected fixture_id")?,
            status: FixtureStatus::parse_rejected(self.status, "rejected status")?,
            error: parse_required_error(self.error)?,
            expected_gate: parse_required_gate(self.expected_gate, FixtureGate::Redaction, "gate")?,
            actual_status: parse_required_passed(self.actual_status)?,
            action: self.action,
            fixture_nonce: parse_optional_nonce(self.fixture_nonce)?,
        })
    }
}

fn require_some(value: Option<String>, name: &str) -> std::result::Result<String, String> {
    value.ok_or_else(|| format!("missing {name}"))
}

fn require_text(value: String, expected: &str, name: &str) -> std::result::Result<String, String> {
    if value == expected {
        Ok(value)
    } else {
        Err(format!("invalid {name}: {value}"))
    }
}

impl FixtureId {
    fn parse(value: String, name: &str) -> std::result::Result<Self, String> {
        parse_nonempty_text(value, name).map(Self)
    }
}

impl ControlId {
    fn parse(value: String, name: &str) -> std::result::Result<Self, String> {
        parse_nonempty_text(value, name).map(Self)
    }
}

impl FixtureBounds {
    fn parse(value: String, name: &str) -> std::result::Result<Self, String> {
        parse_nonempty_text(value, name).map(Self)
    }
}

impl FixtureNonce {
    fn parse(value: String) -> std::result::Result<Self, String> {
        parse_nonempty_text(value, "fixture_nonce").map(Self)
    }
}

impl RedactedSample {
    fn parse(value: String) -> std::result::Result<Self, String> {
        if value.starts_with("[REDACTED:") && value.ends_with(']') {
            Ok(Self(value))
        } else {
            Err("invalid redacted sample".to_string())
        }
    }
}

impl NonzeroOverlapArea {
    fn parse(value: String) -> std::result::Result<Self, String> {
        match value.parse::<u32>() {
            Ok(area) if area > 0 => Ok(Self(area)),
            _ => Err(format!("invalid nonzero overlap area: {value}")),
        }
    }
}

impl FixtureStatus {
    fn parse_expected(value: String, name: &str) -> std::result::Result<Self, String> {
        require_text(value, Self::ExpectedFailed.as_str(), name).map(|_| Self::ExpectedFailed)
    }

    fn parse_rejected(value: String, name: &str) -> std::result::Result<Self, String> {
        require_text(value, Self::Rejected.as_str(), name).map(|_| Self::Rejected)
    }
}

fn parse_nonempty_text(value: String, name: &str) -> std::result::Result<String, String> {
    if value.is_empty() {
        Err(format!("empty {name}"))
    } else {
        Ok(value)
    }
}

fn parse_required_screen(
    value: Option<String>,
    name: &str,
) -> std::result::Result<CanonicalScreenId, String> {
    CanonicalScreenId::parse(require_some(value, name)?)
}
