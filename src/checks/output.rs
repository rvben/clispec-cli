use crate::help;
use crate::runner;
use std::time::Duration;

use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let help_info = help::parse_help(&ctx.help_text);
    let mut checks = Vec::new();

    let sub_help_info = super::subcommand_help_info(ctx);

    // Check 1: JSON output flag in help (--json, --output, -o, --format)
    // Check both top-level and subcommand help
    let has_json_flag = help_info.has_flag("--json")
        || help_info.has_flag("--output")
        || help_info.has_flag("--format")
        || help_info.has_flag("-o")
        || sub_help_info.as_ref().is_some_and(|h| {
            h.has_flag("--json")
                || h.has_flag("--output")
                || h.has_flag("--format")
                || h.has_flag("-o")
        });
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

    // Check 4: Structured errors on stderr (the envelope is the last
    // line of stderr per the spec; whole-stderr JSON is also accepted)
    let bad_result = runner::run(
        &ctx.binary,
        &["__nonexistent_command__"],
        Duration::from_secs(5),
    );
    checks.push(if stderr_has_error_envelope(&bad_result.stderr) {
        CheckResult::pass("Structured errors")
    } else {
        CheckResult::fail("Structured errors")
    });

    // Check 5: Explicit format selection wins over TTY detection
    // (stdout is piped here, so `-o text` must still produce text)
    if !ctx.subcommand.is_empty() {
        let text_flags: &[&[&str]] = &[
            &["-o", "text"],
            &["--output", "text"],
            &["--format", "text"],
        ];
        let mut honored = false;
        for flags in text_flags {
            let mut args: Vec<&str> = ctx.subcommand.iter().map(|s| s.as_str()).collect();
            args.extend_from_slice(flags);
            let result = runner::run(&ctx.binary, &args, Duration::from_secs(5));
            // Non-empty stdout required: a tool that treats -o as an output
            // filename exits 0 with empty stdout and must not pass.
            if result.exit_code == 0
                && !result.stdout.trim().is_empty()
                && serde_json::from_str::<serde_json::Value>(&result.stdout).is_err()
            {
                honored = true;
                break;
            }
        }
        checks.push(if honored {
            CheckResult::pass("Explicit format wins")
        } else {
            CheckResult::fail("Explicit format wins")
        });
    } else {
        checks.push(CheckResult::fail_with(
            "Explicit format wins",
            "no subcommand to test",
        ));
    }

    PrincipleScore::new("Structured Output", checks, 5)
}

/// True when stderr carries a structured error envelope with a `kind`,
/// either as its last non-empty line (the spec rule) or as the whole stream.
fn stderr_has_error_envelope(stderr: &str) -> bool {
    let has_kind = |v: &serde_json::Value| {
        v.get("error")
            .and_then(|e| e.get("kind"))
            .is_some_and(|k| k.is_string())
    };
    if let Some(last_line) = stderr.lines().rev().find(|l| !l.trim().is_empty())
        && serde_json::from_str::<serde_json::Value>(last_line).is_ok_and(|v| has_kind(&v))
    {
        return true;
    }
    serde_json::from_str::<serde_json::Value>(stderr).is_ok_and(|v| has_kind(&v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_on_last_line_after_progress() {
        let stderr = "Fetching services...\ndone.\n{\"error\": {\"kind\": \"auth\", \"message\": \"expired\"}}\n";
        assert!(stderr_has_error_envelope(stderr));
    }

    #[test]
    fn whole_stderr_envelope_accepted() {
        let stderr = "{\n  \"error\": {\n    \"kind\": \"auth\"\n  }\n}\n";
        assert!(stderr_has_error_envelope(stderr));
    }

    #[test]
    fn prose_only_stderr_rejected() {
        assert!(!stderr_has_error_envelope("Error: something went wrong\n"));
        assert!(!stderr_has_error_envelope(""));
    }

    #[test]
    fn envelope_without_kind_rejected() {
        let stderr = "{\"error\": {\"message\": \"expired\"}}\n";
        assert!(!stderr_has_error_envelope(stderr));
    }
}
