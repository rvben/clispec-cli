use clap::CommandFactory;
use serde_json::{Value, json};

use crate::Cli;

pub fn print_schema() {
    let cmd = Cli::command();
    let schema = json!({
        "name": "clispec",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Score CLI tools against The CLI Spec",
        "commands": walk_commands(&cmd),
        "errors": [
            {"kind": "not_found", "retryable": false, "description": "Binary not found on PATH"},
            {"kind": "timeout", "retryable": true, "description": "Binary execution timed out"},
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
                        "required": a.is_required_set(),
                    })
                })
                .collect();

            let mut entry = json!({
                "name": c.get_name(),
                "description": c.get_about().map(|s| s.to_string()).unwrap_or_default(),
            });
            if !args.is_empty() {
                entry["args"] = json!(args);
            }
            entry
        })
        .collect()
}
