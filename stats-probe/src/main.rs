//! Reports built-in metrics for Excalibur.
//!
//! `stats-probe cpu net-down` prints one JSON object keyed by metric name,
//! each entry in the plugin output shape (D27, D45), then exits.

use std::process::ExitCode;

use serde_json::{Map, Value};

use metrics::{NAMES, Probe};

#[cfg(target_os = "macos")]
mod macos;
mod metrics;
mod reading;
#[cfg(not(target_os = "macos"))]
mod unsupported;

const EXIT_USAGE: u8 = 2;

fn main() -> ExitCode {
    let names: Vec<String> = std::env::args().skip(1).collect();
    if names.is_empty() {
        eprintln!("usage: stats-probe <metric>...");
        eprintln!("metrics: {}", NAMES.join(", "));
        return ExitCode::from(EXIT_USAGE);
    }
    match collect(&names) {
        Ok(readings) => {
            println!("{}", Value::Object(readings));
            ExitCode::SUCCESS
        }
        Err(unknown) => {
            eprintln!("stats-probe: unknown metric '{unknown}'");
            eprintln!("metrics: {}", NAMES.join(", "));
            ExitCode::from(EXIT_USAGE)
        }
    }
}

/// Fails with the first unknown name, before any metric is read.
fn collect(names: &[String]) -> Result<Map<String, Value>, &str> {
    if let Some(unknown) = names.iter().find(|n| !NAMES.contains(&n.as_str())) {
        return Err(unknown);
    }
    let probe = Probe::default();
    Ok(names
        .iter()
        .filter_map(|name| Some((name.clone(), probe.read(name)?.to_json())))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn collect_returns_one_entry_per_name() {
        let readings = collect(&names(&["storage", "memory"])).unwrap();
        assert_eq!(readings.len(), 2);
        assert!(readings.contains_key("storage") && readings.contains_key("memory"));
    }

    #[test]
    fn collect_rejects_an_unknown_name_among_known_ones() {
        assert_eq!(collect(&names(&["cpu", "battery"])), Err("battery"));
    }

    #[test]
    fn repeated_names_are_reported_once() {
        assert_eq!(collect(&names(&["cpu", "cpu"])).unwrap().len(), 1);
    }
}
