use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let Some(schema) = ctx.schema_json.as_ref() else {
        return PrincipleScore::new(
            "Idempotent Operations",
            vec![
                CheckResult::fail_with("Effects declarations", "no schema"),
                CheckResult::fail_with("Idempotent conflict contract", "no schema"),
            ],
            2,
        );
    };
    let commands = schema
        .get("commands")
        .and_then(|c| c.as_array())
        .map(Vec::as_slice)
        .unwrap_or_default();

    let missing: Vec<_> = commands
        .iter()
        .filter(|c| c.get("effects").and_then(|v| v.as_str()).is_none())
        .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
        .collect();
    let effects = if missing.is_empty() && !commands.is_empty() {
        CheckResult::pass("Effects declarations")
    } else if commands.is_empty() {
        CheckResult::fail_with("Effects declarations", "no commands declared")
    } else {
        CheckResult::fail_with(
            "Effects declarations",
            &format!("missing on {}", missing.join(", ")),
        )
    };

    let idempotent_count = commands
        .iter()
        .filter(|c| c.get("effects").and_then(|v| v.as_str()) == Some("idempotent"))
        .count();
    let has_conflict = schema
        .get("errors")
        .and_then(|e| e.as_array())
        .is_some_and(|errors| {
            errors
                .iter()
                .any(|e| e.get("kind").and_then(|k| k.as_str()) == Some("conflict"))
        });
    let conflict = if idempotent_count == 0 {
        CheckResult::pass_with("Idempotent conflict contract", "no idempotent commands")
    } else if has_conflict {
        CheckResult::pass("Idempotent conflict contract")
    } else {
        CheckResult::pass_with(
            "Idempotent conflict contract",
            &format!(
                "{idempotent_count} idempotent command(s), no conflict declared; confirm no incompatible repeat state exists"
            ),
        )
    };

    PrincipleScore::new("Idempotent Operations", vec![effects, conflict], 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(schema: serde_json::Value) -> CheckContext {
        CheckContext {
            binary: "echo".into(),
            subcommand: vec![],
            help_text: String::new(),
            schema_json: Some(schema),
        }
    }

    #[test]
    fn read_only_tool_does_not_need_conflict() {
        let result = check(&context(serde_json::json!({
            "commands": [{"name": "list", "effects": "read_only"}],
            "errors": []
        })));
        assert!(result.checks.iter().all(|c| c.passed));
    }

    #[test]
    fn idempotent_tool_declares_conflict() {
        let result = check(&context(serde_json::json!({
            "commands": [{"name": "apply", "effects": "idempotent"}],
            "errors": [{"kind": "conflict"}]
        })));
        assert!(result.checks.iter().all(|c| c.passed));
    }
}
