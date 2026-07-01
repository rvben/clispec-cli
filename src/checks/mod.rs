pub mod bounded;
pub mod idempotent;
pub mod interactive;
pub mod output;
pub mod schema;
pub mod streams;

use serde::Serialize;

/// The v0.2 conformance checklist (clispec.dev/#conformance), one entry per
/// checklist item. Every scored check MUST map to exactly one of these ids;
/// a check that cites no checklist item has no basis in the spec and must
/// not award or deduct points. The summaries are abbreviations for review,
/// not normative text - the published spec is authoritative.
pub const CHECKLIST_ITEMS: [(&str, &str); 10] = [
    (
        "structured-output",
        "Structured output when piped; explicit format flag wins over TTY detection",
    ),
    (
        "error-envelope",
        "On failure, exits non-zero with the error envelope as the last line of stderr",
    ),
    (
        "schema-validates",
        "Exposes a schema subcommand whose output validates against clispec.dev/schema/v0.2.json",
    ),
    (
        "schema-offline",
        "schema succeeds with no authentication, no configuration file, and no network",
    ),
    (
        "help-mentions-schema",
        "Root --help output mentions the schema subcommand",
    ),
    (
        "stream-separation",
        "Data to stdout and diagnostics to stderr in every output mode",
    ),
    (
        "non-interactive",
        "Runs to completion without a TTY; flag alternative for every interactive prompt",
    ),
    (
        "confirmation-refusal",
        "Commands that would prompt refuse without a TTY via confirmation_required",
    ),
    (
        "idempotent-repeats",
        "Re-running a satisfied command exits zero; incompatible repeats emit conflict",
    ),
    (
        "bounded-lists",
        "List commands support --limit/--offset and --fields with in-band truncation metadata",
    ),
];

/// The checklist item a named check verifies. Returns `None` for unknown
/// check names; the unit tests below reject any check without a mapping.
pub fn checklist_item(check_name: &str) -> Option<&'static str> {
    let id = match check_name {
        "JSON output flag"
        | "Valid JSON output"
        | "Structured or declared piped output"
        | "Explicit format wins" => "structured-output",
        "Structured errors" => "error-envelope",
        "schema command exists"
        | "Valid JSON schema"
        | "Validates against clispec v0.2"
        | "Error kinds documented"
        | "Output fields declared"
        | "Global args declared"
        | "Exit codes on error kinds"
        | "Mutation markers on all commands" => "schema-validates",
        "schema works without config" => "schema-offline",
        "schema mentioned in --help" => "help-mentions-schema",
        "Clean stdout when piped" | "Messages on stderr only" => "stream-separation",
        "No TTY hang" => "non-interactive",
        "--yes flag" => "confirmation-refusal",
        "Mutating markers in schema" | "Conflict error kind" => "idempotent-repeats",
        "--limit flag" | "Pagination flag" | "--fields flag" => "bounded-lists",
        _ => return None,
    };
    Some(id)
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub detail: Option<String>,
    /// Conformance checklist item this check verifies (see CHECKLIST_ITEMS).
    pub checklist: Option<&'static str>,
}

impl CheckResult {
    pub fn pass(name: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: true,
            detail: None,
            checklist: checklist_item(name),
        }
    }

    pub fn pass_with(name: &str, detail: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: true,
            detail: Some(detail.to_string()),
            checklist: checklist_item(name),
        }
    }

    pub fn fail(name: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: false,
            detail: None,
            checklist: checklist_item(name),
        }
    }

    pub fn fail_with(name: &str, detail: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: false,
            detail: Some(detail.to_string()),
            checklist: checklist_item(name),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PrincipleScore {
    pub name: String,
    pub score: u32,
    pub max: u32,
    pub checks: Vec<CheckResult>,
}

impl PrincipleScore {
    pub fn new(name: &str, checks: Vec<CheckResult>, max: u32) -> Self {
        let score = checks.iter().filter(|c| c.passed).count() as u32;
        Self {
            name: name.to_string(),
            score,
            max,
            checks,
        }
    }
}

pub struct CheckContext {
    pub binary: String,
    pub subcommand: Vec<String>,
    pub help_text: String,
    pub schema_json: Option<serde_json::Value>,
}

/// A self-contained invocation for exercising the representative command: base
/// args plus optional stdin. Lets a tool whose command reads a path or stdin be
/// probed for its structured output, instead of the scorer guessing that the
/// command name works as a positional.
pub struct Probe {
    pub args: Vec<String>,
    pub stdin: Option<String>,
}

impl CheckContext {
    /// How to invoke the representative command for output/stream probes. Prefers
    /// the command's declared `example` (args + stdin); otherwise falls back to
    /// running the discovered subcommand name directly.
    pub fn probe(&self) -> Probe {
        self.representative_example().unwrap_or_else(|| Probe {
            args: self.subcommand.clone(),
            stdin: None,
        })
    }

    fn representative_example(&self) -> Option<Probe> {
        let commands = self.schema_json.as_ref()?.get("commands")?.as_array()?;
        let example = find_command(commands, &self.subcommand.join(" "))?.get("example")?;
        let args = example
            .get("args")
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let stdin = example
            .get("stdin")
            .and_then(|s| s.as_str())
            .map(String::from);
        Some(Probe { args, stdin })
    }
}

/// Find a command by its space-joined path ("scan" or "apps list"), recursing
/// into nested `subcommands`.
fn find_command<'a>(
    commands: &'a [serde_json::Value],
    path: &str,
) -> Option<&'a serde_json::Value> {
    for cmd in commands {
        let Some(name) = cmd.get("name").and_then(|n| n.as_str()) else {
            continue;
        };
        if name == path {
            return Some(cmd);
        }
        if let Some(rest) = path.strip_prefix(name).map(str::trim_start)
            && !rest.is_empty()
            && let Some(subs) = cmd.get("subcommands").and_then(|s| s.as_array())
            && let Some(found) = find_command(subs, rest)
        {
            return Some(found);
        }
    }
    None
}

/// Run `binary subcommand... --help` and parse the help output.
/// Returns `None` if the subcommand is empty or the command fails.
pub fn subcommand_help_info(ctx: &CheckContext) -> Option<crate::help::HelpInfo> {
    if ctx.subcommand.is_empty() {
        return None;
    }
    let mut args: Vec<&str> = ctx.subcommand.iter().map(|s| s.as_str()).collect();
    args.push("--help");
    let result = crate::runner::run(&ctx.binary, &args, crate::runner::PROBE_TIMEOUT);
    if result.exit_code < 0 {
        return None;
    }
    Some(crate::help::parse_help(&result.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_context() -> CheckContext {
        CheckContext {
            binary: "echo".to_string(),
            subcommand: vec![],
            help_text: String::new(),
            schema_json: None,
        }
    }

    #[test]
    fn probe_uses_declared_example() {
        let ctx = CheckContext {
            binary: "x".to_string(),
            subcommand: vec!["scan".to_string()],
            help_text: String::new(),
            schema_json: Some(serde_json::json!({
                "commands": [{"name": "scan", "example": {"args": ["-"], "stdin": "hi"}}]
            })),
        };
        let probe = ctx.probe();
        assert_eq!(probe.args, vec!["-".to_string()]);
        assert_eq!(probe.stdin.as_deref(), Some("hi"));
    }

    #[test]
    fn probe_falls_back_to_subcommand_without_example() {
        let ctx = CheckContext {
            binary: "x".to_string(),
            subcommand: vec!["report".to_string()],
            help_text: String::new(),
            schema_json: Some(serde_json::json!({"commands": [{"name": "report"}]})),
        };
        let probe = ctx.probe();
        assert_eq!(probe.args, vec!["report".to_string()]);
        assert!(probe.stdin.is_none());
    }

    #[test]
    fn find_command_recurses_into_subcommands() {
        let commands = serde_json::json!([
            {"name": "apps", "subcommands": [{"name": "list", "example": {"args": ["x"]}}]}
        ]);
        let found = find_command(commands.as_array().unwrap(), "apps list");
        assert!(found.is_some_and(|c| c.get("example").is_some()));
    }

    #[test]
    fn check_result_constructors() {
        let pass = CheckResult::pass("test");
        assert!(pass.passed);
        assert!(pass.detail.is_none());

        let fail = CheckResult::fail("test");
        assert!(!fail.passed);

        let fail_detail = CheckResult::fail_with("test", "reason");
        assert!(!fail_detail.passed);
        assert_eq!(fail_detail.detail.as_deref(), Some("reason"));
    }

    #[test]
    fn principle_score_counts_passes() {
        let checks = vec![
            CheckResult::pass("a"),
            CheckResult::fail("b"),
            CheckResult::pass("c"),
        ];
        let score = PrincipleScore::new("test", checks, 3);
        assert_eq!(score.score, 2);
        assert_eq!(score.max, 3);
    }

    #[test]
    fn checks_return_correct_max_scores() {
        let ctx = test_context();
        assert_eq!(output::check(&ctx).max, 5);
        assert_eq!(schema::check(&ctx).max, 10);
        assert_eq!(streams::check(&ctx).max, 2);
        assert_eq!(interactive::check(&ctx).max, 2);
        assert_eq!(idempotent::check(&ctx).max, 2);
        assert_eq!(bounded::check(&ctx).max, 3);
    }

    #[test]
    fn checks_produce_expected_number_of_results() {
        let ctx = test_context();
        assert_eq!(output::check(&ctx).checks.len(), 5);
        assert_eq!(schema::check(&ctx).checks.len(), 10);
        assert_eq!(streams::check(&ctx).checks.len(), 2);
        assert_eq!(interactive::check(&ctx).checks.len(), 2);
        assert_eq!(idempotent::check(&ctx).checks.len(), 2);
        assert_eq!(bounded::check(&ctx).checks.len(), 3);
    }

    fn all_check_results() -> Vec<CheckResult> {
        let ctx = test_context();
        [
            output::check(&ctx),
            schema::check(&ctx),
            streams::check(&ctx),
            interactive::check(&ctx),
            idempotent::check(&ctx),
            bounded::check(&ctx),
        ]
        .into_iter()
        .flat_map(|p| p.checks)
        .collect()
    }

    #[test]
    fn every_check_cites_a_checklist_item() {
        for check in all_check_results() {
            assert!(
                check.checklist.is_some(),
                "check '{}' cites no conformance checklist item; checks without \
                 a basis in the published spec must not be scored",
                check.name
            );
        }
    }

    #[test]
    fn every_checklist_item_is_verified_by_a_check() {
        let cited: std::collections::HashSet<&str> = all_check_results()
            .iter()
            .filter_map(|c| c.checklist)
            .collect();
        for (id, summary) in CHECKLIST_ITEMS {
            assert!(
                cited.contains(id),
                "checklist item '{id}' ({summary}) has no check verifying it"
            );
        }
    }

    #[test]
    fn checklist_mapping_targets_are_canonical() {
        let ids: std::collections::HashSet<&str> =
            CHECKLIST_ITEMS.iter().map(|(id, _)| *id).collect();
        for check in all_check_results() {
            if let Some(item) = check.checklist {
                assert!(
                    ids.contains(item),
                    "check '{}' cites unknown checklist item '{item}'",
                    check.name
                );
            }
        }
    }
}
