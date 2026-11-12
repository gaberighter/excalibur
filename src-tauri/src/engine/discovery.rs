//! Finds plugins in the app bundle and the user's folder (D21, D35).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::config::Config;
use super::manifest::{self, Manifest};

#[derive(Debug, Clone, PartialEq)]
pub struct Plugin {
    pub manifest: Manifest,
    /// The plugin's folder; commands run with it as their working directory.
    pub dir: PathBuf,
    pub builtin: bool,
    /// The refresh interval after config and manifest are taken into account.
    pub interval: Duration,
}

#[derive(Debug, Default)]
pub struct Discovered {
    /// Enabled plugins, in display order.
    pub plugins: Vec<Plugin>,
    pub warnings: Vec<String>,
}

/// A user plugin replaces a bundled one of the same name.
pub fn discover(bundled: Option<&Path>, user: &Path, config: &Config) -> Discovered {
    let mut warnings = Vec::new();
    let mut by_name = BTreeMap::new();
    let folders = bundled
        .map(|dir| (dir, true))
        .into_iter()
        .chain([(user, false)]);
    for (dir, builtin) in folders {
        for (manifest, plugin_dir) in read_folder(dir, &mut warnings) {
            let interval = effective_interval(&manifest, config);
            let plugin = Plugin {
                manifest,
                dir: plugin_dir,
                builtin,
                interval,
            };
            by_name.insert(plugin.manifest.name.clone(), plugin);
        }
    }

    by_name.retain(|name, _| !config.disabled.contains(name));

    let mut plugins = Vec::with_capacity(by_name.len());
    for name in &config.plugins {
        plugins.extend(by_name.remove(name));
    }
    plugins.extend(by_name.into_values());
    Discovered { plugins, warnings }
}

/// Config override, then the manifest's suggestion, then the global default (D18).
fn effective_interval(manifest: &Manifest, config: &Config) -> Duration {
    let secs = config
        .intervals
        .get(&manifest.name)
        .copied()
        .or(manifest.interval)
        .unwrap_or(config.interval);
    Duration::from_secs_f64(secs)
}

/// Reads every subfolder that has a manifest. A missing folder is not an error.
fn read_folder(dir: &Path, warnings: &mut Vec<String>) -> Vec<(Manifest, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut plugin_dirs: Vec<PathBuf> = entries
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.join(manifest::FILE_NAME).is_file())
        .collect();
    plugin_dirs.sort();

    let mut found: Vec<(Manifest, PathBuf)> = Vec::new();
    for plugin_dir in plugin_dirs {
        let file = plugin_dir.join(manifest::FILE_NAME);
        let folder = plugin_dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parsed = std::fs::read_to_string(&file)
            .map_err(|e| e.to_string())
            .and_then(|text| Manifest::parse(&text, &folder));
        match parsed {
            Ok(manifest) if found.iter().any(|(m, _)| m.name == manifest.name) => {
                warnings.push(format!(
                    "{}: another plugin in this folder is already named '{}'; skipped",
                    file.display(),
                    manifest.name
                ));
            }
            Ok(manifest) => found.push((manifest, plugin_dir)),
            Err(e) => warnings.push(format!("{}: {e}; skipped", file.display())),
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_plugin(root: &Path, folder: &str, manifest: &str) {
        let dir = root.join(folder);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(manifest::FILE_NAME), manifest).unwrap();
    }

    fn names(discovered: &Discovered) -> Vec<&str> {
        discovered
            .plugins
            .iter()
            .map(|p| p.manifest.name.as_str())
            .collect()
    }

    #[test]
    fn missing_folders_give_no_plugins() {
        let root = tempfile::tempdir().unwrap();
        let found = discover(
            Some(&root.path().join("bundled")),
            &root.path().join("user"),
            &Config::default(),
        );
        assert!(found.plugins.is_empty());
        assert!(found.warnings.is_empty());
    }

    #[test]
    fn user_plugin_replaces_builtin_of_the_same_name() {
        let root = tempfile::tempdir().unwrap();
        let (bundled, user) = (root.path().join("bundled"), root.path().join("user"));
        add_plugin(&bundled, "cpu", "batch = \"cpu\"\nlabel = \"CPU\"");
        add_plugin(&user, "cpu", "command = [\"run\"]\nlabel = \"My CPU\"");

        let found = discover(Some(&bundled), &user, &Config::default());
        assert_eq!(found.plugins.len(), 1);
        assert_eq!(found.plugins[0].manifest.label, "My CPU");
        assert!(!found.plugins[0].builtin);
        assert_eq!(found.plugins[0].dir, user.join("cpu"));
    }

    #[test]
    fn disabled_plugins_are_left_out() {
        let root = tempfile::tempdir().unwrap();
        add_plugin(root.path(), "cpu", "batch = \"cpu\"");
        add_plugin(root.path(), "gpu", "batch = \"gpu\"");
        let config = Config {
            disabled: vec!["gpu".into()],
            ..Config::default()
        };
        assert_eq!(names(&discover(None, root.path(), &config)), ["cpu"]);
    }

    #[test]
    fn order_follows_config_then_alphabetical() {
        let root = tempfile::tempdir().unwrap();
        for name in ["zeta", "cpu", "alpha", "memory"] {
            add_plugin(root.path(), name, "command = [\"run\"]");
        }
        let config = Config {
            plugins: vec!["memory".into(), "missing".into(), "cpu".into()],
            ..Config::default()
        };
        assert_eq!(
            names(&discover(None, root.path(), &config)),
            ["memory", "cpu", "alpha", "zeta"]
        );
    }

    #[test]
    fn interval_precedence_is_config_then_manifest_then_global() {
        let root = tempfile::tempdir().unwrap();
        add_plugin(root.path(), "a", "command = [\"run\"]\ninterval = 7.0");
        add_plugin(root.path(), "b", "command = [\"run\"]\ninterval = 7.0");
        add_plugin(root.path(), "c", "command = [\"run\"]");
        let config = Config {
            interval: 3.0,
            intervals: [("a".to_string(), 11.0)].into(),
            ..Config::default()
        };
        let found = discover(None, root.path(), &config);
        let secs: Vec<u64> = found.plugins.iter().map(|p| p.interval.as_secs()).collect();
        assert_eq!(names(&found), ["a", "b", "c"]);
        assert_eq!(secs, [11, 7, 3]);
    }

    #[test]
    fn invalid_manifest_is_skipped_with_a_warning() {
        let root = tempfile::tempdir().unwrap();
        add_plugin(root.path(), "good", "command = [\"run\"]");
        add_plugin(root.path(), "bad", "label = \"no source\"");
        let found = discover(None, root.path(), &Config::default());
        assert_eq!(names(&found), ["good"]);
        assert_eq!(found.warnings.len(), 1);
        assert!(found.warnings[0].contains("bad"));
    }

    #[test]
    fn duplicate_name_in_one_folder_is_skipped_with_a_warning() {
        let root = tempfile::tempdir().unwrap();
        add_plugin(root.path(), "one", "name = \"same\"\ncommand = [\"a\"]");
        add_plugin(root.path(), "two", "name = \"same\"\ncommand = [\"b\"]");
        let found = discover(None, root.path(), &Config::default());
        assert_eq!(found.plugins.len(), 1);
        assert_eq!(found.plugins[0].dir, root.path().join("one"));
        assert_eq!(found.warnings.len(), 1);
    }

    #[test]
    fn folders_without_a_manifest_are_ignored() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("notes")).unwrap();
        std::fs::write(root.path().join("README.md"), "hello").unwrap();
        let found = discover(None, root.path(), &Config::default());
        assert!(found.plugins.is_empty());
        assert!(found.warnings.is_empty());
    }
}
