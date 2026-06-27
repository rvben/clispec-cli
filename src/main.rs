mod checks;
mod display;
mod help;
mod runner;
mod schema_cmd;
mod scorer;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use std::io::IsTerminal;
use std::process::ExitCode;

/// Output format. `auto` (default) emits JSON when stdout is not a TTY,
/// human-readable on a TTY.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Auto,
    Text,
    Json,
}

impl OutputFormat {
    /// Whether structured JSON should be emitted. `--json` forces it; `auto`
    /// decides by TTY detection on stdout.
    fn is_json(self, json_alias: bool) -> bool {
        if json_alias {
            return true;
        }
        match self {
            OutputFormat::Json => true,
            OutputFormat::Text => false,
            OutputFormat::Auto => !std::io::stdout().is_terminal(),
        }
    }
}

#[derive(Parser)]
#[command(
    name = "clispec",
    version,
    about = "Score CLI tools against The CLI Spec"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format: auto (default), text, or json.
    /// `auto` emits JSON when stdout is not a TTY, human-readable otherwise.
    #[arg(
        long = "output",
        short = 'o',
        global = true,
        value_name = "FORMAT",
        default_value = "auto"
    )]
    output: OutputFormat,

    /// Output as JSON (alias for --output json).
    #[arg(long, global = true, hide = true)]
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

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => return handle_parse_error(e),
    };
    let as_json = cli.output.is_json(cli.json);

    match cli.command {
        Commands::Score { binary, subcommand } => {
            if which::which(&binary).is_err() {
                return error_envelope(
                    "not_found",
                    &format!("'{binary}' not found on PATH"),
                    Some("Provide a binary name on PATH or a path to an executable."),
                    3,
                );
            }
            let result = scorer::score(&binary, &subcommand);
            display::print_score(&result, as_json);
            ExitCode::SUCCESS
        }
        Commands::Schema => {
            schema_cmd::print_schema();
            ExitCode::SUCCESS
        }
        Commands::Completions { shell } => {
            clap_complete::generate(
                shell,
                &mut Cli::command(),
                "clispec",
                &mut std::io::stdout(),
            );
            ExitCode::SUCCESS
        }
    }
}

/// Turn a clap parse failure into either normal help/version output or a
/// structured JSON error envelope on stderr (so agents get a parseable error,
/// not clap's prose). Help and version requests are not errors.
fn handle_parse_error(e: clap::Error) -> ExitCode {
    use clap::error::ErrorKind;
    match e.kind() {
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayVersion
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            // Not a real error: let clap render it. `--help`/`--version` print
            // to stdout and exit 0; a bare invocation with no subcommand prints
            // help to stderr and exits 2. `e.exit()` does the right thing.
            e.exit()
        }
        _ => {
            // clap's error description runs until the first blank line (which
            // separates it from the Usage section). Joining those lines keeps
            // the useful detail — e.g. the missing argument's name, which lives
            // on the line after "the following required arguments...".
            let rendered = e.to_string();
            let message = rendered
                .lines()
                .take_while(|l| !l.trim().is_empty())
                .map(str::trim)
                .collect::<Vec<_>>()
                .join(" ")
                .trim_start_matches("error: ")
                .to_string();
            let message = if message.is_empty() {
                "usage error".to_string()
            } else {
                message
            };
            error_envelope("usage", &message, None, 2)
        }
    }
}

/// Emit a `{"error": {...}}` envelope on stderr and return the matching exit
/// code. The kind/exit-code pairs are declared in `clispec schema`.
fn error_envelope(kind: &str, message: &str, hint: Option<&str>, code: u8) -> ExitCode {
    let mut error = serde_json::json!({ "kind": kind, "message": message });
    if let Some(hint) = hint {
        error["hint"] = serde_json::Value::String(hint.to_string());
    }
    eprintln!("{}", serde_json::json!({ "error": error }));
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_defaults_to_auto() {
        let cli = Cli::try_parse_from(["clispec", "schema"]).unwrap();
        assert_eq!(cli.output, OutputFormat::Auto);
        assert!(!cli.json);
    }

    #[test]
    fn output_parses_text_and_json() {
        let cli = Cli::try_parse_from(["clispec", "-o", "text", "schema"]).unwrap();
        assert_eq!(cli.output, OutputFormat::Text);
        let cli = Cli::try_parse_from(["clispec", "--output", "json", "schema"]).unwrap();
        assert_eq!(cli.output, OutputFormat::Json);
    }

    #[test]
    fn json_flag_is_a_json_alias() {
        let cli = Cli::try_parse_from(["clispec", "--json", "schema"]).unwrap();
        assert!(cli.json);
        assert!(cli.output.is_json(cli.json));
    }

    #[test]
    fn explicit_format_resolves_without_tty() {
        // text/json are deterministic; only auto consults the TTY.
        assert!(OutputFormat::Json.is_json(false));
        assert!(!OutputFormat::Text.is_json(false));
        // --json forces JSON even over an explicit text format.
        assert!(OutputFormat::Text.is_json(true));
    }
}
