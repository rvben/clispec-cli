mod checks;
mod help;
mod runner;

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
            let help_result = runner::run(&binary, &["--help"], std::time::Duration::from_secs(5));
            let help_info = help::parse_help(&help_result.stdout);

            if help_result.exit_code != 0 {
                eprintln!("Warning: --help exited with {}", help_result.exit_code);
                if !help_result.stderr.is_empty() {
                    eprintln!("{}", help_result.stderr);
                }
            }

            let sub = if subcommand.is_empty() {
                help_info
                    .first_list_subcommand()
                    .map(|s| vec![s.to_string()])
                    .unwrap_or_default()
            } else {
                subcommand
            };

            let schema_result =
                runner::run(&binary, &["schema"], std::time::Duration::from_secs(5));
            let schema_json: Option<serde_json::Value> =
                serde_json::from_str(&schema_result.stdout).ok();

            eprintln!(
                "Detected {} flags, {} subcommands",
                help_info.flags.len(),
                help_info.subcommands.len()
            );

            if help_info.has_flag("--json") {
                eprintln!("Tool supports --json");
            }

            let ctx = checks::CheckContext {
                binary,
                subcommand: sub,
                help_text: help_info.raw,
                schema_json,
            };

            let principles = vec![
                checks::output::check(&ctx),
                checks::schema::check(&ctx),
                checks::streams::check(&ctx),
                checks::interactive::check(&ctx),
                checks::idempotent::check(&ctx),
                checks::bounded::check(&ctx),
            ];

            let score: u32 = principles.iter().map(|p| p.score).sum();
            let max: u32 = principles.iter().map(|p| p.max).sum();

            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({
                        "tool": ctx.binary,
                        "score": score,
                        "max": max,
                        "principles": principles,
                    })
                );
            } else {
                eprintln!("Score: {score}/{max}");
            }
        }
        Commands::Schema => todo!(),
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
