pub mod bounded;
pub mod idempotent;
pub mod interactive;
pub mod output;
pub mod schema;
pub mod streams;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub detail: Option<String>,
}

impl CheckResult {
    pub fn pass(name: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: true,
            detail: None,
        }
    }

    pub fn fail(name: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: false,
            detail: None,
        }
    }

    pub fn fail_with(name: &str, detail: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: false,
            detail: Some(detail.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PrincipleScore {
    pub name: String,
    pub score: u32,
    pub max: u32,
    pub checks: Vec<CheckResult>,
}

impl PrincipleScore {
    pub fn new(name: &str, checks: Vec<CheckResult>, max: u32) -> Self {
        let score = checks.iter().filter(|c| c.passed).count() as u32;
        Self {
            name: name.to_string(),
            score,
            max,
            checks,
        }
    }
}

pub struct CheckContext {
    pub binary: String,
    pub subcommand: Vec<String>,
    pub help_text: String,
    pub schema_json: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_context() -> CheckContext {
        CheckContext {
            binary: "echo".to_string(),
            subcommand: vec![],
            help_text: String::new(),
            schema_json: None,
        }
    }

    #[test]
    fn check_result_constructors() {
        let pass = CheckResult::pass("test");
        assert!(pass.passed);
        assert!(pass.detail.is_none());

        let fail = CheckResult::fail("test");
        assert!(!fail.passed);

        let fail_detail = CheckResult::fail_with("test", "reason");
        assert!(!fail_detail.passed);
        assert_eq!(fail_detail.detail.as_deref(), Some("reason"));
    }

    #[test]
    fn principle_score_counts_passes() {
        let checks = vec![
            CheckResult::pass("a"),
            CheckResult::fail("b"),
            CheckResult::pass("c"),
        ];
        let score = PrincipleScore::new("test", checks, 3);
        assert_eq!(score.score, 2);
        assert_eq!(score.max, 3);
    }

    #[test]
    fn all_stubs_return_zero_score() {
        let ctx = test_context();
        assert_eq!(output::check(&ctx).score, 0);
        assert_eq!(schema::check(&ctx).score, 0);
        assert_eq!(streams::check(&ctx).score, 0);
        assert_eq!(interactive::check(&ctx).score, 0);
        assert_eq!(idempotent::check(&ctx).score, 0);
        assert_eq!(bounded::check(&ctx).score, 0);
    }
}
