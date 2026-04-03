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
        let mut fallback: Option<Vec<String>> = None;

        let entries: Vec<(String, &serde_json::Value)> = if let Some(obj) = commands.as_object() {
            obj.iter().map(|(k, v)| (k.clone(), v)).collect()
        } else if let Some(arr) = commands.as_array() {
            arr.iter()
                .filter_map(|v| {
                    v.get("name")
                        .and_then(|n| n.as_str())
                        .map(|n| (n.to_string(), v))
                })
                .collect()
        } else {
            vec![]
        };

        for (name, cmd) in &entries {
            let is_mutating = cmd
                .get("mutating")
                .and_then(|m| m.as_bool())
                .unwrap_or(false);
            if !is_mutating {
                let parts: Vec<String> = name.split_whitespace().map(|s| s.to_string()).collect();
                // Prefer simple "noun list" commands (2 parts)
                if let Some(last) = parts.last()
                    && (last == "list" || last == "ls")
                    && parts.len() == 2
                {
                    return parts;
                }
                if fallback.is_none() {
                    fallback = Some(parts);
                }
            }
        }

        if let Some(fb) = fallback {
            return fb;
        }
    }

    // Try top-level help for direct list/status commands
    let help_info = help::parse_help(help_text);
    if let Some(sub) = help_info.first_list_subcommand() {
        return vec![sub.to_string()];
    }

    // Try nested subcommands — look for "noun list" patterns
    // Extract candidate nouns from help (words that appear as subcommands)
    let nouns: Vec<&str> = help_text
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            // Lines that look like "  noun   Description" in help
            if trimmed.starts_with(char::is_alphabetic) {
                trimmed.split_whitespace().next()
            } else {
                None
            }
        })
        .collect();

    for noun in &nouns {
        for verb in &["list", "ls", "status"] {
            let result = runner::run(binary, &[noun, verb, "--help"], Duration::from_secs(3));
            if result.exit_code == 0 {
                return vec![noun.to_string(), verb.to_string()];
            }
        }
    }

    vec![]
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
    let percentage = if max > 0 {
        (total_score * 100) / max
    } else {
        0
    };

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
