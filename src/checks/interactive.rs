use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let mut checks = Vec::new();

    // Placeholder: check if help text mentions --yes
    if ctx.help_text.contains("--yes") {
        checks.push(CheckResult::pass("--yes flag"));
    }

    PrincipleScore::new("Non-Interactive", checks, 3)
}
