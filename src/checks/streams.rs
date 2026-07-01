use crate::runner;

use super::{CheckContext, CheckResult, PrincipleScore, Probe};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let mut checks = Vec::new();

    if !ctx.subcommand.is_empty() {
        let clean = structured_stdout_is_clean(ctx, &ctx.probe());
        checks.push(if clean {
            CheckResult::pass("Clean stdout when piped")
        } else {
            CheckResult::fail("Clean stdout when piped")
        });
        checks.push(if clean {
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

/// Whether the tool yields clean JSON on stdout - as its piped default, or via an
/// explicit JSON format flag (for a tool with a declared human default). A clean
/// structured stream confirms data and diagnostics are separated.
fn structured_stdout_is_clean(ctx: &CheckContext, probe: &Probe) -> bool {
    let base: Vec<&str> = probe.args.iter().map(|s| s.as_str()).collect();
    let bare = runner::run_with_stdin(
        &ctx.binary,
        &base,
        probe.stdin.as_deref(),
        runner::PROBE_TIMEOUT,
    );
    if serde_json::from_str::<serde_json::Value>(&bare.stdout).is_ok() {
        return true;
    }
    let json_flags: &[&[&str]] = &[
        &["--json"],
        &["-o", "json"],
        &["--output", "json"],
        &["--format", "json"],
    ];
    for flags in json_flags {
        let mut args = base.clone();
        args.extend_from_slice(flags);
        let result = runner::run_with_stdin(
            &ctx.binary,
            &args,
            probe.stdin.as_deref(),
            runner::PROBE_TIMEOUT,
        );
        if serde_json::from_str::<serde_json::Value>(&result.stdout).is_ok() {
            return true;
        }
    }
    false
}
