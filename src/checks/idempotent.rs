use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let mut checks = Vec::new();

    // Placeholder: check schema for mutating markers
    if let Some(ref schema) = ctx.schema_json
        && schema.get("commands").is_some()
        && schema.get("errors").is_some()
    {
        checks.push(CheckResult::fail_with(
            "Mutating markers in schema",
            "not yet implemented",
        ));
    }

    PrincipleScore::new("Idempotent Operations", checks, 2)
}
