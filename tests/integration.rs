use std::process::Command;

fn clispec() -> Command {
    Command::new(env!("CARGO_BIN_EXE_clispec"))
}

#[test]
fn score_echo_runs() {
    let output = clispec().args(["score", "echo"]).output().unwrap();
    assert!(output.status.success());
}

#[test]
fn score_json_output() {
    let output = clispec()
        .args(["score", "echo", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.get("tool").is_some());
    assert!(json.get("score").is_some());
    assert!(json.get("grade").is_some());
    assert!(json.get("principles").is_some());
}

#[test]
fn schema_is_valid_json() {
    let output = clispec().args(["schema"]).output().unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.get("name").is_some());
    assert!(json.get("commands").is_some());
}

#[test]
fn own_schema_validates_against_clispec_v0_2() {
    let output = clispec().args(["schema"]).output().unwrap();
    assert!(output.status.success());
    let instance: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/v0.2.json")).unwrap();
    let validator = jsonschema::draft202012::new(&schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| format!("{}: {e}", e.instance_path()))
        .collect();
    assert!(errors.is_empty(), "self-schema invalid: {errors:?}");
}

#[test]
fn completions_zsh() {
    let output = clispec().args(["completions", "zsh"]).output().unwrap();
    assert!(output.status.success());
    assert!(!output.stdout.is_empty());
}

#[test]
fn nonexistent_binary() {
    let output = clispec()
        .args(["score", "nonexistent_binary_xyz"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));

    // The error envelope is the last line of stderr, per the spec
    let stderr = String::from_utf8_lossy(&output.stderr);
    let last_line = stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap();
    let json: serde_json::Value = serde_json::from_str(last_line).unwrap();
    assert_eq!(
        json.pointer("/error/kind").and_then(|k| k.as_str()),
        Some("not_found")
    );
}

/// A tool that emits human text by default when piped but DECLARES that default
/// in the schema `output` field (and offers JSON via `-o json`) is compliant per
/// the amended Principle 1. It must pass the structured-output and stream-
/// separation checks that a bare JSON-when-piped probe would have failed.
#[cfg(unix)]
#[test]
fn declared_text_default_tool_is_scored_as_structured() {
    use std::os::unix::fs::PermissionsExt;

    let script = r#"#!/bin/sh
if [ "$1" = "schema" ]; then
  cat <<'JSON'
{"clispec":"0.2","name":"faketxt","version":"0.1.0","output":{"tty":"text","piped":"text"},"global_args":[{"name":"--output","type":"string","enum":["auto","text","json"],"default":"auto"}],"commands":[{"name":"run","mutating":false,"example":{"args":[],"stdin":""}}]}
JSON
  exit 0
fi
if [ "$1" = "--help" ]; then
  echo "Usage: faketxt [--output text|json]. Run 'faketxt schema' for the contract."
  exit 0
fi
for a in "$@"; do
  if [ "$a" = "json" ]; then echo '{"ok":true}'; exit 0; fi
done
echo "plain text line"
"#;
    let path = std::env::temp_dir().join(format!("clispec-faketxt-{}.sh", std::process::id()));
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();

    let output = clispec()
        .args(["score", path.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&path);

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let principles = json["principles"].as_array().unwrap();
    let principle = |name: &str| principles.iter().find(|p| p["name"] == name).unwrap();

    let structured = principle("Structured Output");
    let check3 = structured["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "Structured or declared piped output")
        .expect("check present");
    assert_eq!(
        check3["passed"], true,
        "declared-text tool should pass: {check3}"
    );

    let streams = principle("Stderr/Stdout Separation");
    for check in streams["checks"].as_array().unwrap() {
        assert_eq!(check["passed"], true, "stream check should pass: {check}");
    }
}
