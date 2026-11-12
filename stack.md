# Stack

What the project is built with. The reasoning behind each choice is in [decisions.md](decisions.md); decision numbers are given in brackets.

Items marked **confirmed** were decided during scoping. Items marked **proposed** are the expected choice but have not been agreed or tested yet.

## Overview

| Layer | Choice | Status |
|---|---|---|
| Language (backend) | Rust | Confirmed (D3) |
| App framework | Tauri v2 | Confirmed (D3); v2 is assumed because it has built-in window effects |
| Frontend | Plain HTML/CSS/JS, no framework, no bundler | Confirmed (D4) |
| Sparklines | Hand-drawn with inline SVG or `<canvas>`, no charting library | Proposed |
| System metrics | `sysinfo` crate | Proposed |
| Temperature and GPU | Best effort; both readable without sudo on the M2 | Confirmed in phase 1 (D44) |
| Built-in metrics | One Rust helper binary; one run per refresh cycle returns all built-ins that are due | Confirmed (D22, D43) |
| Plugin data format | JSON on standard output, parsed with `serde` and `serde_json` | Confirmed (D15); crates proposed |
| Config | `~/.config/excalibur/config.toml` | Confirmed (D20) |
| Distribution | Homebrew tap, builds on GitHub Releases | Confirmed (D13) |
| Target platform | macOS on Apple Silicon | Confirmed (D2) |

## Backend: Rust

Responsible for:

- Finding plugins in the app bundle and in `~/.config/excalibur/plugins`, with the user folder taking precedence (D21).
- Running each plugin's executable at its refresh rate and parsing the JSON it prints (D15, D18).
- Turning counter values into rates (D16).
- Treating slow, failing or invalid plugins as unavailable (D6).
- Providing the right-click menu (D24).
- Holding recent history in memory for sparklines (D7).
- Sending samples to the frontend through Tauri events or commands.
- Loading themes and applying their window settings (D11).

The project builds two Rust binaries: the Tauri app, and the helper that provides the built-in metrics (D22). All OS-specific metric code lives in the helper, so that adding another OS later means extending the helper and nothing else (D2).

## App framework: Tauri

Window settings the project depends on:

| Setting | Purpose |
|---|---|
| `transparent: true` | Lets the theme decide how see-through the window is. |
| `macOSPrivateApi: true` | Required for transparent windows on macOS. Makes the app ineligible for the Mac App Store, which is not a goal (D13). |
| `windowEffects` | Native macOS blur for the frosted glass theme (D11). |
| `decorations: false` | Borderless window (D23). The page must mark a drag region so the window can be moved. |
| Hidden from the Dock | Set through the macOS activation policy (D23). |

Confirmed in the phase 1 prototype: window effects and width can be changed while the app is running, and a right-click menu works on a borderless, Dock-less window (D24). The prototype is on the local branch `spike/phase1-window`.

## Frontend: plain HTML/CSS/JS

- No framework and no build step; Tauri serves the static files directly.
- The base stylesheet contains layout only. Colours, blur, fonts and spacing come from the active theme, most likely as CSS custom properties (D11).
- The frontend draws each metric according to what its plugin declares (D10) and must handle a metric being unavailable (D6).

## Metric sources

| Metric | Expected source | Status |
|---|---|---|
| CPU usage | macOS host statistics call via the `libc` crate, printed as percent-seconds (D33) | Confirmed in phase 1 |
| Memory usage | `sysinfo` | Proposed |
| Storage | `sysinfo` | Proposed |
| Network throughput | `sysinfo` | Proposed |
| Temperature | `sysinfo` components; average of the die sensors (D44) | Confirmed in phase 1 |
| GPU usage | "Device Utilization %" from the GPU's IORegistry entry, read without sudo | Confirmed in phase 1 with `ioreg`; Rust reader still to write |

Temperature and GPU were confirmed readable without sudo on the MacBook Air M2 in phase 1. On other Mac models they may not be, in which case they show as unavailable (D6).

## Plugins and themes

- A plugin is a manifest plus an executable. The app runs the executable once per refresh and reads one JSON object from its output (D14, D15). Any language works, including shell scripts.
- A plugin reports one number and chooses value only or value with sparkline (D17). Network is two plugins, upload and download.
- Built-in plugins are manifests pointing at the helper binary (D22) and ship inside the app bundle (D21).
- A theme is a folder with a stylesheet and a settings file for window-level effects (D19). The default frosted glass theme ships inside the app bundle.
- User plugins and themes live in `~/.config/excalibur/plugins` and `~/.config/excalibur/themes` (D20).

File formats, all defined in decisions.md:

| File | Purpose | Decision |
|---|---|---|
| `plugin.toml` | Plugin manifest: name, label, unit, `command` or `batch`, display, kind, interval, decimals, min, max | D26, D34, D45, D46 |
| Plugin output | `{"value": 42.1}` or `{"value": null, "reason": "…"}` | D27 |
| `theme.toml` | Window effect, corner radius, default width | D30 |
| `theme.css` | Colours, fonts and spacing | D19 |
| `config.toml` | Theme, global interval, timeout, plugin order, disabled plugins, per-plugin intervals. Optional; the app runs on defaults without it | D35 |

| `state.toml` | Written by the app; holds the theme chosen from the menu. The app never writes `config.toml` | D40 |

The app does no unit scaling; plugins print values in the unit they want shown (D31). Plugins that overrun are skipped for that tick and killed after a timeout, 5 s by default (D28). Sparklines cover a fixed 60 s window (D29).

Measured in phase 1: launching seven processes per second cost about 5.5 % of one core on the M2. Built-ins are therefore collected in one `stats-probe` run per cycle (D43). Each drop-in plugin still costs one process per refresh, so many fast drop-ins will add up.

## Distribution

- Source in a GitHub repository.
- Release builds of the `.app` attached to GitHub Releases.
- A separate tap repository (`homebrew-<name>`) containing a cask that points at the release.
- Builds are unsigned. The cask removes the quarantine flag after install so the app opens without a Gatekeeper warning (D25).

## Development prerequisites

- Rust toolchain (`rustup`).
- Xcode Command Line Tools.
- Tauri CLI.
- No Node.js needed unless a bundler or framework is added later.
