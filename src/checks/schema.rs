use crate::runner;
use std::time::Duration;

use super::{CheckContext, CheckResult, PrincipleScore};

/// The canonical clispec v0.1 schema, vendored from clispec.dev/schema/v0.1.json.
const CLISPEC_SCHEMA_V0_1: &str = include_str!("../../schemas/v0.1.json");

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let mut checks = Vec::new();

    // Run schema command
    let result = runner::run(&ctx.binary, &["schema"], Duration::from_secs(5));
    let schema: Option<serde_json::Value> = serde_json::from_str(&result.stdout).ok();

    // Check 1: schema command exists and exits 0
    checks.push(if result.exit_code == 0 && schema.is_some() {
        CheckResult::pass("schema command exists")
    } else {
        CheckResult::fail("schema command exists")
    });

    // Check 2: Valid JSON output
    checks.push(if schema.is_some() {
        CheckResult::pass("Valid JSON schema")
    } else {
        CheckResult::fail("Valid JSON schema")
    });

    if let Some(ref s) = schema {
        // Check 3: Validates against clispec v0.1 JSON Schema
        checks.push(match validate_against_clispec_v0_1(s) {
            Ok(()) => CheckResult::pass("Validates against clispec v0.1"),
            Err(detail) => CheckResult::fail_with("Validates against clispec v0.1", &detail),
        });

        // Check 4: Has errors with kind/retryable
        let has_errors = s
            .get("errors")
            .and_then(|e| e.as_array())
            .map(|arr| arr.iter().any(|e| e.get("kind").is_some()))
            .unwrap_or(false);
        checks.push(if has_errors {
            CheckResult::pass("Error kinds documented")
        } else {
            CheckResult::fail("Error kinds documented")
        });

        // Check 5: Commands have output_fields
        let has_output_fields = s
            .get("commands")
            .and_then(|c| {
                c.as_object()
                    .map(|obj| obj.values().any(|v| v.get("output_fields").is_some()))
                    .or_else(|| {
                        c.as_array()
                            .map(|arr| arr.iter().any(|v| v.get("output_fields").is_some()))
                    })
            })
            .unwrap_or(false);
        checks.push(if has_output_fields {
            CheckResult::pass("Output fields declared")
        } else {
            CheckResult::fail("Output fields declared")
        });
    } else {
        checks.push(CheckResult::fail("Validates against clispec v0.1"));
        checks.push(CheckResult::fail("Error kinds documented"));
        checks.push(CheckResult::fail("Output fields declared"));
    }

    PrincipleScore::new("Schema Introspection", checks, 5)
}

/// Validate an instance against the bundled clispec v0.1 JSON Schema.
/// Returns Ok on success, or Err with the first validation error message.
fn validate_against_clispec_v0_1(instance: &serde_json::Value) -> Result<(), String> {
    let schema: serde_json::Value = serde_json::from_str(CLISPEC_SCHEMA_V0_1)
        .expect("bundled clispec schema must be valid JSON");
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|e| format!("bundled schema is not a valid Draft 2020-12 schema: {e}"))?;
    match validator.iter_errors(instance).next() {
        None => Ok(()),
        Some(err) => Err(format!("{}: {}", err.instance_path(), err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_valid() -> serde_json::Value {
        serde_json::json!({
            "name": "mytool",
            "version": "1.0.0",
            "commands": [{ "name": "list" }]
        })
    }

    #[test]
    fn bundled_schema_is_valid_draft_2020_12() {
        let schema: serde_json::Value = serde_json::from_str(CLISPEC_SCHEMA_V0_1).unwrap();
        jsonschema::draft202012::new(&schema).expect("bundled schema must be valid");
    }

    #[test]
    fn minimal_document_validates() {
        validate_against_clispec_v0_1(&minimal_valid()).expect("minimal doc should validate");
    }

    #[test]
    fn rich_document_from_spec_validates() {
        let doc = serde_json::json!({
            "name": "mytool",
            "version": "1.2.0",
            "commands": [{
                "name": "list",
                "description": "List all services",
                "mutating": false,
                "args": [
                    {"name": "--status", "type": "string", "required": false,
                     "enum": ["running", "stopped", "all"], "default": "all"},
                    {"name": "--limit", "type": "integer", "required": false, "default": 100}
                ],
                "output_fields": [
                    {"name": "name", "type": "string"},
                    {"name": "status", "type": "string"},
                    {"name": "uptime_seconds", "type": "integer | null"}
                ]
            }],
            "errors": [
                {"kind": "auth", "retryable": false, "description": "Authentication failed"},
                {"kind": "rate_limit", "retryable": true, "description": "Too many requests"}
            ]
        });
        validate_against_clispec_v0_1(&doc).expect("spec example should validate");
    }

    #[test]
    fn missing_required_field_fails() {
        let doc = serde_json::json!({ "name": "mytool", "version": "1.0.0" });
        validate_against_clispec_v0_1(&doc).expect_err("missing commands should fail");
    }

    #[test]
    fn error_kind_must_be_snake_case() {
        let doc = serde_json::json!({
            "name": "mytool",
            "version": "1.0.0",
            "commands": [{ "name": "list" }],
            "errors": [{ "kind": "Not-Found" }]
        });
        validate_against_clispec_v0_1(&doc).expect_err("non-snake_case kind should fail");
    }

    #[test]
    fn additional_properties_are_permitted() {
        let doc = serde_json::json!({
            "name": "mytool",
            "version": "1.0.0",
            "commands": [{ "name": "list", "x_custom": "anything" }],
            "x_tool_metadata": { "vendor": "acme" }
        });
        validate_against_clispec_v0_1(&doc).expect("extensions should validate");
    }
}
