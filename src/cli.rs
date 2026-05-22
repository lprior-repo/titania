use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Velvet Ballistics xtask commands")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    #[command(name = "ui-snapshot")]
    Snapshot {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        fixture: Option<String>,
        #[arg(long)]
        emit: Option<String>,
        #[arg(long, default_value = "tests/ui_snapshots")]
        output_dir: String,
    },
    #[command(name = "ui-tokens")]
    Tokens {
        #[arg(long, default_value = "design/tokens/velvet_ui_tokens.toml")]
        input: String,
        #[arg(long, default_value = "crates/vb_ui/src/theme/tokens_generated.rs")]
        output: String,
        #[arg(long)]
        emit: Option<String>,
        #[arg(long)]
        check: bool,
    },
    #[command(name = "ui-overlap-check")]
    OverlapCheck {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        screen: Option<String>,
        #[arg(long, default_value = "tests/ui_snapshots")]
        input_dir: String,
    },
    #[command(name = "ai-fast")]
    AiFast {
        #[arg(long)]
        bead: Option<String>,
    },
    #[command(name = "ai-deep")]
    AiDeep {
        #[arg(long)]
        bead: Option<String>,
    },
    #[command(name = "ai-release")]
    AiRelease {
        #[arg(long)]
        bead: Option<String>,
    },
    #[command(name = "proof-plan")]
    ProofPlan {
        #[arg(long)]
        crate_name: Option<String>,
    },
    #[command(name = "proof-check")]
    ProofCheck {
        #[arg(long)]
        level: Option<String>,
        #[arg(long)]
        bead: Option<String>,
    },
    #[command(name = "proof-evidence")]
    ProofEvidence {
        #[arg(long)]
        bead: String,
    },
    #[command(name = "proof-drift")]
    ProofDrift {
        #[arg(long, value_delimiter = ',')]
        sections: Option<Vec<usize>>,
    },
    #[command(name = "loom")]
    Loom {
        #[arg(long)]
        model: String,
    },
    #[command(name = "forbidden-scan")]
    ForbiddenScan {
        #[arg(long)]
        crates: Option<Vec<String>>,
        #[arg(long)]
        allowlist: Option<String>,
    },
    // === New proof/test orchestrator commands ===
    #[command(name = "list-crates")]
    ListCrates {
        #[arg(long)]
        include: Option<Vec<String>>,
        #[arg(long)]
        exclude: Option<Vec<String>>,
        #[arg(long)]
        json: bool,
    },
    #[command(name = "proof")]
    Proof {
        #[command(subcommand)]
        command: ProofCommands,
    },
    #[command(name = "contracts")]
    Contracts {
        #[arg(long, default_value = "contracts")]
        dir: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        check: bool,
    },
    #[command(name = "assure")]
    Assure {
        #[command(subcommand)]
        command: AssureCommands,
    },
}

#[derive(Subcommand)]
pub(crate) enum ProofCommands {
    #[command(name = "list")]
    List {
        #[arg(long)]
        crate_name: Option<String>,
        #[arg(long)]
        json: bool,
    },
    #[command(name = "run")]
    Run {
        #[arg(long, default_value = "standard")]
        profile: String,
        #[arg(long, default_value = "auto")]
        jobs: String,
        #[arg(long)]
        exclude: Option<Vec<String>>,
        #[arg(long)]
        include: Option<Vec<String>>,
        #[arg(long)]
        fail_fast: bool,
        #[arg(long)]
        keep_going: bool,
        #[arg(long, default_value = "300")]
        timeout: u64,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
    #[command(name = "crate")]
    Crate {
        crate_name: String,
        #[arg(long, value_delimiter = ',')]
        lanes: Option<Vec<String>>,
        #[arg(long, default_value = "auto")]
        jobs: String,
        #[arg(long)]
        fail_fast: bool,
        #[arg(long, default_value = "300")]
        timeout: u64,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
    #[command(name = "affected")]
    Affected {
        #[arg(long)]
        base: String,
        #[arg(long, default_value = "auto")]
        jobs: String,
        #[arg(long)]
        fail_fast: bool,
        #[arg(long, default_value = "300")]
        timeout: u64,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum AssureCommands {
    #[command(name = "contract-lint")]
    ContractLint {
        #[arg(long, default_value = xtask::assure::tenant_access::PILOT_BEAD)]
        bead: String,
        #[arg(long)]
        json: bool,
    },
    #[command(name = "oracle-check")]
    OracleCheck {
        #[arg(long, default_value = xtask::assure::tenant_access::PILOT_BEAD)]
        bead: String,
        #[arg(long)]
        json: bool,
    },
    #[command(name = "path-check")]
    PathCheck {
        #[arg(long, default_value = xtask::assure::tenant_access::PILOT_BEAD)]
        bead: String,
        #[arg(long)]
        json: bool,
    },
}
