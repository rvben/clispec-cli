use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let mut checks = Vec::new();

    if let Some(ref schema) = ctx.schema_json {
        // Check 1: Schema declares mutating markers
        let has_mutating = schema
            .get("commands")
            .and_then(|c| {
                c.as_object()
                    .map(|obj| obj.values().any(|v| v.get("mutating").is_some()))
                    .or_else(|| {
                        c.as_array()
                            .map(|arr| arr.iter().any(|v| v.get("mutating").is_some()))
                    })
            })
            .unwrap_or(false);
        checks.push(if has_mutating {
            CheckResult::pass("Mutating markers in schema")
        } else {
            CheckResult::fail("Mutating markers in schema")
        });

        // Check 2: Conflict error kind
        let has_conflict = schema
            .get("errors")
            .and_then(|e| e.as_array())
            .map(|arr| {
                arr.iter()
                    .any(|e| e.get("kind").and_then(|k| k.as_str()) == Some("conflict"))
            })
            .unwrap_or(false);
        checks.push(if has_conflict {
            CheckResult::pass("Conflict error kind")
        } else {
            CheckResult::fail("Conflict error kind")
        });
    } else {
        checks.push(CheckResult::fail_with(
            "Mutating markers in schema",
            "no schema",
        ));
        checks.push(CheckResult::fail_with("Conflict error kind", "no schema"));
    }

    PrincipleScore::new("Idempotent Operations", checks, 2)
}
