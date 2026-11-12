//! Runs plugin executables and reads what they print (D15, D27, D28, D45).

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

/// One reading from a plugin, before counters are turned into rates.
#[derive(Debug, Clone, PartialEq)]
pub enum Reading {
    Value(f64),
    Unavailable(String),
}

impl Reading {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Reading::Unavailable(reason.into())
    }
}

/// How one process is started.
#[derive(Debug, Clone)]
pub struct Exec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    /// The `PATH` the process gets (D47).
    pub path_env: OsString,
    pub timeout: Duration,
}

const POLL_INTERVAL: Duration = Duration::from_millis(5);
/// More than any plugin should print; the rest is not read.
const MAX_OUTPUT_BYTES: u64 = 64 * 1024;

/// Runs one drop-in plugin and returns its reading.
pub fn run_plugin(exec: &Exec) -> Reading {
    match run(exec) {
        Ok(stdout) => parse_output(&stdout),
        Err(reason) => Reading::Unavailable(reason),
    }
}

/// Runs the helper once for several metrics. Every metric asked for gets a
/// reading, even when the run as a whole fails.
pub fn run_batch(exec: &Exec, metrics: &[String]) -> HashMap<String, Reading> {
    let exec = Exec {
        args: metrics.to_vec(),
        ..exec.clone()
    };
    let parsed = run(&exec).and_then(|stdout| {
        serde_json::from_str::<Value>(stdout.trim())
            .ok()
            .and_then(|json| json.as_object().cloned())
            .ok_or_else(|| "helper output was not a JSON object".to_string())
    });
    metrics
        .iter()
        .map(|metric| {
            let reading = match &parsed {
                Ok(object) => match object.get(metric) {
                    Some(entry) => parse_reading(entry),
                    None => Reading::unavailable("helper did not report this metric"),
                },
                Err(reason) => Reading::Unavailable(reason.clone()),
            };
            (metric.clone(), reading)
        })
        .collect()
}

/// Parses `{"value": 42.1}` or `{"value": null, "reason": "…"}`.
pub fn parse_output(stdout: &str) -> Reading {
    match serde_json::from_str::<Value>(stdout.trim()) {
        Ok(json) => parse_reading(&json),
        Err(_) => Reading::unavailable("output was not valid JSON"),
    }
}

fn parse_reading(json: &Value) -> Reading {
    match json.get("value") {
        Some(Value::Number(number)) => match number.as_f64() {
            Some(value) if value.is_finite() => Reading::Value(value),
            _ => Reading::unavailable("value was not a finite number"),
        },
        Some(Value::Null) => Reading::unavailable(
            json.get("reason")
                .and_then(Value::as_str)
                .unwrap_or("no reason given"),
        ),
        Some(_) => Reading::unavailable("'value' was not a number or null"),
        None => Reading::unavailable("output had no 'value'"),
    }
}

/// Runs the process to completion and returns what it printed. Fails if it
/// cannot start, exits non-zero, or is still running at the timeout.
fn run(exec: &Exec) -> Result<String, String> {
    let mut command = Command::new(&exec.program);
    command
        .args(&exec.args)
        .env("PATH", &exec.path_env)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(dir) = &exec.working_dir {
        command.current_dir(dir);
    }
    #[cfg(unix)]
    {
        // Its own process group, so a timeout also stops anything it started.
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("could not start: {e}"))?;

    let stdout = child.stdout.take().expect("stdout was piped");
    let reader = thread::spawn(move || {
        let mut output = String::new();
        let _ = stdout.take(MAX_OUTPUT_BYTES).read_to_string(&mut output);
        output
    });

    let deadline = Instant::now() + exec.timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() >= deadline => break None,
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(e) => {
                kill_group(&mut child);
                return Err(format!("could not wait for process: {e}"));
            }
        }
    };
    // Also after a normal exit: a background process it left behind would
    // otherwise hold the output pipe open.
    kill_group(&mut child);
    let output = reader.join().unwrap_or_default();

    match status {
        None => Err(format!("timed out after {} s", exec.timeout.as_secs_f64())),
        Some(status) if !status.success() => Err(match status.code() {
            Some(code) => format!("exited with status {code}"),
            None => "was stopped by a signal".to_string(),
        }),
        Some(_) => Ok(output),
    }
}

fn kill_group(child: &mut Child) {
    #[cfg(unix)]
    // SAFETY: killpg has no memory-safety preconditions. The child was
    // started as the leader of its own group, so its pid is the group id.
    unsafe {
        libc::killpg(child.id() as libc::pid_t, libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(all(test, unix))]
pub(crate) mod test_support {
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    /// Writes an executable shell script and returns its path.
    pub fn script(dir: &Path, name: &str, body: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::test_support::script;
    use super::*;

    fn exec(program: PathBuf, timeout_ms: u64) -> Exec {
        Exec {
            program,
            args: Vec::new(),
            working_dir: None,
            path_env: std::env::var_os("PATH").unwrap_or_default(),
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    fn unavailable_reason(reading: Reading) -> String {
        match reading {
            Reading::Unavailable(reason) => reason,
            Reading::Value(v) => panic!("expected unavailable, got {v}"),
        }
    }

    #[test]
    fn parses_a_value() {
        assert_eq!(parse_output("{\"value\": 42.1}\n"), Reading::Value(42.1));
        assert_eq!(parse_output("{\"value\": 7}"), Reading::Value(7.0));
    }

    #[test]
    fn parses_unavailable_with_reason() {
        assert_eq!(
            parse_output(r#"{"value": null, "reason": "sensor not found"}"#),
            Reading::unavailable("sensor not found")
        );
    }

    #[test]
    fn invalid_output_is_unavailable() {
        for output in ["", "42.1", "not json", "{}", r#"{"value": "high"}"#, "[1]"] {
            assert!(
                matches!(parse_output(output), Reading::Unavailable(_)),
                "{output:?} should be unavailable"
            );
        }
    }

    #[test]
    fn plugin_value_is_read_from_stdout() {
        let dir = tempfile::tempdir().unwrap();
        let program = script(dir.path(), "run", r#"echo '{"value": 3.5}'"#);
        assert_eq!(run_plugin(&exec(program, 5000)), Reading::Value(3.5));
    }

    #[test]
    fn plugin_runs_in_its_working_dir_with_the_given_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("number"), "9").unwrap();
        let program = script(
            dir.path(),
            "run",
            r#"[ "$PATH" = "/bin:/usr/bin:/custom" ] && echo "{\"value\": $(cat number)}""#,
        );
        let exec = Exec {
            working_dir: Some(dir.path().to_path_buf()),
            path_env: "/bin:/usr/bin:/custom".into(),
            ..exec(program, 5000)
        };
        assert_eq!(run_plugin(&exec), Reading::Value(9.0));
    }

    #[test]
    fn non_zero_exit_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let program = script(dir.path(), "run", "echo '{\"value\": 1}'; exit 3");
        let reason = unavailable_reason(run_plugin(&exec(program, 5000)));
        assert!(reason.contains("status 3"), "{reason}");
    }

    #[test]
    fn missing_program_is_unavailable() {
        let reason = unavailable_reason(run_plugin(&exec("/nonexistent/run".into(), 5000)));
        assert!(reason.contains("could not start"), "{reason}");
    }

    #[test]
    fn slow_plugin_is_killed_at_the_timeout() {
        let dir = tempfile::tempdir().unwrap();
        // `sleep` runs as a child of the shell; both must be stopped.
        let program = script(dir.path(), "run", "sleep 30\necho '{\"value\": 1}'");
        let started = Instant::now();
        let reason = unavailable_reason(run_plugin(&exec(program, 300)));
        assert!(reason.contains("timed out"), "{reason}");
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn leftover_background_process_does_not_block() {
        let dir = tempfile::tempdir().unwrap();
        let program = script(dir.path(), "run", "sleep 30 &\necho '{\"value\": 2}'");
        let started = Instant::now();
        assert_eq!(run_plugin(&exec(program, 5000)), Reading::Value(2.0));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn batch_splits_the_keyed_object() {
        let dir = tempfile::tempdir().unwrap();
        let program = script(
            dir.path(),
            "helper",
            r#"[ "$*" = "cpu gpu temp" ] && echo '{"cpu": {"value": 10}, "gpu": {"value": null, "reason": "no gpu"}}'"#,
        );
        let metrics = ["cpu", "gpu", "temp"].map(String::from);
        let readings = run_batch(&exec(program, 5000), &metrics);
        assert_eq!(readings["cpu"], Reading::Value(10.0));
        assert_eq!(readings["gpu"], Reading::unavailable("no gpu"));
        assert!(matches!(readings["temp"], Reading::Unavailable(_)));
    }

    #[test]
    fn failed_batch_makes_every_metric_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let program = script(dir.path(), "helper", "exit 2");
        let metrics = ["cpu", "gpu"].map(String::from);
        let readings = run_batch(&exec(program, 5000), &metrics);
        assert_eq!(readings.len(), 2);
        assert!(
            readings
                .values()
                .all(|r| matches!(r, Reading::Unavailable(_)))
        );
    }
}
