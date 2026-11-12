//! Themes: a folder with `theme.toml` and `theme.css` (D19, D30).

use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::window::Effect;

pub const SETTINGS_FILE: &str = "theme.toml";
pub const STYLESHEET_FILE: &str = "theme.css";
/// The theme used when none is chosen, or the chosen one is missing.
pub const DEFAULT_ID: &str = "frosted";

const DEFAULT_WIDTH: f64 = 220.0;
const WIDTH_RANGE: std::ops::RangeInclusive<f64> = 120.0..=1000.0;
/// Up to half the widest window, so a theme can make the window a circle.
const RADIUS_RANGE: std::ops::RangeInclusive<f64> = 0.0..=500.0;
const NO_EFFECT: &str = "none";

/// How the metrics are arranged in the window (D50).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    /// One row per metric: label and value, with an optional sparkline
    /// underneath.
    #[default]
    Rows,
    /// One line per metric: label, sparkline and value side by side.
    Compact,
    /// One concentric arc per metric, in a square window. With `radius`
    /// set to half the `width`, the window is a circle.
    Arcs,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    /// The folder name; what config and the menu refer to.
    pub id: String,
    /// The name shown in the menu.
    pub name: String,
    /// The native window material behind the page, if any.
    pub effect: Option<Effect>,
    /// Corner radius of the window, in px.
    pub radius: f64,
    /// Width of the window, in px.
    pub width: f64,
    pub layout: Layout,
    pub css: String,
    pub builtin: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    name: Option<String>,
    effect: Option<String>,
    radius: Option<f64>,
    width: Option<f64>,
    layout: Option<Layout>,
}

impl Theme {
    pub fn parse(id: &str, settings: &str, css: &str, builtin: bool) -> Result<Theme, String> {
        let settings: Settings = toml::from_str(settings).map_err(|e| e.message().to_string())?;

        let effect = match settings.effect.as_deref() {
            None | Some(NO_EFFECT) => None,
            Some(name) => Some(
                serde_json::from_value::<Effect>(name.into())
                    .map_err(|_| format!("unknown effect '{name}'"))?,
            ),
        };
        let radius = settings.radius.unwrap_or(0.0);
        if !RADIUS_RANGE.contains(&radius) {
            return Err(format!(
                "'radius' must be between {} and {}",
                RADIUS_RANGE.start(),
                RADIUS_RANGE.end()
            ));
        }
        let width = settings.width.unwrap_or(DEFAULT_WIDTH);
        if !WIDTH_RANGE.contains(&width) {
            return Err(format!(
                "'width' must be between {} and {}",
                WIDTH_RANGE.start(),
                WIDTH_RANGE.end()
            ));
        }

        Ok(Theme {
            id: id.to_string(),
            name: settings.name.unwrap_or_else(|| id.to_string()),
            effect,
            radius,
            width,
            layout: settings.layout.unwrap_or_default(),
            css: css.to_string(),
            builtin,
        })
    }

    /// The default theme, compiled into the app so there is always a look
    /// even if no theme folder can be read.
    pub fn embedded_default() -> Theme {
        Theme::parse(
            DEFAULT_ID,
            include_str!("../themes/frosted/theme.toml"),
            include_str!("../themes/frosted/theme.css"),
            true,
        )
        .expect("the built-in default theme is valid")
    }
}

#[derive(Debug)]
pub struct Discovered {
    /// Sorted by name. Never empty.
    pub themes: Vec<Theme>,
    pub warnings: Vec<String>,
}

/// A user theme replaces a bundled one with the same folder name (D21).
pub fn discover(bundled: Option<&Path>, user: &Path) -> Discovered {
    let mut warnings = Vec::new();
    let mut themes: Vec<Theme> = Vec::new();
    let folders = bundled
        .map(|dir| (dir, true))
        .into_iter()
        .chain([(user, false)]);
    for (dir, builtin) in folders {
        for theme in read_folder(dir, builtin, &mut warnings) {
            themes.retain(|existing| existing.id != theme.id);
            themes.push(theme);
        }
    }
    if themes.is_empty() {
        themes.push(Theme::embedded_default());
    }
    themes.sort_by_key(|theme| theme.name.to_lowercase());
    Discovered { themes, warnings }
}

/// Reads every subfolder that has a settings file. A missing folder is not an error.
fn read_folder(dir: &Path, builtin: bool, warnings: &mut Vec<String>) -> Vec<Theme> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for theme_dir in entries.filter_map(|entry| Some(entry.ok()?.path())) {
        let settings_file = theme_dir.join(SETTINGS_FILE);
        if !settings_file.is_file() {
            continue;
        }
        let id = theme_dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parsed = std::fs::read_to_string(&settings_file)
            .map_err(|e| e.to_string())
            .and_then(|settings| {
                let css = std::fs::read_to_string(theme_dir.join(STYLESHEET_FILE))
                    .map_err(|e| format!("could not read {STYLESHEET_FILE}: {e}"))?;
                Theme::parse(&id, &settings, &css, builtin)
            });
        match parsed {
            Ok(theme) => found.push(theme),
            Err(e) => warnings.push(format!("{}: {e}; skipped", settings_file.display())),
        }
    }
    found
}

/// The wanted theme, or the default with a warning if it is not there.
pub fn pick<'a>(themes: &'a [Theme], wanted: &str) -> (&'a Theme, Option<String>) {
    if let Some(theme) = themes.iter().find(|theme| theme.id == wanted) {
        return (theme, None);
    }
    let fallback = themes
        .iter()
        .find(|theme| theme.id == DEFAULT_ID)
        .unwrap_or(&themes[0]);
    let warning = format!("theme '{wanted}' was not found; using '{}'", fallback.id);
    (fallback, Some(warning))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_theme(root: &Path, id: &str, settings: &str, css: Option<&str>) {
        let dir = root.join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(SETTINGS_FILE), settings).unwrap();
        if let Some(css) = css {
            std::fs::write(dir.join(STYLESHEET_FILE), css).unwrap();
        }
    }

    fn ids(themes: &[Theme]) -> Vec<&str> {
        themes.iter().map(|t| t.id.as_str()).collect()
    }

    #[test]
    fn full_settings_parse() {
        let theme = Theme::parse(
            "frosted",
            "name = \"Frosted\"\neffect = \"hudWindow\"\nradius = 12\nwidth = 220",
            ":root {}",
            true,
        )
        .unwrap();
        assert_eq!(theme.name, "Frosted");
        assert_eq!(theme.effect, Some(Effect::HudWindow));
        assert_eq!((theme.radius, theme.width), (12.0, 220.0));
        assert_eq!(theme.css, ":root {}");
    }

    #[test]
    fn empty_settings_get_defaults() {
        let theme = Theme::parse("mine", "", "", false).unwrap();
        assert_eq!(theme.name, "mine");
        assert_eq!(theme.effect, None);
        assert_eq!((theme.radius, theme.width), (0.0, 220.0));
        assert_eq!(theme.layout, Layout::Rows);
    }

    #[test]
    fn layout_can_be_chosen() {
        let theme = Theme::parse(
            "round",
            "layout = \"arcs\"\nradius = 90\nwidth = 180",
            "",
            false,
        )
        .unwrap();
        assert_eq!(theme.layout, Layout::Arcs);
        assert_eq!(theme.radius, 90.0);
    }

    #[test]
    fn effect_none_means_no_effect() {
        let theme = Theme::parse("plain", "effect = \"none\"", "", false).unwrap();
        assert_eq!(theme.effect, None);
    }

    #[test]
    fn invalid_settings_are_rejected() {
        for settings in [
            "effect = \"glitter\"",
            "radius = -1",
            "radius = 501",
            "layout = \"spiral\"",
            "width = 10",
            "colour = \"red\"",
            "width = \"wide\"",
        ] {
            assert!(
                Theme::parse("x", settings, "", false).is_err(),
                "{settings}"
            );
        }
    }

    #[test]
    fn embedded_default_is_the_frosted_theme() {
        let theme = Theme::embedded_default();
        assert_eq!(theme.id, DEFAULT_ID);
        assert_eq!(theme.effect, Some(Effect::HudWindow));
        assert!(theme.css.contains("--surface"));
    }

    #[test]
    fn bundled_themes_are_valid() {
        let bundled = Path::new(env!("CARGO_MANIFEST_DIR")).join("themes");
        let found = discover(Some(&bundled), Path::new("/nonexistent"));
        assert!(found.warnings.is_empty(), "{:?}", found.warnings);
        assert_eq!(
            ids(&found.themes),
            ["frosted", "frosted-compact", "frosted-round", "plain"]
        );
    }

    #[test]
    fn user_theme_replaces_builtin_with_the_same_folder_name() {
        let root = tempfile::tempdir().unwrap();
        let (bundled, user) = (root.path().join("bundled"), root.path().join("user"));
        add_theme(&bundled, "frosted", "name = \"Frosted\"", Some("a {}"));
        add_theme(&user, "frosted", "name = \"My Frosted\"", Some("b {}"));
        add_theme(&user, "neon", "name = \"Neon\"", Some("c {}"));

        let found = discover(Some(&bundled), &user);
        assert_eq!(ids(&found.themes), ["frosted", "neon"]);
        assert_eq!(found.themes[0].name, "My Frosted");
        assert!(!found.themes[0].builtin);
    }

    #[test]
    fn invalid_theme_is_skipped_with_a_warning() {
        let root = tempfile::tempdir().unwrap();
        add_theme(root.path(), "good", "", Some(""));
        add_theme(root.path(), "bad-settings", "width = 1", Some(""));
        add_theme(root.path(), "no-stylesheet", "", None);

        let found = discover(None, root.path());
        assert_eq!(ids(&found.themes), ["good"]);
        assert_eq!(found.warnings.len(), 2);
    }

    #[test]
    fn no_themes_on_disk_still_gives_the_default() {
        let root = tempfile::tempdir().unwrap();
        let found = discover(None, root.path());
        assert_eq!(ids(&found.themes), [DEFAULT_ID]);
    }

    #[test]
    fn pick_returns_the_wanted_theme() {
        let themes = [
            Theme::parse("frosted", "", "", true).unwrap(),
            Theme::parse("plain", "", "", true).unwrap(),
        ];
        let (theme, warning) = pick(&themes, "plain");
        assert_eq!(theme.id, "plain");
        assert_eq!(warning, None);
    }

    #[test]
    fn pick_falls_back_to_the_default_with_a_warning() {
        let themes = [
            Theme::parse("aurora", "", "", false).unwrap(),
            Theme::parse("frosted", "", "", true).unwrap(),
        ];
        let (theme, warning) = pick(&themes, "missing");
        assert_eq!(theme.id, "frosted");
        assert!(warning.unwrap().contains("missing"));
    }

    #[test]
    fn pick_falls_back_to_any_theme_when_the_default_is_gone() {
        let themes = [Theme::parse("aurora", "", "", false).unwrap()];
        assert_eq!(pick(&themes, "missing").0.id, "aurora");
    }
}
