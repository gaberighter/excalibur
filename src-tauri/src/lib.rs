use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

pub mod engine;
pub mod theme;

use engine::config::{self, Config};
use engine::{Engine, Event, Paths};
use serde::Serialize;
use tauri::menu::{CheckMenuItemBuilder, ContextMenu, MenuBuilder};
use tauri::window::{EffectState, EffectsBuilder};
use tauri::{AppHandle, Emitter, Manager};
use theme::Theme;

const HELPER_NAME: &str = "stats-probe";
const MAIN_WINDOW: &str = "main";
/// Overrides the user's folder, for trying things without touching the real one.
const USER_DIR_ENV: &str = "EXCALIBUR_CONFIG_DIR";

struct AppPaths {
    engine: Paths,
    /// Built-in themes shipped with the app (D21).
    bundled_themes: PathBuf,
}

impl AppPaths {
    fn user_themes(&self) -> PathBuf {
        self.engine.user_dir.join("themes")
    }
}

/// Where things are when running from the source tree. The bundled app's
/// paths are worked out in the packaging phase.
fn dev_paths() -> AppPaths {
    let source_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let helper = std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.join(HELPER_NAME)))
        .filter(|helper| helper.is_file());
    AppPaths {
        engine: Paths {
            bundled_plugins: Some(source_dir.join("plugins")),
            user_dir: user_dir(),
            helper,
        },
        bundled_themes: source_dir.join("themes"),
    }
}

/// `~/.config/excalibur` (D20).
fn user_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os(USER_DIR_ENV) {
        return PathBuf::from(dir);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join(".config").join("excalibur")
}

/// Runs the engine without a window and prints what it produces, for
/// checking plugins from a terminal.
pub fn print_samples(duration: Duration) {
    let paths = dev_paths().engine;
    if paths.helper.is_none() {
        eprintln!("{HELPER_NAME} was not found beside the app; build it with `cargo build`");
    }
    let _engine = Engine::start(paths, |event| match event {
        Event::Loaded { plugins, warnings } => {
            for warning in warnings {
                eprintln!("warning: {warning}");
            }
            for plugin in plugins {
                println!(
                    "loaded {} ({}, every {} s)",
                    plugin.name,
                    if plugin.builtin { "built-in" } else { "user" },
                    plugin.interval_secs
                );
            }
        }
        Event::Sample(sample) => match (sample.value, sample.reason) {
            (Some(value), _) => println!("{:<12} {value:.2}", sample.plugin),
            (None, reason) => println!(
                "{:<12} unavailable: {}",
                sample.plugin,
                reason.unwrap_or_default()
            ),
        },
    });
    std::thread::sleep(duration);
}

/// Sent to the window for every engine event.
const ENGINE_EVENT: &str = "engine";
/// Sent to the window when the theme changes.
const THEME_EVENT: &str = "theme";
/// Menu item ids for themes are this prefix followed by the theme's id.
const THEME_ITEM_PREFIX: &str = "theme:";

/// The plugins and their recent history, for a window that has just loaded.
#[tauri::command]
fn snapshot(engine: tauri::State<Engine>) -> engine::Snapshot {
    engine.snapshot()
}

const MIN_WINDOW_HEIGHT: f64 = 60.0;
const MAX_WINDOW_HEIGHT: f64 = 2000.0;

/// Makes the window tall enough for its content, so every row is visible
/// however many plugins there are. The width is left alone.
///
/// `content` is the height the page needs and `viewport` the height it has.
/// The window grows or shrinks by the difference, because with a title bar
/// the window's own size is not the size of the page.
#[tauri::command]
fn fit_height(window: tauri::Window, content: f64, viewport: f64) -> Result<(), String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let current = window
        .inner_size()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    let height = (current.height + content - viewport).clamp(MIN_WINDOW_HEIGHT, MAX_WINDOW_HEIGHT);
    if (height - current.height).abs() < 1.0 {
        return Ok(());
    }
    window
        .set_size(tauri::LogicalSize::new(current.width, height))
        .map_err(|e| e.to_string())
}

/// The theme in use, and where themes are found.
struct ThemeState {
    bundled: PathBuf,
    user: PathBuf,
    current: Mutex<Active>,
}

struct Active {
    theme: Theme,
    /// Problems found while loading themes, for the window to show.
    warnings: Vec<String>,
}

impl ThemeState {
    fn discover(&self) -> theme::Discovered {
        theme::discover(Some(&self.bundled), &self.user)
    }

    /// Loads the theme called `wanted`, or the default if it is not there.
    fn load(&self, wanted: &str) -> Active {
        let mut discovered = self.discover();
        let (theme, warning) = theme::pick(&discovered.themes, wanted);
        let theme = theme.clone();
        discovered.warnings.extend(warning);
        Active {
            theme,
            warnings: discovered.warnings,
        }
    }
}

/// What the page needs to wear a theme.
#[derive(Clone, Serialize)]
struct ThemePayload {
    id: String,
    name: String,
    css: String,
    radius: f64,
    warnings: Vec<String>,
}

impl From<&Active> for ThemePayload {
    fn from(active: &Active) -> Self {
        ThemePayload {
            id: active.theme.id.clone(),
            name: active.theme.name.clone(),
            css: active.theme.css.clone(),
            radius: active.theme.radius,
            warnings: active.warnings.clone(),
        }
    }
}

/// The theme in use, for a window that has just loaded.
#[tauri::command]
fn theme(state: tauri::State<ThemeState>) -> ThemePayload {
    ThemePayload::from(&*state.current.lock().unwrap())
}

/// Applies the parts of a theme that belong to the window, not the page:
/// the native material behind it, its corner radius and its width (D30).
fn apply_to_window(app: &AppHandle, theme: &Theme) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return Ok(());
    };
    let effects = theme.effect.map(|effect| {
        EffectsBuilder::new()
            .effect(effect)
            // Stay translucent when another app is in front, which is how a
            // monitor window spends most of its time.
            .state(EffectState::Active)
            .radius(theme.radius)
            .build()
    });
    window.set_effects(effects)?;
    let scale = window.scale_factor()?;
    let current = window.inner_size()?.to_logical::<f64>(scale);
    window.set_size(tauri::LogicalSize::new(theme.width, current.height))
}

/// Changes the theme while the app is running.
fn switch_theme(app: &AppHandle, id: &str) {
    let state = app.state::<ThemeState>();
    let active = state.load(id);
    if let Err(e) = apply_to_window(app, &active.theme) {
        log::warn!(
            "could not apply theme '{}' to the window: {e}",
            active.theme.id
        );
    }
    let payload = ThemePayload::from(&active);
    *state.current.lock().unwrap() = active;
    if let Err(e) = app.emit(THEME_EVENT, payload) {
        log::warn!("could not send the theme to the window: {e}");
    }
}

/// Shows the right-click menu (D24). Themes are listed afresh each time, so
/// a theme folder added while the app is running appears without a restart.
#[tauri::command]
fn show_menu(window: tauri::Window, state: tauri::State<ThemeState>) -> Result<(), String> {
    let current = state.current.lock().unwrap().theme.id.clone();
    let mut menu = MenuBuilder::new(&window);
    for theme in state.discover().themes {
        let item =
            CheckMenuItemBuilder::with_id(format!("{THEME_ITEM_PREFIX}{}", theme.id), &theme.name)
                .checked(theme.id == current)
                .build(&window)
                .map_err(|e| e.to_string())?;
        menu = menu.item(&item);
    }
    let menu = menu.build().map_err(|e| e.to_string())?;
    menu.popup(window).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            snapshot, fit_height, theme, show_menu
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let paths = dev_paths();

            let (config, _) = Config::load(&paths.engine.user_dir.join(config::FILE_NAME));
            let themes = ThemeState {
                user: paths.user_themes(),
                bundled: paths.bundled_themes,
                current: Mutex::new(Active {
                    theme: Theme::embedded_default(),
                    warnings: Vec::new(),
                }),
            };
            let active = themes.load(&config.theme);
            apply_to_window(app.handle(), &active.theme)?;
            *themes.current.lock().unwrap() = active;
            app.manage(themes);

            let handle = app.handle().clone();
            let engine = Engine::start(paths.engine, move |event| {
                if let Err(e) = handle.emit(ENGINE_EVENT, event) {
                    log::warn!("could not send engine event to the window: {e}");
                }
            });
            app.manage(engine);
            Ok(())
        })
        .on_menu_event(|app, event| {
            if let Some(id) = event.id().as_ref().strip_prefix(THEME_ITEM_PREFIX) {
                switch_theme(app, id);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
