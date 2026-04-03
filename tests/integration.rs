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
    assert!(output.status.success());
}
