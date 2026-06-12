use serde::Serialize;

use crate::checks::{self, CheckContext, PrincipleScore};
use crate::help;
use crate::runner;
use std::time::Duration;

#[derive(Debug, Serialize)]
pub struct Score {
    pub tool: String,
    pub path: String,
    pub score: u32,
    pub max: u32,
    pub percentage: u32,
    pub grade: String,
    pub principles: Vec<PrincipleScore>,
}

impl Score {
    fn grade_for(percentage: u32) -> String {
        match percentage {
            90..=100 => "Excellent".to_string(),
            70..=89 => "Good".to_string(),
            50..=69 => "Fair".to_string(),
            _ => "Needs Work".to_string(),
        }
    }
}

fn discover_subcommand(
    binary: &str,
    help_text: &str,
    schema_json: &Option<serde_json::Value>,
) -> Vec<String> {
    // Try schema first — prefer a non-mutating "list" command, fall back to any non-mutating
    if let Some(schema) = schema_json
        && let Some(commands) = schema.get("commands")
    {
        let entries = flatten_commands(commands);

        // Explicitly read-only commands (mutating == false) are safe to
        // probe. Absent mutating means unknown in v0.2, so those are only a
        // last resort for older schemas that relied on the v0.1 default.
        let mut fallback_explicit: Option<Vec<String>> = None;
        let mut fallback_unknown: Option<Vec<String>> = None;

        for (name, cmd) in &entries {
            let mutating = cmd.get("mutating").and_then(|m| m.as_bool());
            if mutating == Some(true) {
                continue;
            }
            let parts: Vec<String> = name.split_whitespace().map(|s| s.to_string()).collect();
            // Prefer simple "noun list" commands (2 parts)
            let is_list = parts.len() == 2
                && parts
                    .last()
                    .is_some_and(|last| last == "list" || last == "ls");
            match mutating {
                Some(false) => {
                    if is_list {
                        return parts;
                    }
                    if fallback_explicit.is_none() {
                        fallback_explicit = Some(parts);
                    }
                }
                _ => {
                    if fallback_unknown.is_none() {
                        fallback_unknown = Some(parts);
                    }
                }
            }
        }

        if let Some(fb) = fallback_explicit.or(fallback_unknown) {
            return fb;
        }
    }

    // Try top-level help for a direct list/status command
    let help_info = help::parse_help(help_text);
    let listed = &help_info.listed_subcommands;
    for verb in ["list", "ls", "status", "info", "get", "show"] {
        if listed.iter().any(|s| s == verb) {
            return vec![verb.to_string()];
        }
    }
    if listed.is_empty()
        && let Some(sub) = help_info.first_list_subcommand()
    {
        return vec![sub.to_string()];
    }

    // Try nested subcommands — probe each noun from the Commands section
    // for a "noun list" style command
    const NOT_NOUNS: &[&str] = &[
        "help",
        "version",
        "schema",
        "completion",
        "completions",
        "init",
        "login",
        "logout",
    ];
    for noun in listed.iter().filter(|n| !NOT_NOUNS.contains(&n.as_str())) {
        for verb in &["list", "ls", "status"] {
            let result = runner::run(binary, &[noun, verb, "--help"], Duration::from_secs(3));
            if result.exit_code == 0 {
                return vec![noun.to_string(), verb.to_string()];
            }
        }
    }

    vec![]
}

/// Flatten a schema `commands` value into (path, command) pairs for every
/// LEAF command, recursing through nested `subcommands` arrays and building
/// space-joined paths ("apps list"). Container commands are excluded:
/// probing one only prints its help text. Object-form schemas (path-keyed
/// maps) are already flat and pass through as-is.
fn flatten_commands(commands: &serde_json::Value) -> Vec<(String, &serde_json::Value)> {
    fn walk<'a>(
        prefix: &str,
        cmd: &'a serde_json::Value,
        out: &mut Vec<(String, &'a serde_json::Value)>,
    ) {
        let Some(name) = cmd.get("name").and_then(|n| n.as_str()) else {
            return;
        };
        let path = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix} {name}")
        };
        if let Some(subs) = cmd.get("subcommands").and_then(|s| s.as_array())
            && !subs.is_empty()
        {
            for sub in subs {
                walk(&path, sub, out);
            }
            return;
        }
        out.push((path, cmd));
    }

    let mut out = Vec::new();
    match commands {
        serde_json::Value::Array(arr) => {
            for cmd in arr {
                walk("", cmd, &mut out);
            }
        }
        serde_json::Value::Object(obj) => {
            out.extend(obj.iter().map(|(k, v)| (k.clone(), v)));
        }
        _ => {}
    }
    out
}

pub fn score(binary: &str, subcommand: &[String]) -> Score {
    let path = which::which(binary)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| binary.to_string());

    let help_result = runner::run(binary, &["--help"], Duration::from_secs(5));
    let help_text = if help_result.exit_code == 0 {
        help_result.stdout.clone()
    } else {
        help_result.stderr.clone()
    };

    let schema_result = runner::run(binary, &["schema"], Duration::from_secs(5));
    let schema_json: Option<serde_json::Value> = serde_json::from_str(&schema_result.stdout).ok();

    let subcommand = if subcommand.is_empty() {
        discover_subcommand(binary, &help_text, &schema_json)
    } else {
        subcommand.to_vec()
    };

    let ctx = CheckContext {
        binary: binary.to_string(),
        subcommand,
        help_text,
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

    let total_score: u32 = principles.iter().map(|p| p.score).sum();
    let max: u32 = principles.iter().map(|p| p.max).sum();
    let percentage = (total_score * 100).checked_div(max).unwrap_or(0);

    Score {
        tool: binary.to_string(),
        path,
        score: total_score,
        max,
        percentage,
        grade: Score::grade_for(percentage),
        principles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_commands_recurses_into_nested_subcommands() {
        let commands = serde_json::json!([
            {"name": "apps", "mutating": false, "subcommands": [
                {"name": "list", "mutating": false},
                {"name": "delete", "mutating": true}
            ]},
            {"name": "deploy", "mutating": true}
        ]);
        let paths: Vec<String> = flatten_commands(&commands)
            .into_iter()
            .map(|(p, _)| p)
            .collect();
        assert_eq!(paths, vec!["apps list", "apps delete", "deploy"]);
    }

    #[test]
    fn flatten_commands_excludes_containers() {
        let commands = serde_json::json!([
            {"name": "apps", "subcommands": [{"name": "list"}]}
        ]);
        let entries = flatten_commands(&commands);
        assert!(
            entries.iter().all(|(p, _)| p != "apps"),
            "container command must not be a probe candidate: {entries:?}"
        );
    }

    #[test]
    fn flatten_commands_passes_object_form_through() {
        let commands = serde_json::json!({
            "apps list": {"mutating": false},
            "deploy": {"mutating": true}
        });
        let mut paths: Vec<String> = flatten_commands(&commands)
            .into_iter()
            .map(|(p, _)| p)
            .collect();
        paths.sort();
        assert_eq!(paths, vec!["apps list", "deploy"]);
    }

    #[test]
    fn discover_prefers_nested_read_only_list_command() {
        let schema = serde_json::json!({
            "name": "mytool",
            "version": "1.0.0",
            "commands": [
                {"name": "deploy", "mutating": true},
                {"name": "apps", "subcommands": [
                    {"name": "delete", "mutating": true},
                    {"name": "list", "mutating": false}
                ]}
            ]
        });
        let found = discover_subcommand("/nonexistent-binary", "", &Some(schema));
        assert_eq!(found, vec!["apps".to_string(), "list".to_string()]);
    }

    #[test]
    fn discover_prefers_explicitly_read_only_over_unknown_mutating() {
        // v0.2: absent mutating means unknown, so a command that states
        // mutating=false outranks one that says nothing.
        let schema = serde_json::json!({
            "name": "mytool",
            "version": "1.0.0",
            "commands": [
                {"name": "mystery"},
                {"name": "status", "mutating": false}
            ]
        });
        let found = discover_subcommand("/nonexistent-binary", "", &Some(schema));
        assert_eq!(found, vec!["status".to_string()]);
    }
}
