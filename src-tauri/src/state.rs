//! `state.toml`: what the app itself remembers between launches (D40).
//!
//! Kept apart from `config.toml`, which only the user writes.

use std::path::Path;

use serde::{Deserialize, Serialize};

pub const FILE_NAME: &str = "state.toml";

const HEADER: &str = "# Written by Excalibur. Settings you edit yourself belong in config.toml.\n";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// The theme last picked from the menu. Takes precedence over `theme`
    /// in `config.toml`.
    pub theme: Option<String>,
    /// The accent last picked from the menu: a preset's id, or `theme` for
    /// each theme's own colour. Takes precedence over `accent` in
    /// `config.toml`.
    pub accent: Option<String>,
}

impl State {
    /// A missing or unreadable file is the same as having remembered nothing.
    pub fn load(file: &Path) -> State {
        std::fs::read_to_string(file)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Changes what is remembered, keeping everything else in the file.
    pub fn update(file: &Path, change: impl FnOnce(&mut State)) -> Result<(), String> {
        let mut state = State::load(file);
        change(&mut state);
        state.save(file)
    }

    /// Creates the folder if it does not exist yet.
    pub fn save(&self, file: &Path) -> Result<(), String> {
        let body = toml::to_string(self).map_err(|e| e.to_string())?;
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(file, format!("{HEADER}{body}")).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_remembers_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(State::load(&dir.path().join(FILE_NAME)), State::default());
    }

    #[test]
    fn saved_theme_is_loaded_again() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("not-yet-there").join(FILE_NAME);
        let state = State {
            theme: Some("plain".into()),
            accent: None,
        };
        state.save(&file).unwrap();
        assert_eq!(State::load(&file), state);
        assert!(std::fs::read_to_string(&file).unwrap().starts_with('#'));
    }

    #[test]
    fn update_changes_one_thing_and_keeps_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(FILE_NAME);
        State::update(&file, |state| state.theme = Some("plain".into())).unwrap();
        State::update(&file, |state| state.accent = Some("orange".into())).unwrap();
        let state = State::load(&file);
        assert_eq!(state.theme.as_deref(), Some("plain"));
        assert_eq!(state.accent.as_deref(), Some("orange"));

        State::update(&file, |state| state.accent = None).unwrap();
        assert_eq!(State::load(&file).accent, None);
        assert_eq!(State::load(&file).theme.as_deref(), Some("plain"));
    }

    #[test]
    fn unreadable_file_remembers_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(FILE_NAME);
        std::fs::write(&file, "theme = [not toml").unwrap();
        assert_eq!(State::load(&file), State::default());
    }
}
