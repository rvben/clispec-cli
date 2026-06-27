use crate::runner;

use super::{CheckContext, CheckResult, PrincipleScore};

/// The canonical clispec v0.2 schema, vendored from clispec.dev/schema/v0.2.json.
/// v0.2 is additive over v0.1, so v0.1-shaped documents validate too.
const CLISPEC_SCHEMA_V0_2: &str = include_str!("../../schemas/v0.2.json");

pub fn check(ctx: &CheckContext) -> PrincipleScore {
    let mut checks = Vec::new();

    // Run schema command
    let result = runner::run(&ctx.binary, &["schema"], runner::PROBE_TIMEOUT);
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
        // Check 3: Validates against clispec v0.2 JSON Schema
        checks.push(match validate_against_clispec_v0_2(s) {
            Ok(()) => CheckResult::pass("Validates against clispec v0.2"),
            Err(detail) => CheckResult::fail_with("Validates against clispec v0.2", &detail),
        });

        // Check 4: Has errors with kind
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

        // Check 6: Global args declared at the top level. An empty array
        // is a valid declaration: it states the tool has no global flags.
        let has_global_args = s.get("global_args").is_some_and(|g| g.is_array());
        checks.push(if has_global_args {
            CheckResult::pass("Global args declared")
        } else {
            CheckResult::fail("Global args declared")
        });

        // Check 7: Error kinds expose exit codes. The published schema makes
        // exit_code optional per kind (a passthrough kind like a remote job's
        // own exit code legitimately omits it), so the check verifies the
        // tool adopted the feature, not 100% coverage. Partial coverage
        // passes with the ratio in the detail.
        checks.push(match exit_code_coverage(s) {
            ExitCodeCoverage::Full => CheckResult::pass("Exit codes on error kinds"),
            ExitCodeCoverage::NoErrors => {
                CheckResult::fail_with("Exit codes on error kinds", "no error kinds declared")
            }
            ExitCodeCoverage::None { total } => CheckResult::fail_with(
                "Exit codes on error kinds",
                &format!("none of {total} error kinds declare exit_code"),
            ),
            ExitCodeCoverage::Partial { declared, total } => CheckResult::pass_with(
                "Exit codes on error kinds",
                &format!("{declared} of {total} error kinds declare exit_code"),
            ),
        });

        // Check 8: Every leaf command carries an explicit mutating marker
        // (the spec defines absence as unknown, not read-only)
        checks.push(match mutating_coverage(s) {
            MutatingCoverage::Full => CheckResult::pass("Mutation markers on all commands"),
            MutatingCoverage::NoCommands => {
                CheckResult::fail_with("Mutation markers on all commands", "no commands declared")
            }
            MutatingCoverage::Partial { missing, total } => CheckResult::fail_with(
                "Mutation markers on all commands",
                &format!("{missing} of {total} commands missing mutating"),
            ),
        });
    } else {
        checks.push(CheckResult::fail("Validates against clispec v0.2"));
        checks.push(CheckResult::fail("Error kinds documented"));
        checks.push(CheckResult::fail("Output fields declared"));
        checks.push(CheckResult::fail("Global args declared"));
        checks.push(CheckResult::fail("Exit codes on error kinds"));
        checks.push(CheckResult::fail("Mutation markers on all commands"));
    }

    // Check 9: schema is discoverable from root --help
    checks.push(if ctx.help_text.to_lowercase().contains("schema") {
        CheckResult::pass("schema mentioned in --help")
    } else {
        CheckResult::fail("schema mentioned in --help")
    });

    // Check 10: schema works without configuration (HOME pointed at an
    // empty directory; auth tokens inherited from the real env are an
    // accepted blind spot of this probe)
    checks.push(if schema_works_without_config(&ctx.binary) {
        CheckResult::pass("schema works without config")
    } else {
        CheckResult::fail("schema works without config")
    });

    PrincipleScore::new("Schema Introspection", checks, 10)
}

/// Validate an instance against the bundled clispec v0.2 JSON Schema.
/// Returns Ok on success, or Err with the first validation error message.
fn validate_against_clispec_v0_2(instance: &serde_json::Value) -> Result<(), String> {
    let schema: serde_json::Value = serde_json::from_str(CLISPEC_SCHEMA_V0_2)
        .expect("bundled clispec schema must be valid JSON");
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|e| format!("bundled schema is not a valid Draft 2020-12 schema: {e}"))?;
    match validator.iter_errors(instance).next() {
        None => Ok(()),
        Some(err) => Err(format!("{}: {}", err.instance_path(), err)),
    }
}

enum ExitCodeCoverage {
    Full,
    NoErrors,
    None { total: u32 },
    Partial { declared: u32, total: u32 },
}

fn exit_code_coverage(schema: &serde_json::Value) -> ExitCodeCoverage {
    let Some(errors) = schema.get("errors").and_then(|e| e.as_array()) else {
        return ExitCodeCoverage::NoErrors;
    };
    if errors.is_empty() {
        return ExitCodeCoverage::NoErrors;
    }
    let total = errors.len() as u32;
    let declared = errors
        .iter()
        .filter(|e| e.get("exit_code").is_some_and(|c| c.is_i64() || c.is_u64()))
        .count() as u32;
    if declared == total {
        ExitCodeCoverage::Full
    } else if declared == 0 {
        ExitCodeCoverage::None { total }
    } else {
        ExitCodeCoverage::Partial { declared, total }
    }
}

enum MutatingCoverage {
    Full,
    NoCommands,
    Partial { missing: u32, total: u32 },
}

fn mutating_coverage(schema: &serde_json::Value) -> MutatingCoverage {
    fn walk(cmd: &serde_json::Value, total: &mut u32, missing: &mut u32) {
        if let Some(subs) = cmd.get("subcommands").and_then(|s| s.as_array())
            && !subs.is_empty()
        {
            for sub in subs {
                walk(sub, total, missing);
            }
            return;
        }
        *total += 1;
        if !cmd.get("mutating").is_some_and(|m| m.is_boolean()) {
            *missing += 1;
        }
    }

    let mut total = 0;
    let mut missing = 0;
    match schema.get("commands") {
        Some(serde_json::Value::Array(arr)) => {
            for cmd in arr {
                walk(cmd, &mut total, &mut missing);
            }
        }
        Some(serde_json::Value::Object(obj)) => {
            for cmd in obj.values() {
                walk(cmd, &mut total, &mut missing);
            }
        }
        _ => {}
    }

    if total == 0 {
        MutatingCoverage::NoCommands
    } else if missing == 0 {
        MutatingCoverage::Full
    } else {
        MutatingCoverage::Partial { missing, total }
    }
}

/// Run `binary schema` with HOME and XDG_CONFIG_HOME pointed at an empty
/// directory. The spec requires schema to work before any setup has happened.
fn schema_works_without_config(binary: &str) -> bool {
    let tmp = std::env::temp_dir().join(format!("clispec-noconfig-{}", std::process::id()));
    if std::fs::create_dir_all(&tmp).is_err() {
        return false;
    }
    let tmp = tmp.to_string_lossy();
    let result = runner::run_with_env(
        binary,
        &["schema"],
        runner::PROBE_TIMEOUT,
        &[("HOME", &tmp), ("XDG_CONFIG_HOME", &tmp)],
    );
    result.exit_code == 0 && serde_json::from_str::<serde_json::Value>(&result.stdout).is_ok()
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
        let schema: serde_json::Value = serde_json::from_str(CLISPEC_SCHEMA_V0_2).unwrap();
        jsonschema::draft202012::new(&schema).expect("bundled schema must be valid");
    }

    #[test]
    fn minimal_document_validates() {
        validate_against_clispec_v0_2(&minimal_valid()).expect("minimal doc should validate");
    }

    #[test]
    fn v0_1_shaped_document_still_validates() {
        // The pre-v0.2 spec example: no clispec field, no global_args,
        // no exit_code on errors. v0.2 is additive, so this must pass.
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
        validate_against_clispec_v0_2(&doc).expect("v0.1-shaped doc should validate");
    }

    #[test]
    fn v0_2_spec_example_validates() {
        let doc = serde_json::json!({
            "clispec": "0.2",
            "name": "mytool",
            "version": "1.2.0",
            "global_args": [
                {"name": "--output", "type": "string",
                 "enum": ["auto", "text", "json", "yaml"], "default": "auto"},
                {"name": "--quiet", "type": "boolean", "default": false}
            ],
            "commands": [{ "name": "list", "mutating": false }],
            "errors": [
                {"kind": "auth", "exit_code": 3, "retryable": false},
                {"kind": "not_found", "exit_code": 4, "retryable": false}
            ]
        });
        validate_against_clispec_v0_2(&doc).expect("v0.2 spec example should validate");
    }

    #[test]
    fn missing_required_field_fails() {
        let doc = serde_json::json!({ "name": "mytool", "version": "1.0.0" });
        validate_against_clispec_v0_2(&doc).expect_err("missing commands should fail");
    }

    #[test]
    fn error_kind_must_be_snake_case() {
        let doc = serde_json::json!({
            "name": "mytool",
            "version": "1.0.0",
            "commands": [{ "name": "list" }],
            "errors": [{ "kind": "Not-Found" }]
        });
        validate_against_clispec_v0_2(&doc).expect_err("non-snake_case kind should fail");
    }

    #[test]
    fn additional_properties_are_permitted() {
        let doc = serde_json::json!({
            "name": "mytool",
            "version": "1.0.0",
            "commands": [{ "name": "list", "x_custom": "anything" }],
            "x_tool_metadata": { "vendor": "acme" }
        });
        validate_against_clispec_v0_2(&doc).expect("extensions should validate");
    }

    #[test]
    fn exit_code_coverage_full_partial_none() {
        let full = serde_json::json!({"errors": [
            {"kind": "auth", "exit_code": 3},
            {"kind": "not_found", "exit_code": 4}
        ]});
        assert!(matches!(exit_code_coverage(&full), ExitCodeCoverage::Full));

        // exit_code is optional per kind in the published schema; a
        // passthrough kind (e.g. a remote job's own exit code) omits it.
        // Partial coverage counts the kinds that DO declare it.
        let partial = serde_json::json!({"errors": [
            {"kind": "auth", "exit_code": 3},
            {"kind": "job_failed"}
        ]});
        assert!(matches!(
            exit_code_coverage(&partial),
            ExitCodeCoverage::Partial {
                declared: 1,
                total: 2
            }
        ));

        let none_declared = serde_json::json!({"errors": [
            {"kind": "auth"},
            {"kind": "not_found"}
        ]});
        assert!(matches!(
            exit_code_coverage(&none_declared),
            ExitCodeCoverage::None { total: 2 }
        ));

        let none = serde_json::json!({"name": "mytool"});
        assert!(matches!(
            exit_code_coverage(&none),
            ExitCodeCoverage::NoErrors
        ));
    }

    #[test]
    fn mutating_coverage_counts_leaves_recursively() {
        let doc = serde_json::json!({"commands": [
            {"name": "list", "mutating": false},
            {"name": "apps", "subcommands": [
                {"name": "deploy", "mutating": true},
                {"name": "status"}
            ]}
        ]});
        assert!(matches!(
            mutating_coverage(&doc),
            MutatingCoverage::Partial {
                missing: 1,
                total: 3
            }
        ));

        let full = serde_json::json!({"commands": [
            {"name": "list", "mutating": false}
        ]});
        assert!(matches!(mutating_coverage(&full), MutatingCoverage::Full));

        let none = serde_json::json!({"name": "mytool"});
        assert!(matches!(
            mutating_coverage(&none),
            MutatingCoverage::NoCommands
        ));
    }
}
