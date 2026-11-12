//! Runs the built binary the way the app does.

use std::process::{Command, Output};
use std::thread::sleep;
use std::time::Duration;

use serde_json::Value;

const NAMES: [&str; 7] = [
    "cpu",
    "memory",
    "storage",
    "temperature",
    "gpu",
    "net-down",
    "net-up",
];
const COUNTERS: [&str; 3] = ["cpu", "net-down", "net-up"];

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_stats-probe"))
        .args(args)
        .output()
        .expect("stats-probe should run")
}

fn run_json(args: &[&str]) -> Value {
    let output = run(args);
    assert!(output.status.success(), "{args:?} should exit 0");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "output should be one line");
    serde_json::from_str(&stdout).expect("output should be JSON")
}

/// A reading is `{"value": <number>}`, optionally with a `"max"`, or
/// `{"value": null, "reason": <text>}`. Sensors differ between machines, so
/// either is accepted.
fn assert_reading_shape(name: &str, reading: &Value) {
    let object = reading
        .as_object()
        .unwrap_or_else(|| panic!("{name} should be an object"));
    match &object["value"] {
        Value::Number(_) => {
            let extra: Vec<&String> = object.keys().filter(|k| *k != "value").collect();
            assert!(
                extra.iter().all(|k| *k == "max"),
                "{name}: unexpected keys {extra:?}"
            );
            assert!(
                object.get("max").is_none_or(Value::is_number),
                "{name}: max should be a number"
            );
        }
        Value::Null => assert!(object["reason"].is_string(), "{name}: null needs a reason"),
        other => panic!("{name}: value should be a number or null, got {other}"),
    }
}

#[test]
fn each_metric_alone_is_keyed_by_its_name() {
    for name in NAMES {
        let json = run_json(&[name]);
        let object = json.as_object().unwrap();
        assert_eq!(object.len(), 1);
        assert_reading_shape(name, &object[name]);
    }
}

#[test]
fn a_batch_returns_every_metric_asked_for() {
    let json = run_json(&NAMES);
    let object = json.as_object().unwrap();
    assert_eq!(object.len(), NAMES.len());
    for name in NAMES {
        assert_reading_shape(name, &object[name]);
    }
}

#[test]
fn counters_do_not_decrease_between_runs() {
    let first = run_json(&COUNTERS);
    sleep(Duration::from_millis(200));
    let second = run_json(&COUNTERS);
    for name in COUNTERS {
        if let (Some(a), Some(b)) = (
            first[name]["value"].as_f64(),
            second[name]["value"].as_f64(),
        ) {
            assert!(b >= a, "{name} went backwards: {a} -> {b}");
        }
    }
}

#[test]
fn an_unknown_metric_fails_without_output() {
    let output = run(&["cpu", "battery"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("battery"));
}

#[test]
fn no_arguments_prints_usage_and_fails() {
    let output = run(&[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("usage"));
}

#[cfg(target_os = "macos")]
#[test]
fn core_metrics_are_available_on_macos() {
    let json = run_json(&["cpu", "memory", "storage"]);
    for name in ["cpu", "memory", "storage"] {
        let value = json[name]["value"].as_f64();
        assert!(value.is_some_and(|v| v > 0.0), "{name} should be positive");
    }
    // Memory and storage report what they are out of.
    for name in ["memory", "storage"] {
        let (value, max) = (json[name]["value"].as_f64(), json[name]["max"].as_f64());
        assert!(
            max >= value,
            "{name}: max {max:?} should be at least {value:?}"
        );
    }
}
