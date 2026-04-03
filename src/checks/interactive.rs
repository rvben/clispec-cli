use crate::help;
use crate::runner;
use std::time::Duration;

use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let help_info = help::parse_help(&ctx.help_text);
    let sub_help_info = super::subcommand_help_info(ctx);
    let mut checks = Vec::new();

    // Check 1: No hang without TTY (run with closed stdin, timeout 5s)
    // runner::run already uses Stdio::null() for stdin
    let result = runner::run(&ctx.binary, &["--help"], Duration::from_secs(5));
    checks.push(if result.exit_code >= 0 {
        CheckResult::pass("No TTY hang")
    } else {
        CheckResult::fail("No TTY hang")
    });

    // Check 2: --yes or --force flag (check both top-level and subcommand help)
    let has_yes_or_force = help_info.has_flag("--yes")
        || help_info.has_flag("--force")
        || sub_help_info
            .as_ref()
            .is_some_and(|h| h.has_flag("--yes") || h.has_flag("--force"));
    checks.push(if has_yes_or_force {
        CheckResult::pass("--yes flag")
    } else {
        CheckResult::fail("--yes flag")
    });

    // Check 3: init command exists
    // Check top-level help, then probe `binary init --help` and `binary config init --help`
    let has_init = help_info.has_subcommand("init")
        || help_info.has_subcommand("config init")
        || probe_command_exists(&ctx.binary, &["init", "--help"])
        || probe_command_exists(&ctx.binary, &["config", "init", "--help"]);
    checks.push(if has_init {
        CheckResult::pass("init command")
    } else {
        CheckResult::fail("init command")
    });

    PrincipleScore::new("Non-Interactive", checks, 3)
}

/// Probe whether a command exists by running it and checking for exit code 0.
fn probe_command_exists(binary: &str, args: &[&str]) -> bool {
    let result = runner::run(binary, args, Duration::from_secs(5));
    result.exit_code == 0
}
