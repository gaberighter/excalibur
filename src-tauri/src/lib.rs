use std::path::PathBuf;
use std::time::Duration;

pub mod engine;

use engine::{Engine, Event, Paths};
use tauri::{Emitter, Manager};

const HELPER_NAME: &str = "stats-probe";

/// Where things are when running from the source tree. The bundled app's
/// paths are worked out in the packaging phase.
fn dev_paths() -> Paths {
    let helper = std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.join(HELPER_NAME)))
        .filter(|helper| helper.is_file());
    Paths {
        bundled_plugins: Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("plugins")),
        user_dir: user_dir(),
        helper,
    }
}

/// `~/.config/excalibur` (D20).
fn user_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join(".config").join("excalibur")
}

/// Runs the engine without a window and prints what it produces, for
/// checking plugins from a terminal.
pub fn print_samples(duration: Duration) {
    let paths = dev_paths();
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
    log::info!(
        "fitting window to content: page needs {content}, has {viewport}; window {} -> {height}",
        current.height
    );
    window
        .set_size(tauri::LogicalSize::new(current.width, height))
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![snapshot, fit_height])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let handle = app.handle().clone();
            let engine = Engine::start(dev_paths(), move |event| {
                if let Err(e) = handle.emit(ENGINE_EVENT, event) {
                    log::warn!("could not send engine event to the window: {e}");
                }
            });
            app.manage(engine);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
