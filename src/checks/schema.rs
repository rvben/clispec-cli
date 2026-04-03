use crate::runner;
use std::time::Duration;

use super::{CheckContext, CheckResult, PrincipleScore};

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
        // Check 3: Has commands field
        checks.push(if s.get("commands").is_some() {
            CheckResult::pass("Commands documented")
        } else {
            CheckResult::fail("Commands documented")
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
        checks.push(CheckResult::fail("Commands documented"));
        checks.push(CheckResult::fail("Error kinds documented"));
        checks.push(CheckResult::fail("Output fields declared"));
    }

    PrincipleScore::new("Schema Introspection", checks, 5)
}
