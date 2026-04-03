use crate::runner;
use std::time::Duration;

use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let mut checks = Vec::new();

    if !ctx.subcommand.is_empty() {
        let args: Vec<&str> = ctx.subcommand.iter().map(|s| s.as_str()).collect();
        let result = runner::run(&ctx.binary, &args, Duration::from_secs(5));

        // Check 1: stdout is parseable JSON when piped
        let stdout_clean = serde_json::from_str::<serde_json::Value>(&result.stdout).is_ok();
        checks.push(if stdout_clean {
            CheckResult::pass("Clean stdout when piped")
        } else {
            CheckResult::fail("Clean stdout when piped")
        });

        // Check 2: Messages on stderr only (stdout is clean JSON)
        checks.push(if stdout_clean {
            CheckResult::pass("Messages on stderr only")
        } else {
            CheckResult::fail("Messages on stderr only")
        });
    } else {
        checks.push(CheckResult::fail_with(
            "Clean stdout when piped",
            "no subcommand to test",
        ));
        checks.push(CheckResult::fail_with(
            "Messages on stderr only",
            "no subcommand to test",
        ));
    }

    PrincipleScore::new("Stderr/Stdout Separation", checks, 2)
}
