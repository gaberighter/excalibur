//! `plugin.toml`, the description of one metric (D26, D34, D45, D46).

use serde::{Deserialize, Serialize};

pub const FILE_NAME: &str = "plugin.toml";

const MIN_INTERVAL_SECS: f64 = 0.1;
const MAX_DECIMALS: u8 = 6;
const DEFAULT_DECIMALS: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Display {
    #[default]
    Value,
    Sparkline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// The reading is shown as it is.
    #[default]
    Gauge,
    /// The reading is a running total; its change per second is shown (D16).
    Counter,
}

/// Where a plugin's readings come from.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    /// An executable run once per refresh (D15).
    Command(Vec<String>),
    /// A metric collected in the shared `stats-probe` run (D45).
    Batch(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub name: String,
    pub label: String,
    pub unit: String,
    pub source: Source,
    pub display: Display,
    pub kind: Kind,
    /// Seconds between refreshes, if the plugin suggests one (D18).
    pub interval: Option<f64>,
    pub decimals: u8,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    name: Option<String>,
    label: Option<String>,
    unit: Option<String>,
    command: Option<Vec<String>>,
    batch: Option<String>,
    display: Option<Display>,
    kind: Option<Kind>,
    interval: Option<f64>,
    decimals: Option<u8>,
    min: Option<f64>,
    max: Option<f64>,
}

impl Manifest {
    /// `folder` is the plugin's folder name, used when the manifest has no `name`.
    pub fn parse(text: &str, folder: &str) -> Result<Manifest, String> {
        let raw: Raw = toml::from_str(text).map_err(|e| e.message().to_string())?;

        let name = raw.name.unwrap_or_else(|| folder.to_string());
        if !is_valid_name(&name) {
            return Err(format!(
                "name '{name}' may only contain letters, digits, '-' and '_'"
            ));
        }

        let source = match (raw.command, raw.batch) {
            (Some(command), None) if command.is_empty() => {
                return Err("'command' must not be empty".into());
            }
            (Some(command), None) => Source::Command(command),
            (None, Some(batch)) => Source::Batch(batch),
            (Some(_), Some(_)) => return Err("set 'command' or 'batch', not both".into()),
            (None, None) => return Err("one of 'command' or 'batch' is required".into()),
        };

        if let Some(interval) = raw.interval
            && !(interval.is_finite() && interval >= MIN_INTERVAL_SECS)
        {
            return Err(format!(
                "'interval' must be at least {MIN_INTERVAL_SECS} seconds"
            ));
        }

        let decimals = raw.decimals.unwrap_or(DEFAULT_DECIMALS);
        if decimals > MAX_DECIMALS {
            return Err(format!("'decimals' must be at most {MAX_DECIMALS}"));
        }

        for (field, bound) in [("min", raw.min), ("max", raw.max)] {
            if bound.is_some_and(|b| !b.is_finite()) {
                return Err(format!("'{field}' must be a finite number"));
            }
        }
        if let (Some(min), Some(max)) = (raw.min, raw.max)
            && min >= max
        {
            return Err("'min' must be less than 'max'".into());
        }

        Ok(Manifest {
            label: raw.label.unwrap_or_else(|| name.clone()),
            name,
            unit: raw.unit.unwrap_or_default(),
            source,
            display: raw.display.unwrap_or_default(),
            kind: raw.kind.unwrap_or_default(),
            interval: raw.interval,
            decimals,
            min: raw.min,
            max: raw.max,
        })
    }
}

fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_manifest_parses() {
        let manifest = Manifest::parse(
            r#"
            name     = "cpu"
            label    = "CPU"
            unit     = "%"
            batch    = "cpu"
            display  = "sparkline"
            kind     = "counter"
            interval = 1.0
            decimals = 1
            min      = 0
            max      = 100
            "#,
            "folder",
        )
        .unwrap();
        assert_eq!(manifest.name, "cpu");
        assert_eq!(manifest.label, "CPU");
        assert_eq!(manifest.source, Source::Batch("cpu".into()));
        assert_eq!(manifest.display, Display::Sparkline);
        assert_eq!(manifest.kind, Kind::Counter);
        assert_eq!(manifest.interval, Some(1.0));
        assert_eq!((manifest.min, manifest.max), (Some(0.0), Some(100.0)));
    }

    #[test]
    fn minimal_manifest_gets_defaults() {
        let manifest = Manifest::parse(r#"command = ["run"]"#, "uptime").unwrap();
        assert_eq!(manifest.name, "uptime");
        assert_eq!(manifest.label, "uptime");
        assert_eq!(manifest.unit, "");
        assert_eq!(manifest.source, Source::Command(vec!["run".into()]));
        assert_eq!(manifest.display, Display::Value);
        assert_eq!(manifest.kind, Kind::Gauge);
        assert_eq!(manifest.interval, None);
        assert_eq!(manifest.decimals, 1);
    }

    #[test]
    fn command_and_batch_together_are_rejected() {
        let err = Manifest::parse("command = [\"run\"]\nbatch = \"cpu\"", "x").unwrap_err();
        assert!(err.contains("not both"), "{err}");
    }

    #[test]
    fn neither_command_nor_batch_is_rejected() {
        let err = Manifest::parse("label = \"X\"", "x").unwrap_err();
        assert!(err.contains("required"), "{err}");
    }

    #[test]
    fn empty_command_is_rejected() {
        assert!(Manifest::parse("command = []", "x").is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let err = Manifest::parse("command = [\"run\"]\ncolour = \"red\"", "x").unwrap_err();
        assert!(err.contains("colour"), "{err}");
    }

    #[test]
    fn unknown_display_is_rejected() {
        assert!(Manifest::parse("command = [\"run\"]\ndisplay = \"bar\"", "x").is_err());
    }

    #[test]
    fn bad_bounds_are_rejected() {
        assert!(Manifest::parse("command = [\"run\"]\nmin = 5\nmax = 5", "x").is_err());
        assert!(Manifest::parse("command = [\"run\"]\nmax = inf", "x").is_err());
    }

    #[test]
    fn too_short_interval_is_rejected() {
        assert!(Manifest::parse("command = [\"run\"]\ninterval = 0", "x").is_err());
    }

    #[test]
    fn bad_name_is_rejected() {
        assert!(Manifest::parse("command = [\"run\"]", "my plugin").is_err());
        assert!(Manifest::parse("name = \"a/b\"\ncommand = [\"run\"]", "x").is_err());
    }
}
