use clap::CommandFactory;
use serde_json::{Value, json};

use crate::Cli;

pub fn print_schema() {
    let cmd = Cli::command();
    let schema = json!({
        "clispec": "0.2",
        "name": "clispec",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Score CLI tools against The CLI Spec",
        "global_args": [
            {"name": "--json", "type": "boolean", "required": false,
             "description": "Output as JSON"}
        ],
        "commands": walk_commands(&cmd),
        "errors": [
            {"kind": "not_found", "exit_code": 3, "retryable": false,
             "description": "Binary not found on PATH"},
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
                .filter(|a| !["help", "version", "json"].contains(&a.get_id().as_str()))
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
                "mutating": false,
            });
            if !args.is_empty() {
                entry["args"] = json!(args);
            }
            if let Some(fields) = output_fields_for(c.get_name()) {
                entry["output_fields"] = fields;
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
            {"name": "principles", "type": "object[]",
             "description": "Per-principle scores with per-check breakdown"}
        ])),
        "schema" => Some(json!([
            {"name": "clispec", "type": "string",
             "description": "Schema version string, e.g. \"0.2\"."},
            {"name": "name", "type": "string",
             "description": "Binary name of the tool."},
            {"name": "version", "type": "string",
             "description": "Semver version of the tool."},
            {"name": "description", "type": "string",
             "description": "One-line description of the tool."},
            {"name": "global_args", "type": "object[]",
             "description": "Flags that apply to every command (e.g. --json)."},
            {"name": "commands", "type": "object[]",
             "description": "Array of command descriptors, each with name, description, mutating, args, and output_fields."},
            {"name": "errors", "type": "object[]",
             "description": "Structured error kinds with kind, exit_code, retryable, and description."}
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
