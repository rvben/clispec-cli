use std::process::Command;
use std::time::Duration;

pub struct RunResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub fn run(binary: &str, args: &[&str], _timeout: Duration) -> RunResult {
    let child = Command::new(binary)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let child = match child {
        Ok(c) => c,
        Err(e) => {
            return RunResult {
                stdout: String::new(),
                stderr: format!("Failed to execute {binary}: {e}"),
                exit_code: -1,
            };
        }
    };

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            return RunResult {
                stdout: String::new(),
                stderr: format!("Failed to wait for {binary}: {e}"),
                exit_code: -1,
            };
        }
    };

    RunResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_echo() {
        let result = run("echo", &["hello"], Duration::from_secs(5));
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout.trim(), "hello");
    }

    #[test]
    fn run_nonexistent_binary() {
        let result = run("nonexistent_binary_xyz", &[], Duration::from_secs(5));
        assert_eq!(result.exit_code, -1);
        assert!(!result.stderr.is_empty());
    }
}
