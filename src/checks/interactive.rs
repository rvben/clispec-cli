use crate::help;
use crate::runner;

use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let help_info = help::parse_help(&ctx.help_text);
    let sub_help_info = super::subcommand_help_info(ctx);
    let mut checks = Vec::new();

    // Check 1: No hang without TTY (run with closed stdin; the runner enforces
    // PROBE_TIMEOUT as a safety ceiling). runner::run already uses Stdio::null().
    let result = runner::run(&ctx.binary, &["--help"], runner::PROBE_TIMEOUT);
    checks.push(if result.exit_code >= 0 {
        CheckResult::pass("No TTY hang")
    } else {
        CheckResult::fail("No TTY hang")
    });

    // Check 2: a bypass flag for confirmation prompts. The spec scopes this to
    // commands that actually prompt, which a tool signals with a
    // `confirmation_required` error kind. Only then must it expose a bypass flag
    // (--yes/--force) for non-TTY use. A tool that declares no confirmation gate
    // has nothing to bypass and satisfies the rule as-is; a `mutating` command
    // that never prompts must not be penalized (the spec forbids adding gates as
    // a mechanical compliance step).
    let has_yes_or_force = help_info.has_flag("--yes")
        || help_info.has_flag("--force")
        || sub_help_info
            .as_ref()
            .is_some_and(|h| h.has_flag("--yes") || h.has_flag("--force"));
    // The bypass flag may live on the destructive subcommand only; the schema
    // declares it even when the probed help text does not show it.
    let schema_declares_bypass = ctx.schema_json.as_ref().is_some_and(schema_has_bypass_flag);
    let declares_confirmation = ctx
        .schema_json
        .as_ref()
        .is_some_and(schema_declares_confirmation);
    checks.push(
        if !declares_confirmation || has_yes_or_force || schema_declares_bypass {
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
    // The bypass flag usually lives on a nested destructive subcommand
    // (e.g. `apps delete --yes`), so the walk must recurse.
    fn any_command_has_bypass(
        cmd: &serde_json::Value,
        args_have_bypass: &impl Fn(&serde_json::Value) -> bool,
    ) -> bool {
        if cmd.get("args").is_some_and(args_have_bypass) {
            return true;
        }
        cmd.get("subcommands")
            .and_then(|s| s.as_array())
            .is_some_and(|subs| {
                subs.iter()
                    .any(|sub| any_command_has_bypass(sub, args_have_bypass))
            })
    }
    schema
        .get("commands")
        .and_then(|c| c.as_array())
        .is_some_and(|cmds| {
            cmds.iter()
                .any(|c| any_command_has_bypass(c, &args_have_bypass))
        })
}

/// True when the schema declares a `confirmation_required` error kind - the
/// spec's signal that some command gates on confirmation and therefore needs a
/// non-TTY bypass flag.
fn schema_declares_confirmation(schema: &serde_json::Value) -> bool {
    schema
        .get("errors")
        .and_then(|e| e.as_array())
        .is_some_and(|errors| {
            errors.iter().any(|e| {
                e.get("kind").and_then(serde_json::Value::as_str) == Some("confirmation_required")
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bypass_flag_found_on_nested_subcommand() {
        let schema = serde_json::json!({
            "commands": [
                {"name": "apps", "subcommands": [
                    {"name": "delete", "mutating": true, "args": [
                        {"name": "--yes", "type": "boolean"}
                    ]}
                ]}
            ]
        });
        assert!(schema_has_bypass_flag(&schema));
    }

    #[test]
    fn no_bypass_flag_anywhere_is_false() {
        let schema = serde_json::json!({
            "commands": [
                {"name": "apps", "subcommands": [
                    {"name": "delete", "mutating": true, "args": [
                        {"name": "--slug", "type": "string"}
                    ]}
                ]}
            ]
        });
        assert!(!schema_has_bypass_flag(&schema));
    }

    #[test]
    fn bypass_flag_in_global_args_is_found() {
        let schema = serde_json::json!({
            "global_args": [{"name": "--force", "type": "boolean"}],
            "commands": []
        });
        assert!(schema_has_bypass_flag(&schema));
    }

    #[test]
    fn schema_declares_confirmation_detects_error_kind() {
        assert!(schema_declares_confirmation(
            &serde_json::json!({"errors": [{"kind": "confirmation_required"}]})
        ));
        assert!(!schema_declares_confirmation(
            &serde_json::json!({"errors": [{"kind": "usage"}]})
        ));
    }

    fn context_with(schema: serde_json::Value) -> CheckContext {
        CheckContext {
            binary: "echo".to_string(),
            subcommand: vec![],
            help_text: String::new(),
            schema_json: Some(schema),
        }
    }

    fn yes_flag_passed(ctx: &CheckContext) -> bool {
        check(ctx)
            .checks
            .iter()
            .find(|c| c.name == "--yes flag")
            .expect("check present")
            .passed
    }

    #[test]
    fn mutating_command_without_confirmation_gate_passes() {
        // A mutating command that never prompts (no confirmation_required error
        // kind, no bypass flag) must not be dinged.
        let ctx = context_with(serde_json::json!({
            "commands": [{"name": "fix", "mutating": true}],
            "errors": [{"kind": "io"}]
        }));
        assert!(yes_flag_passed(&ctx));
    }

    #[test]
    fn confirmation_gate_without_bypass_flag_fails() {
        let ctx = context_with(serde_json::json!({
            "commands": [{"name": "delete", "mutating": true,
                "args": [{"name": "--slug", "type": "string"}]}],
            "errors": [{"kind": "confirmation_required"}]
        }));
        assert!(!yes_flag_passed(&ctx));
    }

    #[test]
    fn confirmation_gate_with_bypass_flag_passes() {
        let ctx = context_with(serde_json::json!({
            "commands": [{"name": "delete", "mutating": true,
                "args": [{"name": "--yes", "type": "boolean"}]}],
            "errors": [{"kind": "confirmation_required"}]
        }));
        assert!(yes_flag_passed(&ctx));
    }
}
