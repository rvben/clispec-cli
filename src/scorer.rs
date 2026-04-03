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
        let help_info = help::parse_help(&help_text);
        help_info
            .first_list_subcommand()
            .map(|s| vec![s.to_string()])
            .unwrap_or_default()
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
