use crate::help;
use crate::runner;
use std::time::Duration;

use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let help_info = help::parse_help(&ctx.help_text);
    let mut checks = Vec::new();

    // Check 1: --json flag in help
    checks.push(if help_info.has_flag("--json") {
        CheckResult::pass("--json flag")
    } else {
        CheckResult::fail("--json flag")
    });

    // Check 2: Valid JSON with --json
    if !ctx.subcommand.is_empty() {
        let mut args: Vec<&str> = ctx.subcommand.iter().map(|s| s.as_str()).collect();
        args.push("--json");
        let result = runner::run(&ctx.binary, &args, Duration::from_secs(5));
        let is_valid = serde_json::from_str::<serde_json::Value>(&result.stdout).is_ok();
        checks.push(if is_valid && result.exit_code == 0 {
            CheckResult::pass("Valid JSON output")
        } else {
            CheckResult::fail("Valid JSON output")
        });
    } else {
        checks.push(CheckResult::fail_with(
            "Valid JSON output",
            "no subcommand to test",
        ));
    }

    // Check 3: Auto-JSON when piped (stdout not a TTY)
    // When we run via Command, stdout is already not a TTY
    if !ctx.subcommand.is_empty() {
        let args: Vec<&str> = ctx.subcommand.iter().map(|s| s.as_str()).collect();
        let result = runner::run(&ctx.binary, &args, Duration::from_secs(5));
        let is_json = serde_json::from_str::<serde_json::Value>(&result.stdout).is_ok();
        checks.push(if is_json {
            CheckResult::pass("Auto-JSON when piped")
        } else {
            CheckResult::fail("Auto-JSON when piped")
        });
    } else {
        checks.push(CheckResult::fail_with(
            "Auto-JSON when piped",
            "no subcommand to test",
        ));
    }

    // Check 4: Structured errors on stderr
    let bad_result = runner::run(
        &ctx.binary,
        &["__nonexistent_command__"],
        Duration::from_secs(5),
    );
    let stderr_json = serde_json::from_str::<serde_json::Value>(&bad_result.stderr);
    let has_kind = stderr_json
        .as_ref()
        .ok()
        .and_then(|v| v.get("error"))
        .and_then(|e| e.get("kind"))
        .is_some();
    checks.push(if has_kind {
        CheckResult::pass("Structured errors")
    } else {
        CheckResult::fail("Structured errors")
    });

    // Check 5: --quiet flag
    checks.push(if help_info.has_flag("--quiet") {
        CheckResult::pass("--quiet flag")
    } else {
        CheckResult::fail("--quiet flag")
    });

    PrincipleScore::new("Structured Output", checks, 5)
}
