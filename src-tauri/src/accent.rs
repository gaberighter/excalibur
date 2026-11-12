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

/// The accent as the page receives it, for dark and for light surfaces.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Colours {
    pub on_dark: String,
    pub on_light: String,
}

/// A preset's id, a colour written as `#rgb` or `#rrggbb`, or `theme`.
pub fn is_valid(value: &str) -> bool {
    value == THEME_DEFAULT || colours(value).is_some()
}

/// The colours a setting stands for, or `None` for the theme's own.
pub fn colours(value: &str) -> Option<Colours> {
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
/// longer means anything is ignored.
pub fn in_force<'a>(picked: Option<&'a str>, config: Option<&'a str>) -> &'a str {
    picked
        .filter(|value| is_valid(value))
        .or(config)
        .unwrap_or(THEME_DEFAULT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_ids_are_unique_and_do_not_clash_with_theme_default() {
        for (index, preset) in PRESETS.iter().enumerate() {
            assert_ne!(preset.id, THEME_DEFAULT);
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
        assert_eq!(in_force(None, None), THEME_DEFAULT);
    }

    #[test]
    fn a_stale_pick_falls_back_to_config() {
        assert_eq!(in_force(Some("no-longer-a-preset"), Some("red")), "red");
        assert_eq!(in_force(Some("no-longer-a-preset"), None), THEME_DEFAULT);
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
