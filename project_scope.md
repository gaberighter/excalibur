# Project Scope

Related documents: [decisions.md](decisions.md) (what was decided and why), [stack.md](stack.md) (what it is built with).

## Problem

I own a MacBook Air M2 and I cannot see all of my device status metrics in one place. By device status metrics I mean things like temperature, storage, memory usage, CPU usage, GPU usage and network activity.

Today these are scattered:

- Storage is in System Settings.
- CPU, memory, GPU and network are spread across separate tabs and windows of Activity Monitor.
- Temperature is not shown anywhere by macOS; it needs a third-party tool or a terminal command with administrator rights.

Checking the state of the machine therefore means opening several apps and seeking each number out, and none of them give a compact view that can stay on screen while I work.

## Solution

A small, clean, floating window that shows all of these metrics together, built so that new metrics and new looks can be added without changing the app itself.

### What it shows

First version metrics:

| Metric | Notes |
|---|---|
| CPU usage | |
| Memory usage | |
| Storage | |
| Network | Upload and download throughput |
| GPU usage | Best effort on Apple Silicon |
| Temperature | Best effort on Apple Silicon |

Each metric shows its current value and, where it makes sense, a sparkline of recent history. History is held in memory only; nothing is written to disk.

Temperature and GPU have no official public API on Apple Silicon. The app tries to read them without administrator rights and shows the metric as unavailable if it cannot. It never asks for a password.

### How it looks

- A small floating window with no title bar and no Dock icon.
- Frosted glass by default.
- The look is defined by a theme, not hard-coded. Themes can be added later and switched quickly.
- Right-clicking the window gives a menu to switch theme, reload plugins, open the config folder and quit.

### Extensibility

- **Metric plugins.** Every metric, including the built-in ones, is a plugin: a small executable that prints one reading each time it is run, plus a short description of the metric. New metrics are added by dropping a plugin into a folder; the app does not need to be rebuilt.
- **Plugin-controlled display.** A plugin chooses whether it is shown as a value only or as a value with a sparkline.
- **Themes.** Themes are drop-in in the same way: a folder with a stylesheet and a settings file.
- **Refresh rate.** A global default of 1–2 seconds, which individual plugins can override.
- **One place for my own files.** Config, plugins and themes live in `~/.config/excalibur`. Built-in plugins and the default theme ship inside the app and can be overridden or disabled from there.

### Maintainability and installation

- Installable on any Mac I use through my own Homebrew tap, with updates through `brew upgrade`.
- Kept deliberately small: Rust backend, plain HTML/CSS/JS frontend, no frontend framework.

### Portability

Portability to other operating systems is a bonus, not a first-version requirement. The stack is cross-platform, but only the macOS metric sources are built now. Other operating systems get their own metric sources when I actually need them, without changes to the window, plugin interface or themes.

## Out of scope for the first version

- Battery metrics and a top-processes list.
- Fill bars, and plugins that report more than one value.
- Always-on-top, and remembering window position and size.
- A menu bar icon.
- Code signing and notarisation.
- History persisted to disk, or looking back over hours or days.
- Linux and Windows metric sources.
- Anything requiring administrator rights.
- Mac App Store distribution.
