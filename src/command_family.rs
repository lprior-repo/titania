#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandFamily {
    AiContext,
    AiPlan,
    AiCheck,
    AiEvidence,
    Invariants,
    Scans,
    CertCheck,
    Perf,
    Replay,
    Crash,
    Diff,
    Mutants,
    Loom,
    Kani,
    Fuzz,
    Prop,
    Repro,
    TestPlan,
    Review,
    WhyFailed,
}

impl CommandFamily {
    #[must_use]
    pub fn public_name(self) -> &'static str {
        match self {
            Self::AiContext => "ai-context",
            Self::AiPlan => "ai-plan",
            Self::AiCheck => "ai-check",
            Self::AiEvidence => "ai-evidence",
            Self::Invariants => "invariants",
            Self::Scans => "scans",
            Self::CertCheck => "cert-check",
            Self::Perf => "perf",
            Self::Replay => "replay",
            Self::Crash => "crash",
            Self::Diff => "diff",
            Self::Mutants => "mutants",
            Self::Loom => "loom",
            Self::Kani => "kani",
            Self::Fuzz => "fuzz",
            Self::Prop => "prop",
            Self::Repro => "repro",
            Self::TestPlan => "test-plan",
            Self::Review => "review",
            Self::WhyFailed => "why-failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "ai-context" => Some(Self::AiContext),
            "ai-plan" => Some(Self::AiPlan),
            "ai-check" => Some(Self::AiCheck),
            "ai-evidence" => Some(Self::AiEvidence),
            "invariants" => Some(Self::Invariants),
            "scans" => Some(Self::Scans),
            "cert-check" => Some(Self::CertCheck),
            "perf" => Some(Self::Perf),
            "replay" => Some(Self::Replay),
            "crash" => Some(Self::Crash),
            "diff" => Some(Self::Diff),
            "mutants" => Some(Self::Mutants),
            "loom" => Some(Self::Loom),
            "kani" => Some(Self::Kani),
            "fuzz" => Some(Self::Fuzz),
            "prop" => Some(Self::Prop),
            "repro" => Some(Self::Repro),
            "test-plan" => Some(Self::TestPlan),
            "review" => Some(Self::Review),
            "why-failed" => Some(Self::WhyFailed),
            _ => None,
        }
    }
}
