use crate::runner;

use super::{CheckContext, CheckResult, PrincipleScore};

/// The canonical clispec v0.3 candidate schema, vendored from
/// clispec.dev/schema/v0.3.json.
const CLISPEC_SCHEMA_V0_3: &str = include_str!("../../schemas/v0.3.json");

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
        // Check 3: Validates against the clispec v0.3 JSON Schema
        checks.push(match validate_against_clispec_v0_3(s) {
            Ok(()) => CheckResult::pass("Validates against clispec v0.3"),
            Err(detail) => CheckResult::fail_with("Validates against clispec v0.3", &detail),
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

        // Check 5: Every structured data command describes stdout. v0.3
        // permits either the compact output_fields form or stdout_schema.
        let has_output_fields =
            s.get("commands")
                .and_then(|c| c.as_array())
                .is_some_and(|commands| {
                    commands.iter().all(|command| {
                        command.get("output_kind").and_then(|v| v.as_str()) != Some("data")
                            && command.get("output_kind").is_some()
                            || command.get("output_fields").is_some()
                            || command.get("stdout_schema").is_some()
                    })
                });
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

        // Check 8: Every command carries the required v0.3 effects declaration.
        checks.push(match effects_coverage(s) {
            EffectsCoverage::Full => CheckResult::pass("Effects on all commands"),
            EffectsCoverage::NoCommands => {
                CheckResult::fail_with("Effects on all commands", "no commands declared")
            }
            EffectsCoverage::Partial { missing, total } => CheckResult::fail_with(
                "Effects on all commands",
                &format!("{missing} of {total} commands missing effects"),
            ),
        });
    } else {
        checks.push(CheckResult::fail("Validates against clispec v0.3"));
        checks.push(CheckResult::fail("Error kinds documented"));
        checks.push(CheckResult::fail("Output fields declared"));
        checks.push(CheckResult::fail("Global args declared"));
        checks.push(CheckResult::fail("Exit codes on error kinds"));
        checks.push(CheckResult::fail("Effects on all commands"));
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

/// Validate an instance against the bundled clispec v0.3 JSON Schema.
/// Returns Ok on success, or Err with the first validation error message.
fn validate_against_clispec_v0_3(instance: &serde_json::Value) -> Result<(), String> {
    let schema: serde_json::Value = serde_json::from_str(CLISPEC_SCHEMA_V0_3)
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
        .filter(|e| {
            e.get("exit_code").is_some_and(|c| c.is_i64() || c.is_u64())
                || e.get("exit_code_passthrough").and_then(|v| v.as_bool()) == Some(true)
        })
        .count() as u32;
    if declared == total {
        ExitCodeCoverage::Full
    } else if declared == 0 {
        ExitCodeCoverage::None { total }
    } else {
        ExitCodeCoverage::Partial { declared, total }
    }
}

enum EffectsCoverage {
    Full,
    NoCommands,
    Partial { missing: u32, total: u32 },
}

fn effects_coverage(schema: &serde_json::Value) -> EffectsCoverage {
    fn walk(cmd: &serde_json::Value, total: &mut u32, missing: &mut u32) {
        *total += 1;
        if !cmd
            .get("effects")
            .and_then(|e| e.as_str())
            .is_some_and(|e| matches!(e, "read_only" | "idempotent" | "non_idempotent"))
        {
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
        EffectsCoverage::NoCommands
    } else if missing == 0 {
        EffectsCoverage::Full
    } else {
        EffectsCoverage::Partial { missing, total }
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
            "clispec": "0.3",
            "name": "mytool",
            "version": "1.0.0",
            "commands": [{
                "name": "list",
                "description": "List services",
                "effects": "read_only",
                "cardinality": "single",
                "stdout_schema": {}
            }],
            "errors": [{
                "kind": "usage",
                "exit_code": 2,
                "retryable": false,
                "description": "Invalid invocation"
            }]
        })
    }

    #[test]
    fn bundled_schema_is_valid_draft_2020_12() {
        let schema: serde_json::Value = serde_json::from_str(CLISPEC_SCHEMA_V0_3).unwrap();
        jsonschema::draft202012::new(&schema).expect("bundled schema must be valid");
    }

    #[test]
    fn minimal_document_validates() {
        validate_against_clispec_v0_3(&minimal_valid()).expect("minimal doc should validate");
    }

    #[test]
    fn v0_2_document_is_rejected() {
        let mut doc = minimal_valid();
        doc["clispec"] = serde_json::json!("0.2");
        validate_against_clispec_v0_3(&doc).expect_err("v0.2 must not validate as v0.3");
    }

    #[test]
    fn missing_required_field_fails() {
        let doc = serde_json::json!({ "name": "mytool", "version": "1.0.0" });
        validate_against_clispec_v0_3(&doc).expect_err("missing commands should fail");
    }

    #[test]
    fn error_kind_must_be_snake_case() {
        let mut doc = minimal_valid();
        doc["errors"] = serde_json::json!([{
            "kind": "Not-Found", "exit_code": 2, "retryable": false
        }]);
        validate_against_clispec_v0_3(&doc).expect_err("non-snake_case kind should fail");
    }

    #[test]
    fn additional_properties_are_permitted() {
        let mut doc = minimal_valid();
        doc["commands"][0]["x_custom"] = serde_json::json!("anything");
        doc["x_tool_metadata"] = serde_json::json!({"vendor": "acme"});
        validate_against_clispec_v0_3(&doc).expect("extensions should validate");
    }

    #[test]
    fn exit_code_coverage_full_partial_none() {
        let full = serde_json::json!({"errors": [
            {"kind": "auth", "exit_code": 3},
            {"kind": "not_found", "exit_code": 4}
        ]});
        assert!(matches!(exit_code_coverage(&full), ExitCodeCoverage::Full));

        let with_passthrough = serde_json::json!({"errors": [
            {"kind": "auth", "exit_code": 3},
            {"kind": "job_failed", "exit_code_passthrough": true}
        ]});
        assert!(matches!(
            exit_code_coverage(&with_passthrough),
            ExitCodeCoverage::Full
        ));

        // The helper still reports partial legacy documents clearly even
        // though v0.3 validation rejects them before this check is scored.
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
    fn effects_coverage_counts_flat_commands() {
        let doc = serde_json::json!({"commands": [
            {"name": "list", "effects": "read_only"},
            {"name": "apps deploy", "effects": "idempotent"},
            {"name": "apps status"}
        ]});
        assert!(matches!(
            effects_coverage(&doc),
            EffectsCoverage::Partial {
                missing: 1,
                total: 3
            }
        ));

        let full = serde_json::json!({"commands": [
            {"name": "list", "effects": "read_only"}
        ]});
        assert!(matches!(effects_coverage(&full), EffectsCoverage::Full));

        let none = serde_json::json!({"name": "mytool"});
        assert!(matches!(
            effects_coverage(&none),
            EffectsCoverage::NoCommands
        ));
    }
}
