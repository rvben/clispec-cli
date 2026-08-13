use std::collections::HashSet;

use super::{CheckContext, CheckResult, PrincipleScore};

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let Some(schema) = ctx.schema_json.as_ref() else {
        return PrincipleScore::new(
            "Bounded Output",
            vec![
                CheckResult::fail_with("Cardinality declarations", "no schema"),
                CheckResult::fail_with("Unbounded pagination declarations", "no schema"),
                CheckResult::fail_with("Unbounded field selection", "no schema"),
            ],
            3,
        );
    };
    let commands = schema
        .get("commands")
        .and_then(|c| c.as_array())
        .map(Vec::as_slice)
        .unwrap_or_default();
    let global_args = arg_names(schema.get("global_args"));

    let data: Vec<_> = commands
        .iter()
        .filter(|c| {
            c.get("output_kind")
                .and_then(|k| k.as_str())
                .unwrap_or("data")
                == "data"
        })
        .collect();
    let missing_cardinality: Vec<_> = data
        .iter()
        .filter(|c| c.get("cardinality").and_then(|v| v.as_str()).is_none())
        .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
        .collect();
    let cardinality = if missing_cardinality.is_empty() {
        CheckResult::pass("Cardinality declarations")
    } else {
        CheckResult::fail_with(
            "Cardinality declarations",
            &format!("missing on {}", missing_cardinality.join(", ")),
        )
    };

    let unbounded: Vec<_> = data
        .iter()
        .copied()
        .filter(|c| c.get("cardinality").and_then(|v| v.as_str()) == Some("unbounded"))
        .collect();
    let bad_pagination: Vec<_> = unbounded
        .iter()
        .filter(|c| !pagination_is_usable(c, &global_args))
        .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
        .collect();
    let pagination = if bad_pagination.is_empty() {
        CheckResult::pass_with(
            "Unbounded pagination declarations",
            if unbounded.is_empty() {
                "no unbounded commands"
            } else {
                "all unbounded commands declare usable pagination"
            },
        )
    } else {
        CheckResult::fail_with(
            "Unbounded pagination declarations",
            &format!("missing or unusable on {}", bad_pagination.join(", ")),
        )
    };

    let bad_fields: Vec<_> = unbounded
        .iter()
        .filter(|c| {
            let known = command_arg_names(c, &global_args);
            c.get("fields_arg")
                .and_then(|v| v.as_str())
                .is_none_or(|name| !known.contains(name))
        })
        .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
        .collect();
    let fields = if bad_fields.is_empty() {
        CheckResult::pass_with(
            "Unbounded field selection",
            if unbounded.is_empty() {
                "no unbounded commands"
            } else {
                "all unbounded commands declare fields_arg"
            },
        )
    } else {
        CheckResult::fail_with(
            "Unbounded field selection",
            &format!("missing or unresolved on {}", bad_fields.join(", ")),
        )
    };

    PrincipleScore::new("Bounded Output", vec![cardinality, pagination, fields], 3)
}

fn arg_names(value: Option<&serde_json::Value>) -> HashSet<&str> {
    value
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
        .collect()
}

fn command_arg_names<'a>(
    command: &'a serde_json::Value,
    globals: &HashSet<&'a str>,
) -> HashSet<&'a str> {
    let mut names = globals.clone();
    names.extend(arg_names(command.get("args")));
    names
}

fn pagination_is_usable(command: &serde_json::Value, globals: &HashSet<&str>) -> bool {
    let Some(pagination) = command.get("pagination") else {
        return false;
    };
    let known = command_arg_names(command, globals);
    let resolves = |key: &str| {
        pagination
            .get(key)
            .and_then(|v| v.as_str())
            .is_some_and(|name| known.contains(name))
    };
    match pagination.get("style").and_then(|v| v.as_str()) {
        Some("offset") => resolves("offset_arg") && resolves("limit_arg"),
        Some("cursor") => {
            pagination
                .get("cursor_field")
                .and_then(|v| v.as_str())
                .is_some()
                && resolves("cursor_arg")
                && resolves("limit_arg")
        }
        _ => false,
    }
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
    fn bounded_commands_do_not_owe_pagination_flags() {
        let result = check(&context(serde_json::json!({
            "commands": [{
                "name": "show", "output_kind": "data", "cardinality": "single"
            }]
        })));
        assert!(result.checks.iter().all(|c| c.passed));
    }

    #[test]
    fn unbounded_commands_need_resolving_pagination_and_fields_args() {
        let result = check(&context(serde_json::json!({
            "global_args": [{"name": "--limit"}, {"name": "--offset"}],
            "commands": [{
                "name": "list", "cardinality": "unbounded",
                "args": [{"name": "--fields"}],
                "pagination": {"style": "offset", "limit_arg": "--limit", "offset_arg": "--offset"},
                "fields_arg": "--fields"
            }]
        })));
        assert!(result.checks.iter().all(|c| c.passed));
    }
}
