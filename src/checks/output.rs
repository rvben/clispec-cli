use crate::help;
use crate::runner;
use std::time::Duration;

use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let help_info = help::parse_help(&ctx.help_text);
    let mut checks = Vec::new();

    // Check 1: JSON output flag in help (--json, --output, -o, --format)
    let has_json_flag = help_info.has_flag("--json")
        || help_info.has_flag("--output")
        || help_info.has_flag("--format")
        || help_info.has_flag("-o");
    checks.push(if has_json_flag {
        CheckResult::pass("JSON output flag")
    } else {
        CheckResult::fail("JSON output flag")
    });

    // Check 2: Valid JSON output
    // Try multiple flag conventions to find one that works
    if !ctx.subcommand.is_empty() {
        let json_flags: &[&[&str]] = &[
            &["--json"],
            &["-o", "json"],
            &["--output", "json"],
            &["--format", "json"],
        ];
        let mut found_valid = false;
        for flags in json_flags {
            let mut args: Vec<&str> = ctx.subcommand.iter().map(|s| s.as_str()).collect();
            args.extend_from_slice(flags);
            let result = runner::run(&ctx.binary, &args, Duration::from_secs(5));
            if result.exit_code == 0
                && serde_json::from_str::<serde_json::Value>(&result.stdout).is_ok()
            {
                found_valid = true;
                break;
            }
        }
        checks.push(if found_valid {
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

    // Check 5: --quiet or -q flag
    checks.push(
        if help_info.has_flag("--quiet") || help_info.has_flag("-q") {
            CheckResult::pass("--quiet flag")
        } else {
            CheckResult::fail("--quiet flag")
        },
    );

    PrincipleScore::new("Structured Output", checks, 5)
}
