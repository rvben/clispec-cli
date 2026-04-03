use crate::help;
use crate::runner;
use std::time::Duration;

use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let help_info = help::parse_help(&ctx.help_text);
    let mut checks = Vec::new();

    // Check 1: No hang without TTY (run with closed stdin, timeout 5s)
    // runner::run already uses Stdio::null() for stdin
    let result = runner::run(&ctx.binary, &["--help"], Duration::from_secs(5));
    checks.push(if result.exit_code >= 0 {
        CheckResult::pass("No TTY hang")
    } else {
        CheckResult::fail("No TTY hang")
    });

    // Check 2: --yes or --force flag
    checks.push(
        if help_info.has_flag("--yes") || help_info.has_flag("--force") {
            CheckResult::pass("--yes flag")
        } else {
            CheckResult::fail("--yes flag")
        },
    );

    // Check 3: init command exists
    checks.push(
        if help_info.has_subcommand("init") || help_info.has_subcommand("config init") {
            CheckResult::pass("init command")
        } else {
            CheckResult::fail("init command")
        },
    );

    PrincipleScore::new("Non-Interactive", checks, 3)
}
