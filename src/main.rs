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
        Commands::Score {
            binary,
            subcommand: _,
        } => {
            eprintln!("Scoring {binary}...");
            todo!()
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
