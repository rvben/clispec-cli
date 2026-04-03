use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let mut checks = Vec::new();

    // Placeholder: check if help text mentions --json
    if ctx.help_text.contains("--json") {
        checks.push(CheckResult::pass("--json flag"));
    }

    PrincipleScore::new("Structured Output", checks, 5)
}
