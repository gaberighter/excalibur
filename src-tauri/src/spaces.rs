//! Lets the window sit beside an app in full screen (D64).
//!
//! Joining every Space is set in `tauri.conf.json` (`visibleOnAllWorkspaces`).
//! macOS also keeps windows off a full-screen app's Space unless they are
//! marked as auxiliary to it, which Tauri has no setting for.

use tauri::WebviewWindow;

/// Must be called on the main thread, as `setup` is.
pub fn allow_beside_full_screen(window: &WebviewWindow) -> tauri::Result<()> {
    os::allow_beside_full_screen(window)
}

#[cfg(target_os = "macos")]
mod os {
    use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};

    pub fn allow_beside_full_screen(window: &tauri::WebviewWindow) -> tauri::Result<()> {
        let pointer = window.ns_window()?;
        // SAFETY: Tauri hands out the window's own NSWindow, which lives as
        // long as the window, and the caller is on the main thread.
        let ns_window = unsafe { &*pointer.cast::<NSWindow>() };
        let behavior =
            ns_window.collectionBehavior() | NSWindowCollectionBehavior::FullScreenAuxiliary;
        ns_window.setCollectionBehavior(behavior);
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
mod os {
    pub fn allow_beside_full_screen(_window: &tauri::WebviewWindow) -> tauri::Result<()> {
        Ok(())
    }
}
