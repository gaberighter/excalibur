//! `config.toml`, the user's settings (D35, D47). Every setting is optional.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const FILE_NAME: &str = "config.toml";

const MIN_INTERVAL_SECS: f64 = 0.1;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub theme: String,
    /// The colour of sparklines and arcs: a preset's name, or a colour such
    /// as `#ff8800`, `system` or `theme`. Without it the accent follows the
    /// system (D57, D59).
    pub accent: Option<String>,
    /// Default seconds between refreshes (D18).
    pub interval: f64,
    /// Seconds before a running plugin is killed (D28).
    pub timeout: f64,
    /// Order of plugins in the window; unlisted plugins follow alphabetically.
    pub plugins: Vec<String>,
    pub disabled: Vec<String>,
    /// Per-plugin refresh intervals, overriding the manifest's.
    pub intervals: BTreeMap<String, f64>,
    /// Extra folders searched for plugin commands (D47).
    pub path: Vec<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: "frosted".into(),
            accent: None,
            interval: 2.0,
            timeout: 5.0,
            plugins: [
                "cpu",
                "memory",
                "temperature",
                "gpu",
                "net-down",
                "net-up",
                "storage",
            ]
            .map(String::from)
            .to_vec(),
            disabled: Vec::new(),
            intervals: BTreeMap::new(),
            path: Vec::new(),
        }
    }
}

impl Config {
    pub fn parse(text: &str) -> Result<Config, String> {
        let config: Config = toml::from_str(text).map_err(|e| e.message().to_string())?;
        let intervals = config
            .intervals
            .iter()
            .map(|(name, secs)| (name.as_str(), *secs));
        for (name, secs) in std::iter::once(("interval", config.interval)).chain(intervals) {
            if !(secs.is_finite() && secs >= MIN_INTERVAL_SECS) {
                return Err(format!(
                    "interval '{name}' must be at least {MIN_INTERVAL_SECS} seconds"
                ));
            }
        }
        if !(config.timeout.is_finite() && config.timeout > 0.0) {
            return Err("'timeout' must be greater than zero".into());
        }
        if let Some(accent) = &config.accent
            && !crate::accent::is_valid(accent)
        {
            let presets: Vec<&str> = crate::accent::PRESETS.iter().map(|p| p.id).collect();
            return Err(format!(
                "'accent' must be a colour such as \"#ff8800\" or one of: {}",
                presets.join(", ")
            ));
        }
        Ok(config)
    }

    /// A missing file gives the defaults. An invalid one also gives the
    /// defaults, with a warning to show the user.
    pub fn load(file: &Path) -> (Config, Option<String>) {
        let text = match std::fs::read_to_string(file) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Config::default(), None),
            Err(e) => {
                let warning = format!("{}: {e}; using default settings", file.display());
                return (Config::default(), Some(warning));
            }
        };
        match Config::parse(&text) {
            Ok(config) => (config, None),
            Err(e) => {
                let warning = format!("{}: {e}; using default settings", file.display());
                (Config::default(), Some(warning))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_gives_defaults() {
        assert_eq!(Config::parse("").unwrap(), Config::default());
    }

    #[test]
    fn defaults_match_the_documented_ones() {
        let config = Config::default();
        assert_eq!(config.theme, "frosted");
        assert_eq!(config.interval, 2.0);
        assert_eq!(config.timeout, 5.0);
        assert_eq!(config.plugins.first().map(String::as_str), Some("cpu"));
        assert_eq!(config.plugins.last().map(String::as_str), Some("storage"));
    }

    #[test]
    fn settings_override_defaults() {
        let config = Config::parse(
            r#"
            theme    = "plain"
            interval = 1.5
            disabled = ["gpu"]
            path     = ["/opt/tools/bin"]

            [intervals]
            storage = 30.0
            "#,
        )
        .unwrap();
        assert_eq!(config.theme, "plain");
        assert_eq!(config.interval, 1.5);
        assert_eq!(config.timeout, 5.0);
        assert_eq!(config.disabled, ["gpu"]);
        assert_eq!(config.intervals["storage"], 30.0);
        assert_eq!(config.path, [PathBuf::from("/opt/tools/bin")]);
    }

    #[test]
    fn invalid_values_are_rejected() {
        assert!(Config::parse("interval = 0").is_err());
        assert!(Config::parse("timeout = -1").is_err());
        assert!(Config::parse("[intervals]\ncpu = 0.01").is_err());
        assert!(Config::parse("colour = \"red\"").is_err());
        assert!(Config::parse("accent = \"mauve\"").is_err());
    }

    #[test]
    fn accent_can_be_a_preset_or_a_colour() {
        assert_eq!(Config::default().accent, None);
        let preset = Config::parse("accent = \"orange\"").unwrap();
        assert_eq!(preset.accent.as_deref(), Some("orange"));
        let colour = Config::parse("accent = \"#ff8800\"").unwrap();
        assert_eq!(colour.accent.as_deref(), Some("#ff8800"));
    }

    #[test]
    fn missing_file_gives_defaults_without_warning() {
        let dir = tempfile::tempdir().unwrap();
        let (config, warning) = Config::load(&dir.path().join(FILE_NAME));
        assert_eq!(config, Config::default());
        assert_eq!(warning, None);
    }

    #[test]
    fn invalid_file_gives_defaults_with_warning() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(FILE_NAME);
        std::fs::write(&file, "interval = \"fast\"").unwrap();
        let (config, warning) = Config::load(&file);
        assert_eq!(config, Config::default());
        assert!(warning.unwrap().contains("using default settings"));
    }
}
