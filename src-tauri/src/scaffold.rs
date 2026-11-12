//! Sets up the user's folder the first time it is opened from the menu (D41).

use std::path::Path;

use crate::engine::config;

/// Every setting, commented out and showing its default.
const CONFIG_TEMPLATE: &str = include_str!("../config.template.toml");

/// Creates the folder, its `plugins` and `themes` subfolders and a commented
/// `config.toml`. Anything that already exists is left untouched.
pub fn ensure_user_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir.join("plugins"))?;
    std::fs::create_dir_all(dir.join("themes"))?;
    let config_file = dir.join(config::FILE_NAME);
    if !config_file.exists() {
        std::fs::write(config_file, CONFIG_TEMPLATE)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::engine::config::Config;

    #[test]
    fn creates_folders_and_a_config_that_changes_nothing() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("excalibur");
        ensure_user_dir(&dir).unwrap();

        assert!(dir.join("plugins").is_dir());
        assert!(dir.join("themes").is_dir());
        let (config, warning) = Config::load(&dir.join(config::FILE_NAME));
        assert_eq!(config, Config::default());
        assert_eq!(warning, None);
    }

    #[test]
    fn existing_config_is_not_overwritten() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join(config::FILE_NAME);
        std::fs::write(&file, "interval = 9.0\n").unwrap();
        ensure_user_dir(root.path()).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "interval = 9.0\n");
    }

    /// Guards against the template drifting from the real settings: with its
    /// setting lines uncommented it must parse, and give the defaults.
    #[test]
    fn template_shows_the_real_defaults() {
        let uncommented: String = CONFIG_TEMPLATE
            .lines()
            .map(|line| match line.strip_prefix("# ") {
                Some(rest) if rest.contains(" = ") || rest.starts_with('[') => rest,
                _ => line,
            })
            .collect::<Vec<_>>()
            .join("\n");
        let config = Config::parse(&uncommented).unwrap();
        // The [intervals] lines are an example, not a default.
        assert_eq!(config.intervals.len(), 2);
        let without_example = Config {
            intervals: BTreeMap::new(),
            ..config
        };
        assert_eq!(without_example, Config::default());
    }
}
