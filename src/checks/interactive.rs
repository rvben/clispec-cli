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

    // Check 2: --yes or --force flag (check both top-level and subcommand help).
    // The spec requires a flag alternative for every interactive prompt; a tool
    // whose schema declares no mutating commands has nothing to confirm, so the
    // requirement is vacuously satisfied.
    let has_yes_or_force = help_info.has_flag("--yes")
        || help_info.has_flag("--force")
        || sub_help_info
            .as_ref()
            .is_some_and(|h| h.has_flag("--yes") || h.has_flag("--force"));
    let no_mutating_commands = ctx
        .schema_json
        .as_ref()
        .and_then(|s| s.get("commands"))
        .and_then(|c| c.as_array())
        .is_some_and(|cmds| {
            !cmds.is_empty()
                && cmds.iter().all(|c| {
                    c.get("mutating")
                        .and_then(serde_json::Value::as_bool)
                        .is_some_and(|m| !m)
                })
        });
    // The bypass flag may live on the destructive subcommand only; the schema
    // declares it even when the probed help text does not show it.
    let schema_declares_bypass = ctx.schema_json.as_ref().is_some_and(schema_has_bypass_flag);
    checks.push(
        if has_yes_or_force || no_mutating_commands || schema_declares_bypass {
            CheckResult::pass("--yes flag")
        } else {
            CheckResult::fail("--yes flag")
        },
    );

    PrincipleScore::new("Non-Interactive", checks, 2)
}

/// True when any command (or global arg) in the schema declares a --yes or
/// --force style bypass flag, including required positional `yes` args.
fn schema_has_bypass_flag(schema: &serde_json::Value) -> bool {
    let arg_is_bypass = |arg: &serde_json::Value| {
        arg.get("name")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|n| {
                let n = n.trim_start_matches('-');
                n == "yes" || n == "force" || n == "y"
            })
    };
    let args_have_bypass = |v: &serde_json::Value| {
        v.as_array()
            .is_some_and(|args| args.iter().any(arg_is_bypass))
    };
    if schema.get("global_args").is_some_and(args_have_bypass) {
        return true;
    }
    schema
        .get("commands")
        .and_then(|c| c.as_array())
        .is_some_and(|cmds| {
            cmds.iter()
                .any(|c| c.get("args").is_some_and(args_have_bypass))
        })
}
