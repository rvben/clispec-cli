use clap::CommandFactory;
use serde_json::{Value, json};

use crate::Cli;

pub fn print_schema() {
    let cmd = Cli::command();
    let schema = json!({
        "clispec": "0.3",
        "name": "clispec",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Score CLI tools against The CLI Spec",
        "output": {"tty": "text", "piped": "json"},
        "global_args": [
            {"name": "--output", "type": "string", "required": false, "default": "auto",
             "short": "-o",
             "enum": ["auto", "text", "json"],
             "description": "Output format. auto emits JSON when stdout is not a TTY, human-readable otherwise."},
            {"name": "--json", "type": "boolean", "required": false,
             "description": "Alias for --output json."}
        ],
        "commands": walk_commands(&cmd),
        "errors": [
            {"kind": "usage", "exit_code": 2, "retryable": false,
             "description": "Invalid arguments, unrecognized command, or bad flag value."},
            {"kind": "not_found", "exit_code": 3, "retryable": false,
             "description": "The binary to score was not found on PATH."},
        ]
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&schema).expect("serialize")
    );
}

fn walk_commands(cmd: &clap::Command) -> Vec<Value> {
    cmd.get_subcommands()
        .filter(|c| c.get_name() != "help")
        .map(|c| {
            let args: Vec<Value> = c
                .get_arguments()
                .filter(|a| !["help", "version", "json", "output"].contains(&a.get_id().as_str()))
                .map(|a| {
                    json!({
                        "name": a.get_long().map(|l| format!("--{l}")).unwrap_or_else(|| a.get_id().to_string()),
                        "type": arg_type(a),
                        "required": a.is_required_set(),
                    })
                })
                .collect();

            let mut entry = json!({
                "name": c.get_name(),
                "description": c.get_about().map(|s| s.to_string()).unwrap_or_default(),
                "effects": "read_only",
                "mutating": false,
            });
            if !args.is_empty() {
                entry["args"] = json!(args);
            }
            match c.get_name() {
                "completions" => {
                    entry["output_kind"] = json!("opaque");
                    entry["media_type"] = json!("text/plain");
                }
                "schema" => {
                    entry["cardinality"] = json!("single");
                    entry["stdout_schema"] =
                        json!({"$ref": "https://clispec.dev/schema/v0.3.json"});
                }
                name => {
                    entry["cardinality"] = json!("single");
                    if let Some(fields) = output_fields_for(name) {
                        entry["output_fields"] = fields;
                    } else {
                        entry["stdout_schema"] = json!({});
                    }
                }
            }
            if c.get_name() == "score" {
                entry["example"] = json!({"args": ["score", "echo"]});
            }
            entry
        })
        .collect()
}

fn output_fields_for(command: &str) -> Option<Value> {
    match command {
        "score" => Some(json!([
            {"name": "tool", "type": "string"},
            {"name": "path", "type": "string"},
            {"name": "score", "type": "integer"},
            {"name": "max", "type": "integer"},
            {"name": "percentage", "type": "integer"},
            {"name": "grade", "type": "string",
             "description": "Excellent | Good | Fair | Needs Work"},
            {"name": "principles", "type": "array", "items": {"type": "object"},
             "description": "Per-principle scores with per-check breakdown"}
        ])),
        _ => None,
    }
}

fn arg_type(arg: &clap::Arg) -> &'static str {
    use clap::ArgAction;
    match arg.get_action() {
        ArgAction::SetTrue | ArgAction::SetFalse => "boolean",
        ArgAction::Count => "integer",
        ArgAction::Append => "string[]",
        _ => "string",
    }
}
