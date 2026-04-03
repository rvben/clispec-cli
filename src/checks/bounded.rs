use crate::help;

use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let help_info = help::parse_help(&ctx.help_text);
    let mut checks = Vec::new();

    // Check 1: --limit flag
    checks.push(if help_info.has_flag("--limit") {
        CheckResult::pass("--limit flag")
    } else {
        CheckResult::fail("--limit flag")
    });

    // Check 2: --offset, --cursor, or --page flag
    checks.push(
        if help_info.has_flag("--offset")
            || help_info.has_flag("--cursor")
            || help_info.has_flag("--page")
        {
            CheckResult::pass("Pagination flag")
        } else {
            CheckResult::fail("Pagination flag")
        },
    );

    // Check 3: --fields flag
    checks.push(if help_info.has_flag("--fields") {
        CheckResult::pass("--fields flag")
    } else {
        CheckResult::fail("--fields flag")
    });

    PrincipleScore::new("Bounded Output", checks, 3)
}
