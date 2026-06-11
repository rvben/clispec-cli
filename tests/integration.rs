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
