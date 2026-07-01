use crate::help;
use crate::runner;

use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let help_info = help::parse_help(&ctx.help_text);
    let mut checks = Vec::new();
    // Exit codes the schema declares as outcomes (data states, not failures)
    // count as success: a diff-like tool exits 1 with a valid report on
    // stdout, and that must not fail the output checks.
    let outcome_codes = declared_outcome_codes(ctx);
    let ok_exit = |code: i32| code == 0 || outcome_codes.contains(&code);

    // How to invoke the representative command (its declared `example`, or the
    // discovered subcommand name as a fallback), plus any stdin it wants.
    let probe = ctx.probe();

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
            let mut args: Vec<&str> = probe.args.iter().map(|s| s.as_str()).collect();
            args.extend_from_slice(flags);
            let result = runner::run_with_stdin(
                &ctx.binary,
                &args,
                probe.stdin.as_deref(),
                runner::PROBE_TIMEOUT,
            );
            if ok_exit(result.exit_code)
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

    // Check 3: structured output when piped. Emitting structured output by
    // default when piped is the SHOULD ideal; a tool MAY keep a human-readable
    // default if it DECLARES it in the schema `output` field, so the behavior is
    // discoverable rather than a surprise. Undeclared human output, or a declared
    // structured default the tool does not actually emit, fails.
    const CHECK_3: &str = "Structured or declared piped output";
    if !ctx.subcommand.is_empty() {
        let args: Vec<&str> = probe.args.iter().map(|s| s.as_str()).collect();
        let result = runner::run_with_stdin(
            &ctx.binary,
            &args,
            probe.stdin.as_deref(),
            runner::PROBE_TIMEOUT,
        );
        let auto_structured = serde_json::from_str::<serde_json::Value>(&result.stdout).is_ok();
        let piped_default = declared_piped_default(ctx);
        checks.push(if auto_structured {
            CheckResult::pass_with(CHECK_3, "structured by default when piped")
        } else if piped_default.as_deref().is_some_and(is_human_format) {
            CheckResult::pass_with(
                CHECK_3,
                &format!(
                    "declared human default (output.piped = {})",
                    piped_default.unwrap()
                ),
            )
        } else if let Some(format) = piped_default {
            CheckResult::fail_with(
                CHECK_3,
                &format!("declares output.piped = {format} but does not emit it when piped"),
            )
        } else {
            CheckResult::fail_with(
                CHECK_3,
                "not structured when piped and no `output` default declared",
            )
        });
    } else {
        checks.push(CheckResult::fail_with(CHECK_3, "no subcommand to test"));
    }

    // Check 4: Structured errors on stderr (the envelope is the last
    // line of stderr per the spec; whole-stderr JSON is also accepted)
    let bad_result = runner::run(
        &ctx.binary,
        &["__nonexistent_command__"],
        runner::PROBE_TIMEOUT,
    );
    checks.push(if stderr_has_error_envelope(&bad_result.stderr) {
        CheckResult::pass("Structured errors")
    } else {
        CheckResult::fail("Structured errors")
    });

    // Check 5: Explicit format selection wins over TTY detection
    // (stdout is piped here, so an explicit human format must still produce
    // non-JSON output). Tools name their human format "text" or "table";
    // probe both vocabularies.
    if !ctx.subcommand.is_empty() {
        let text_flags: &[&[&str]] = &[
            &["-o", "text"],
            &["--output", "text"],
            &["--format", "text"],
            &["-o", "table"],
            &["--output", "table"],
            &["--format", "table"],
        ];
        let mut honored = false;
        for flags in text_flags {
            let mut args: Vec<&str> = probe.args.iter().map(|s| s.as_str()).collect();
            args.extend_from_slice(flags);
            let result = runner::run_with_stdin(
                &ctx.binary,
                &args,
                probe.stdin.as_deref(),
                runner::PROBE_TIMEOUT,
            );
            // Non-empty stdout required: a tool that treats -o as an output
            // filename exits 0 with empty stdout and must not pass.
            if ok_exit(result.exit_code)
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

/// Exit codes the schema's `outcomes` array declares as data states.
fn declared_outcome_codes(ctx: &CheckContext) -> Vec<i32> {
    ctx.schema_json
        .as_ref()
        .and_then(|s| s.get("outcomes"))
        .and_then(|o| o.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|o| o.get("code").and_then(serde_json::Value::as_i64))
                .map(|c| c as i32)
                .collect()
        })
        .unwrap_or_default()
}

/// The `output.piped` value declared in the schema (the format emitted when
/// stdout is not a TTY and no format flag is given), if the tool declares one.
fn declared_piped_default(ctx: &CheckContext) -> Option<String> {
    ctx.schema_json
        .as_ref()?
        .get("output")?
        .get("piped")?
        .as_str()
        .map(str::to_string)
}

/// Whether a format name is human-readable (non-structured).
fn is_human_format(format: &str) -> bool {
    matches!(format, "text" | "table" | "plain")
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

    fn ctx_with(schema: serde_json::Value) -> CheckContext {
        CheckContext {
            binary: "x".to_string(),
            subcommand: vec![],
            help_text: String::new(),
            schema_json: Some(schema),
        }
    }

    #[test]
    fn declared_piped_default_reads_output_field() {
        let ctx = ctx_with(serde_json::json!({"output": {"tty": "text", "piped": "text"}}));
        assert_eq!(declared_piped_default(&ctx).as_deref(), Some("text"));
        assert_eq!(
            declared_piped_default(&ctx_with(serde_json::json!({}))),
            None
        );
    }

    #[test]
    fn human_formats_classified() {
        assert!(is_human_format("text"));
        assert!(is_human_format("table"));
        assert!(!is_human_format("json"));
        assert!(!is_human_format("yaml"));
    }
}
