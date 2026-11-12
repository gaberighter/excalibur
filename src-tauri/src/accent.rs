//! The accent colour: the colour of sparklines and arcs, chosen by the user
//! and applied over whatever theme is in use (D57).

use serde::Serialize;

pub struct Preset {
    /// What config and `state.toml` call it.
    pub id: &'static str,
    /// The name shown in the menu.
    pub name: &'static str,
    /// The colour on a dark surface, and a darker one for a light surface.
    /// Each stands out from the built-in themes' surfaces; a test checks it.
    pub on_dark: &'static str,
    pub on_light: &'static str,
}

const fn preset(
    id: &'static str,
    name: &'static str,
    on_dark: &'static str,
    on_light: &'static str,
) -> Preset {
    Preset {
        id,
        name,
        on_dark,
        on_light,
    }
}

pub const PRESETS: [Preset; 9] = [
    preset("blue", "Blue", "#8ab8ff", "#2563eb"),
    preset("teal", "Teal", "#5eead4", "#0f766e"),
    preset("green", "Green", "#86efac", "#15803d"),
    preset("yellow", "Yellow", "#fde047", "#a16207"),
    preset("orange", "Orange", "#fdba74", "#c2410c"),
    preset("red", "Red", "#fca5a5", "#dc2626"),
    preset("pink", "Pink", "#f9a8d4", "#be185d"),
    preset("purple", "Purple", "#c4b5fd", "#7c3aed"),
    preset("graphite", "Graphite", "#f5f6f8", "#3f4652"),
];

/// Stands for "no accent of my own": each theme uses its own colour.
pub const THEME_DEFAULT: &str = "theme";

/// Stands for the accent colour chosen in the operating system's settings.
pub const SYSTEM: &str = "system";
/// The page's own keyword for the system accent, used where the app cannot
/// read the system's setting itself. Web content is often given a fixed
/// colour for it, so it is a last resort.
const SYSTEM_KEYWORD: &str = "AccentColor";

/// The accent as the page receives it, for dark and for light surfaces.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Colours {
    pub on_dark: String,
    pub on_light: String,
}

/// A preset's id, a colour written as `#rgb` or `#rrggbb`, `system`, or `theme`.
pub fn is_valid(value: &str) -> bool {
    value == THEME_DEFAULT || colours(value).is_some()
}

/// The colours a setting stands for, or `None` for the theme's own.
pub fn colours(value: &str) -> Option<Colours> {
    if value == SYSTEM {
        return Some(system_colours(SYSTEM_KEYWORD));
    }
    if let Some(preset) = PRESETS.iter().find(|preset| preset.id == value) {
        return Some(Colours {
            on_dark: preset.on_dark.to_string(),
            on_light: preset.on_light.to_string(),
        });
    }
    // A colour given by hand is used as it is on every surface.
    is_hex_colour(value).then(|| Colours {
        on_dark: value.to_string(),
        on_light: value.to_string(),
    })
}

/// As `colours`, with the system's accent filled in when it could be read.
pub fn colours_with_system(value: &str, system: Option<&str>) -> Option<Colours> {
    match (value, system) {
        (SYSTEM, Some(colour)) => Some(system_colours(colour)),
        _ => colours(value),
    }
}

/// System accents are mid-tones meant for buttons, too dark for a thin line
/// on a dark surface, so there they are lightened.
fn system_colours(base: &str) -> Colours {
    Colours {
        on_dark: format!("color-mix(in oklab, {base} 55%, white)"),
        on_light: base.to_string(),
    }
}

fn is_hex_colour(value: &str) -> bool {
    match value.strip_prefix('#') {
        Some(digits) => {
            matches!(digits.len(), 3 | 6) && digits.chars().all(|c| c.is_ascii_hexdigit())
        }
        None => false,
    }
}

/// Which setting is in force. The accent picked from the menu wins over the
/// one in config, as it does for themes (D40). A remembered pick that no
/// longer means anything is ignored. With neither, the accent follows the
/// system (D59).
pub fn in_force<'a>(picked: Option<&'a str>, config: Option<&'a str>) -> &'a str {
    picked
        .filter(|value| is_valid(value))
        .or(config)
        .unwrap_or(SYSTEM)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_ids_are_unique_and_do_not_clash_with_theme_default() {
        for (index, preset) in PRESETS.iter().enumerate() {
            assert_ne!(preset.id, THEME_DEFAULT);
            assert_ne!(preset.id, SYSTEM);
            assert!(PRESETS[..index].iter().all(|other| other.id != preset.id));
        }
    }

    #[test]
    fn a_preset_gives_its_two_colours() {
        let colours = colours("orange").unwrap();
        assert_eq!(colours.on_dark, "#fdba74");
        assert_eq!(colours.on_light, "#c2410c");
    }

    #[test]
    fn a_hex_colour_is_used_on_every_surface() {
        let colours = colours("#FF8800").unwrap();
        assert_eq!(colours.on_dark, "#FF8800");
        assert_eq!(colours.on_light, "#FF8800");
        assert!(is_valid("#f80"));
    }

    #[test]
    fn system_follows_the_operating_system_and_is_lighter_on_dark() {
        let colours = colours_with_system(SYSTEM, Some("#f7821b")).unwrap();
        assert_eq!(colours.on_light, "#f7821b");
        assert_eq!(colours.on_dark, "color-mix(in oklab, #f7821b 55%, white)");
        assert!(is_valid(SYSTEM));
    }

    #[test]
    fn system_falls_back_to_the_page_keyword_when_it_cannot_be_read() {
        let colours = colours_with_system(SYSTEM, None).unwrap();
        assert_eq!(colours.on_light, "AccentColor");
    }

    #[test]
    fn the_system_colour_does_not_affect_other_choices() {
        assert_eq!(
            colours_with_system("orange", Some("#007aff")),
            colours("orange")
        );
        assert_eq!(colours_with_system(THEME_DEFAULT, Some("#007aff")), None);
    }

    #[test]
    fn theme_default_and_unknown_values_give_no_colours() {
        assert_eq!(colours(THEME_DEFAULT), None);
        assert_eq!(colours("mauve"), None);
    }

    #[test]
    fn only_presets_hex_colours_and_theme_are_valid() {
        assert!(is_valid("blue") && is_valid("#12ab9F") && is_valid(THEME_DEFAULT));
        for bad in [
            "",
            "mauve",
            "#12",
            "#12345",
            "#gggggg",
            "ff8800",
            "rgb(1,2,3)",
        ] {
            assert!(!is_valid(bad), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn the_menu_pick_wins_over_config() {
        assert_eq!(in_force(Some("green"), Some("red")), "green");
        assert_eq!(in_force(Some(THEME_DEFAULT), Some("red")), THEME_DEFAULT);
        assert_eq!(in_force(None, Some("red")), "red");
        assert_eq!(in_force(None, None), SYSTEM);
        assert_eq!(in_force(Some(THEME_DEFAULT), None), THEME_DEFAULT);
    }

    #[test]
    fn a_stale_pick_falls_back_to_config() {
        assert_eq!(in_force(Some("no-longer-a-preset"), Some("red")), "red");
        assert_eq!(in_force(Some("no-longer-a-preset"), None), SYSTEM);
    }

    fn luminance(hex: &str) -> f64 {
        let channel = |i: usize| {
            let c = u8::from_str_radix(&hex[i..i + 2], 16).unwrap() as f64 / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
    }

    fn contrast(a: &str, b: &str) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// 3:1 is the least a line or arc needs against what it is drawn on.
    #[test]
    fn presets_stand_out_from_the_built_in_surfaces() {
        // Plain in dark mode; the frosted material over a bright desktop,
        // which is as light as it gets; Plain in light mode.
        const PLAIN_DARK: &str = "#1c1f26";
        const FROSTED_LIGHTEST: &str = "#5c5c5e";
        const PLAIN_LIGHT: &str = "#f5f6f8";
        for preset in &PRESETS {
            for surface in [PLAIN_DARK, FROSTED_LIGHTEST] {
                let ratio = contrast(preset.on_dark, surface);
                assert!(ratio >= 3.0, "{} on {surface}: {ratio:.1}", preset.id);
            }
            let ratio = contrast(preset.on_light, PLAIN_LIGHT);
            assert!(ratio >= 3.0, "{} on {PLAIN_LIGHT}: {ratio:.1}", preset.id);
        }
    }
}
