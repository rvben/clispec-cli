use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let mut checks = Vec::new();

    // Placeholder: check if help text mentions --limit
    if ctx.help_text.contains("--limit") {
        checks.push(CheckResult::pass("--limit flag"));
    }

    PrincipleScore::new("Bounded Output", checks, 3)
}
