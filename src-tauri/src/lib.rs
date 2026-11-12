use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub mod accent;
pub mod engine;
pub mod placement;
pub mod scaffold;
pub mod state;
pub mod system_accent;
pub mod theme;

use engine::config::{self, Config};
use engine::{Engine, Event, Paths};
use placement::Area;
use serde::Serialize;
use state::State;
use tauri::menu::{CheckMenuItemBuilder, ContextMenu, MenuBuilder, SubmenuBuilder};
use tauri::window::{EffectState, EffectsBuilder};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
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

/// Where the app's own files are. In the packaged app the plugins, themes
/// and helper are resources inside the bundle (D21); `resources` is that
/// folder. Run from the source tree, they are where they are in the repo.
fn app_paths(resources: Option<PathBuf>) -> AppPaths {
    let Some(resources) = resources.filter(|dir| dir.join("plugins").is_dir()) else {
        return source_tree_paths();
    };
    let helper = resources.join(HELPER_NAME);
    AppPaths {
        engine: Paths {
            bundled_plugins: Some(resources.join("plugins")),
            user_dir: user_dir(),
            helper: helper.is_file().then_some(helper),
        },
        bundled_themes: resources.join("themes"),
    }
}

/// The helper is built beside the app, and the rest is in the source folder.
#[cfg(debug_assertions)]
fn source_tree_paths() -> AppPaths {
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

/// A release build never looks in the source tree. Without its resources it
/// has no built-in plugins, and only the theme compiled into it.
#[cfg(not(debug_assertions))]
fn source_tree_paths() -> AppPaths {
    AppPaths {
        engine: Paths {
            bundled_plugins: None,
            user_dir: user_dir(),
            helper: None,
        },
        bundled_themes: PathBuf::new(),
    }
}

/// The resources folder of a macOS app bundle, worked out from where the
/// app's binary is: `Excalibur.app/Contents/MacOS/excalibur`.
fn resources_beside_binary() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.parent()?.join("Resources"))
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
    let paths = app_paths(resources_beside_binary()).engine;
    if paths.helper.is_none() {
        eprintln!("{HELPER_NAME} was not found; from the source tree, build it with `cargo build`");
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
/// Menu item ids for accents are this prefix followed by the preset's id.
const ACCENT_ITEM_PREFIX: &str = "accent:";
/// The accent item that goes back to the colour set in config.
const ACCENT_FROM_CONFIG: &str = "config";
const RELOAD_ITEM: &str = "reload";
const OPEN_CONFIG_ITEM: &str = "open-config";
const QUIT_ITEM: &str = "quit";

/// The plugins and their recent history, for a window that has just loaded.
#[tauri::command]
fn snapshot(engine: tauri::State<Engine>) -> engine::Snapshot {
    engine.snapshot()
}

const MIN_WINDOW_HEIGHT: f64 = 60.0;
const MAX_WINDOW_HEIGHT: f64 = 2000.0;

/// How often the system's accent colour is read again.
const SYSTEM_ACCENT_CHECK: Duration = Duration::from_secs(2);

/// Gap between the window and the screen's edges where it first appears.
const CORNER_MARGIN: f64 = 16.0;
/// The window is shown this long after launch even if the page has not
/// reported its size, so a page that fails to load cannot leave an app with
/// no Dock icon and no window.
const SHOW_ANYWAY_AFTER: Duration = Duration::from_secs(3);

/// Whether the window has been put in its starting corner and shown.
#[derive(Default)]
struct Placed(AtomicBool);

enum Position {
    /// The bottom right corner of the screen (D49).
    Corner,
    /// Where it is, moved only if part of it would be off the screen.
    Current,
}

/// Resizes the window and makes sure all of it is still on the screen.
fn resize(
    window: &WebviewWindow,
    width: f64,
    height: f64,
    position: Position,
) -> tauri::Result<()> {
    window.set_size(tauri::LogicalSize::new(width, height))?;
    let monitor = match window.current_monitor()? {
        Some(monitor) => monitor,
        None => match window.primary_monitor()? {
            Some(monitor) => monitor,
            None => return Ok(()),
        },
    };
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    let area = Area {
        x: work.position.x as f64,
        y: work.position.y as f64,
        width: work.size.width as f64,
        height: work.size.height as f64,
    };
    // The size just asked for; the window may not report it yet.
    let size = (width * scale, height * scale);
    let (x, y) = match position {
        Position::Corner => placement::bottom_right(area, size, CORNER_MARGIN * scale),
        Position::Current => {
            let current = window.outer_position()?;
            placement::keep_inside(area, (current.x as f64, current.y as f64), size)
        }
    };
    window.set_position(tauri::PhysicalPosition::new(
        x.round() as i32,
        y.round() as i32,
    ))
}

/// Makes the window tall enough for its content, so every row is visible
/// however many plugins there are. The width is left alone.
///
/// `content` is the height the page needs and `viewport` the height it has;
/// the window grows or shrinks by the difference. The first call also puts
/// the window in its starting corner and shows it, now that its size is known.
#[tauri::command]
fn fit_height(
    window: WebviewWindow,
    placed: tauri::State<Placed>,
    content: f64,
    viewport: f64,
) -> Result<(), String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let current = window
        .inner_size()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(scale);
    let height = (current.height + content - viewport).clamp(MIN_WINDOW_HEIGHT, MAX_WINDOW_HEIGHT);
    let first = !placed.0.swap(true, Ordering::SeqCst);
    if first {
        resize(&window, current.width, height, Position::Corner).map_err(|e| e.to_string())?;
        return window.show().map_err(|e| e.to_string());
    }
    if (height - current.height).abs() < 1.0 {
        return Ok(());
    }
    resize(&window, current.width, height, Position::Current).map_err(|e| e.to_string())
}

/// The theme in use, and where themes are found.
struct ThemeState {
    bundled: PathBuf,
    user: PathBuf,
    /// Where the theme picked from the menu is remembered (D40).
    state_file: PathBuf,
    current: Mutex<Active>,
    accent: Mutex<AccentChoice>,
}

/// The two places an accent can come from (D57).
#[derive(Default)]
struct AccentChoice {
    /// Picked from the menu and remembered in `state.toml`.
    picked: Option<String>,
    /// Set in `config.toml`.
    config: Option<String>,
}

impl AccentChoice {
    fn in_force(&self) -> &str {
        accent::in_force(self.picked.as_deref(), self.config.as_deref())
    }
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

    /// What the page needs to wear the theme and accent in use.
    fn payload(&self) -> ThemePayload {
        let active = self.current.lock().unwrap();
        ThemePayload {
            id: active.theme.id.clone(),
            name: active.theme.name.clone(),
            css: active.theme.css.clone(),
            radius: active.theme.radius,
            width: active.theme.width,
            layout: active.theme.layout,
            accent: accent::colours_with_system(
                self.accent.lock().unwrap().in_force(),
                system_accent::current(),
            ),
            warnings: active.warnings.clone(),
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
    width: f64,
    layout: theme::Layout,
    /// The user's accent colour, if they have one; otherwise the theme's own.
    accent: Option<accent::Colours>,
    warnings: Vec<String>,
}

/// The theme in use, for a window that has just loaded.
#[tauri::command]
fn theme(state: tauri::State<ThemeState>) -> ThemePayload {
    state.payload()
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
    resize(&window, theme.width, current.height, Position::Current)
}

/// Changes the theme while the app is running, and remembers the choice.
fn switch_theme(app: &AppHandle, id: &str) {
    let state = app.state::<ThemeState>();
    let active = state.load(id);
    let id = active.theme.id.clone();
    if let Err(e) = State::update(&state.state_file, |remembered| remembered.theme = Some(id)) {
        log::warn!("could not remember the chosen theme: {e}");
    }
    if let Err(e) = apply_to_window(app, &active.theme) {
        log::warn!(
            "could not apply theme '{}' to the window: {e}",
            active.theme.id
        );
    }
    *state.current.lock().unwrap() = active;
    send_theme(app);
}

fn send_theme(app: &AppHandle) {
    if let Err(e) = app.emit(THEME_EVENT, app.state::<ThemeState>().payload()) {
        log::warn!("could not send the theme to the window: {e}");
    }
}

/// Changes the accent colour while the app is running, and remembers the
/// choice. `picked` is a preset's id, `theme` for each theme's own colour,
/// or `None` to go back to whatever config says.
fn pick_accent(app: &AppHandle, picked: Option<String>) {
    let state = app.state::<ThemeState>();
    let remembered = picked.clone();
    if let Err(e) = State::update(&state.state_file, |state| state.accent = remembered) {
        log::warn!("could not remember the chosen accent: {e}");
    }
    state.accent.lock().unwrap().picked = picked;
    send_theme(app);
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
    let accents = accent_menu(&window, &state).map_err(|e| e.to_string())?;
    let menu = menu
        .separator()
        .item(&accents)
        .separator()
        .text(RELOAD_ITEM, "Reload Plugins")
        .text(OPEN_CONFIG_ITEM, "Open Config Folder")
        .separator()
        .text(QUIT_ITEM, "Quit Excalibur")
        .build()
        .map_err(|e| e.to_string())?;
    menu.popup(window).map_err(|e| e.to_string())
}

/// The accent colours, with the one in force ticked.
fn accent_menu(
    window: &tauri::Window,
    state: &ThemeState,
) -> tauri::Result<tauri::menu::Submenu<tauri::Wry>> {
    let choice = state.accent.lock().unwrap();
    let in_force = choice.in_force();
    // Ticked when the colour comes from config and not from this menu.
    let from_config = choice.picked.is_none() && choice.config.is_some();
    let item = |id: &str, name: &str, checked: bool| {
        CheckMenuItemBuilder::with_id(format!("{ACCENT_ITEM_PREFIX}{id}"), name)
            .checked(checked)
            .build(window)
    };

    let mut menu = SubmenuBuilder::new(window, "Accent Color").item(&item(
        accent::THEME_DEFAULT,
        "Theme Default",
        !from_config && in_force == accent::THEME_DEFAULT,
    )?);
    menu = menu.item(&item(
        accent::SYSTEM,
        "System Accent",
        !from_config && in_force == accent::SYSTEM,
    )?);
    if choice.config.is_some() {
        menu = menu.item(&item(ACCENT_FROM_CONFIG, "From config.toml", from_config)?);
    }
    menu = menu.separator();
    for preset in &accent::PRESETS {
        menu = menu.item(&item(
            preset.id,
            preset.name,
            !from_config && in_force == preset.id,
        )?);
    }
    menu.build()
}

/// Opens the user's folder in the file manager, creating it first if it is
/// not there (D41).
fn open_config_folder(dir: &Path) {
    if let Err(e) = scaffold::ensure_user_dir(dir) {
        log::warn!("could not set up {}: {e}", dir.display());
        return;
    }
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(opener).arg(dir).spawn() {
        log::warn!("could not open {}: {e}", dir.display());
    }
}

fn on_menu_item(app: &AppHandle, id: &str) {
    match id {
        RELOAD_ITEM => app.state::<Engine>().reload(),
        OPEN_CONFIG_ITEM => open_config_folder(&app.state::<UserDir>().0),
        QUIT_ITEM => app.exit(0),
        _ => {
            if let Some(theme) = id.strip_prefix(THEME_ITEM_PREFIX) {
                switch_theme(app, theme);
            } else if let Some(accent) = id.strip_prefix(ACCENT_ITEM_PREFIX) {
                // "From config.toml" forgets the pick, so config applies again.
                let picked = (accent != ACCENT_FROM_CONFIG).then(|| accent.to_string());
                pick_accent(app, picked);
            }
        }
    }
}

/// `~/.config/excalibur`, or its override.
struct UserDir(PathBuf);

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
            // No Dock icon and no entry in the app switcher (D23).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let paths = app_paths(app.path().resource_dir().ok());
            let user_dir = paths.engine.user_dir.clone();
            app.manage(UserDir(user_dir.clone()));
            app.manage(Placed::default());

            // The theme picked from the menu wins over the one in config (D40).
            let (config, _) = Config::load(&user_dir.join(config::FILE_NAME));
            let state_file = user_dir.join(state::FILE_NAME);
            let remembered = State::load(&state_file);
            let wanted = remembered.theme.unwrap_or(config.theme);
            let themes = ThemeState {
                user: paths.user_themes(),
                bundled: paths.bundled_themes,
                state_file,
                accent: Mutex::new(AccentChoice {
                    picked: remembered.accent,
                    config: config.accent,
                }),
                current: Mutex::new(Active {
                    theme: Theme::embedded_default(),
                    warnings: Vec::new(),
                }),
            };
            let active = themes.load(&wanted);
            apply_to_window(app.handle(), &active.theme)?;
            *themes.current.lock().unwrap() = active;
            app.manage(themes);

            // The system's accent can change while the app runs. Nothing
            // announces it in a portable way, so it is read again now and then.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut last = system_accent::current();
                loop {
                    std::thread::sleep(SYSTEM_ACCENT_CHECK);
                    let themes = handle.state::<ThemeState>();
                    if themes.accent.lock().unwrap().in_force() != accent::SYSTEM {
                        continue;
                    }
                    let now = system_accent::current();
                    if now != last {
                        last = now;
                        send_theme(&handle);
                    }
                }
            });

            // The window starts hidden and is shown by the page's first
            // `fit_height`, once its size and corner are known.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(SHOW_ANYWAY_AFTER);
                if !handle.state::<Placed>().0.swap(true, Ordering::SeqCst)
                    && let Some(window) = handle.get_webview_window(MAIN_WINDOW)
                {
                    let _ = window.show();
                }
            });

            let handle = app.handle().clone();
            let engine = Engine::start(paths.engine, move |event| {
                if let Err(e) = handle.emit(ENGINE_EVENT, event) {
                    log::warn!("could not send engine event to the window: {e}");
                }
            });
            app.manage(engine);
            Ok(())
        })
        .on_menu_event(|app, event| on_menu_item(app, event.id().as_ref()))
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
