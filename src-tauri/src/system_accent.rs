//! Reads the accent colour chosen in the operating system's settings (D58).
//!
//! The page cannot be asked for it: the web's `AccentColor` keyword gives
//! web content a fixed blue whatever the setting is.

/// The system's accent colour as `#rrggbb`, or `None` where it cannot be read.
pub fn current() -> Option<&'static str> {
    os::current()
}

/// macOS stores the accent as a number, and stores nothing for the default,
/// "Multicolour", which draws controls in blue.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn macos_colour(setting: Option<i64>) -> Option<&'static str> {
    Some(match setting {
        None | Some(4) => "#007aff",
        Some(-1) => "#989898",
        Some(0) => "#e0383e",
        Some(1) => "#f7821b",
        Some(2) => "#ffc726",
        Some(3) => "#62ba46",
        Some(5) => "#953d96",
        Some(6) => "#f74f9e",
        Some(_) => return None,
    })
}

#[cfg(target_os = "macos")]
mod os {
    use core_foundation::base::{CFType, TCFType};
    use core_foundation::number::CFNumber;
    use core_foundation::string::CFString;
    use core_foundation_sys::preferences::{
        CFPreferencesAppSynchronize, CFPreferencesCopyAppValue, kCFPreferencesAnyApplication,
    };

    const SETTING: &str = "AppleAccentColor";

    pub fn current() -> Option<&'static str> {
        let key = CFString::from_static_string(SETTING);
        // SAFETY: the key and the application id are valid CFStrings. The
        // value comes back owned by us, or null if the setting is absent.
        let value = unsafe {
            // Picks up a change made in System Settings since the last read.
            CFPreferencesAppSynchronize(kCFPreferencesAnyApplication);
            let value =
                CFPreferencesCopyAppValue(key.as_concrete_TypeRef(), kCFPreferencesAnyApplication);
            (!value.is_null()).then(|| CFType::wrap_under_create_rule(value))
        };
        let setting = match value {
            Some(value) => Some(value.downcast::<CFNumber>()?.to_i64()?),
            None => None,
        };
        super::macos_colour(setting)
    }
}

#[cfg(not(target_os = "macos"))]
mod os {
    pub fn current() -> Option<&'static str> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_setting_is_the_multicolour_default_blue() {
        assert_eq!(macos_colour(None), Some("#007aff"));
        assert_eq!(macos_colour(Some(4)), Some("#007aff"));
    }

    #[test]
    fn each_macos_accent_has_a_colour() {
        let colours: Vec<_> = (-1..=6).map(|n| macos_colour(Some(n)).unwrap()).collect();
        assert_eq!(colours[2], "#f7821b", "1 is orange");
        for (index, colour) in colours.iter().enumerate() {
            assert!(colour.starts_with('#') && colour.len() == 7);
            // Blue appears once, as 4; everything else is distinct.
            assert!(!colours[..index].contains(colour));
        }
    }

    #[test]
    fn an_unknown_setting_gives_nothing() {
        assert_eq!(macos_colour(Some(42)), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_setting_can_be_read_on_macos() {
        assert!(current().is_some());
    }
}
