use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let mut checks = Vec::new();

    // Placeholder: check if a subcommand is available to test
    if !ctx.subcommand.is_empty() {
        checks.push(CheckResult::fail("Clean stdout when piped"));
    }

    PrincipleScore::new("Stderr/Stdout Separation", checks, 2)
}
