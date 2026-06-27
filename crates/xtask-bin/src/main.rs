use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask", version, about = "Deterministic Rust quality gate")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run scoped quality lanes.
    Gate {
        #[arg(long, default_value = "edit")]
        scope: String,
        #[arg(long, default_value = "json")]
        emit: String,
        #[arg(long)]
        out: Option<String>,
    },
    /// Report required tools, versions, and policy health.
    Doctor {
        #[arg(long)]
        scope: Option<String>,
    },
    /// Explain a rule and show accepted repairs.
    Explain { rule_id: String },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Gate { scope, emit, out } => {
            eprintln!("xtask gate --scope {scope} --emit {emit}");
            if let Some(path) = out {
                eprintln!("output: {path}");
            }
            // TODO: implement gate logic
        }
        Command::Doctor { scope } => {
            eprintln!(
                "xtask doctor --scope {}",
                scope.unwrap_or_else(|| "full".to_owned())
            );
            // TODO: implement doctor logic
        }
        Command::Explain { rule_id } => {
            eprintln!("xtask explain {rule_id}");
            // TODO: implement explain logic
        }
    }
}
