use crate::help;

use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let help_info = help::parse_help(&ctx.help_text);
    let sub_help_info = super::subcommand_help_info(ctx);
    let mut checks = Vec::new();

    // Check 1: --limit flag (top-level or subcommand)
    let has_limit = help_info.has_flag("--limit")
        || sub_help_info
            .as_ref()
            .is_some_and(|h| h.has_flag("--limit"));
    checks.push(if has_limit {
        CheckResult::pass("--limit flag")
    } else {
        CheckResult::fail("--limit flag")
    });

    // Check 2: --offset, --cursor, or --page flag (top-level or subcommand)
    let has_pagination = help_info.has_flag("--offset")
        || help_info.has_flag("--cursor")
        || help_info.has_flag("--page")
        || sub_help_info.as_ref().is_some_and(|h| {
            h.has_flag("--offset") || h.has_flag("--cursor") || h.has_flag("--page")
        });
    checks.push(if has_pagination {
        CheckResult::pass("Pagination flag")
    } else {
        CheckResult::fail("Pagination flag")
    });

    // Check 3: --fields flag (top-level or subcommand)
    let has_fields = help_info.has_flag("--fields")
        || sub_help_info
            .as_ref()
            .is_some_and(|h| h.has_flag("--fields"));
    checks.push(if has_fields {
        CheckResult::pass("--fields flag")
    } else {
        CheckResult::fail("--fields flag")
    });

    PrincipleScore::new("Bounded Output", checks, 3)
}
