mod checks;
mod display;
mod help;
mod runner;
mod schema_cmd;
mod scorer;

use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "clispec",
    version,
    about = "Score CLI tools against The CLI Spec"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Score a CLI tool against The CLI Spec
    Score {
        /// Binary name or path to score
        binary: String,
        /// Subcommand to test with (optional)
        subcommand: Vec<String>,
    },
    /// Output JSON schema for agent integration
    Schema,
    /// Generate shell completions
    Completions { shell: clap_complete::Shell },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Score { binary, subcommand } => {
            if which::which(&binary).is_err() {
                let error = serde_json::json!({
                    "error": {
                        "kind": "not_found",
                        "message": format!("'{binary}' not found on PATH"),
                        "hint": "Provide a binary name on PATH or a path to an executable."
                    }
                });
                eprintln!("{error}");
                std::process::exit(3);
            }
            let result = scorer::score(&binary, &subcommand);
            display::print_score(&result, cli.json);
        }
        Commands::Schema => schema_cmd::print_schema(),
        Commands::Completions { shell } => {
            clap_complete::generate(
                shell,
                &mut Cli::command(),
                "clispec",
                &mut std::io::stdout(),
            );
        }
    }
}
