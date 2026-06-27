use std::io::Read;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub struct RunResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Safety ceiling for a single probe. Generous on purpose: it exists only to
/// stop a genuinely hung tool from hanging the scorer, not to penalize a tool
/// that is merely slow (e.g. one that does a network lookup before erroring).
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(30);

pub fn run(binary: &str, args: &[&str], timeout: Duration) -> RunResult {
    run_with_env(binary, args, timeout, &[])
}

/// Forcibly terminate a probed child and reap it. On Unix the child runs in its
/// own process group (see `process_group(0)` at spawn), so we signal the whole
/// group: a probed tool that forked grandchildren (e.g. a shell running
/// `sleep`) would otherwise keep the stdout/stderr pipes open and block the
/// reader threads past the timeout.
fn kill_and_reap(child: &mut Child) {
    #[cfg(unix)]
    {
        let pgid = child.id() as i32;
        // SAFETY: kill(2) with a negative pid signals the process group; an
        // invalid/exited group is a harmless ESRCH.
        unsafe {
            libc::kill(-pgid, libc::SIGKILL);
        }
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Run a binary with additional environment variable overrides.
/// Used to probe behavior in a sanitized environment (e.g. `schema` with
/// HOME pointing at an empty directory to prove it needs no config).
pub fn run_with_env(
    binary: &str,
    args: &[&str],
    timeout: Duration,
    envs: &[(&str, &str)],
) -> RunResult {
    let mut command = Command::new(binary);
    command
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    // Put the child in its own process group so a timeout can signal the whole
    // group, taking any grandchildren (and the pipes they hold) down with it.
    #[cfg(unix)]
    command.process_group(0);
    for (key, value) in envs {
        command.env(key, value);
    }

    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            return RunResult {
                stdout: String::new(),
                stderr: format!("Failed to execute {binary}: {e}"),
                exit_code: -1,
            };
        }
    };

    // Drain stdout/stderr on threads so a chatty child can't deadlock on a full
    // pipe buffer while we poll for completion.
    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();
    let stdout_handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = stdout_pipe.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        buf
    });
    let stderr_handle = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = stderr_pipe.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        buf
    });

    // Wait up to `timeout`, killing the child if it overruns. A hanging probed
    // tool must never hang the scorer. Output captured before the kill is kept
    // as-is (no synthetic message that would clobber a tool's last-line error
    // envelope); the -1 exit code signals the abnormal termination.
    let start = Instant::now();
    let exit_code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    kill_and_reap(&mut child);
                    break -1;
                }
                std::thread::sleep(Duration::from_millis(15));
            }
            // try_wait errored (e.g. the child was already reaped). Make sure
            // the child is gone and its pipes are closed so the reader threads
            // reach EOF — otherwise the join() below would block forever.
            Err(_) => {
                kill_and_reap(&mut child);
                break -1;
            }
        }
    };

    let stdout = String::from_utf8_lossy(&stdout_handle.join().unwrap_or_default()).to_string();
    let stderr = String::from_utf8_lossy(&stderr_handle.join().unwrap_or_default()).to_string();

    RunResult {
        stdout,
        stderr,
        exit_code,
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

    #[test]
    fn run_captures_stderr_of_fast_failing_command() {
        let r = run(
            "sh",
            &["-c", "printf '{\"error\":{\"kind\":\"x\"}}' >&2; exit 2"],
            Duration::from_secs(5),
        );
        assert_eq!(r.exit_code, 2, "stderr was {:?}", r.stderr);
        assert!(
            r.stderr.contains("\"kind\""),
            "lost fast stderr: {:?}",
            r.stderr
        );
    }

    #[test]
    fn run_kills_on_timeout() {
        // A probed tool that hangs must not hang the scorer: the run returns
        // near the timeout, not after sleep 5. The trailing `echo` keeps the
        // shell from exec-optimizing away, so `sleep` runs as a forked
        // grandchild that inherits the pipes — this only returns promptly if
        // the whole process group is killed (not just the direct child).
        let start = std::time::Instant::now();
        let result = run(
            "sh",
            &["-c", "sleep 5; echo done"],
            Duration::from_millis(300),
        );
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_secs(3),
            "should return near the timeout, took {elapsed:?}"
        );
        assert_ne!(
            result.exit_code, 0,
            "a killed process must not report success"
        );
    }
}
