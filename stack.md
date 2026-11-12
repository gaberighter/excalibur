# Stack

What the project is built with, as built. The reasoning behind each choice is in [decisions.md](decisions.md); decision numbers are given in brackets.

## Overview

| Layer | Choice | Decision |
|---|---|---|
| Language | Rust | D3 |
| App framework | Tauri v2 | D3 |
| Page | Plain HTML, CSS and JavaScript; no framework, no bundler, no build step | D4 |
| Sparklines and arcs | Hand-drawn inline SVG; no charting library | D7, D51 |
| Built-in metrics | One helper binary, `stats-probe`; one run per refresh cycle returns every built-in that is due | D22, D43, D45 |
| Plugin data format | JSON on standard output | D15, D27, D52 |
| Settings and manifests | TOML | D20, D26, D30 |
| Distribution | A zip of the app on GitHub Releases, installed by a Homebrew cask from a personal tap | D13, D25 |
| Target platform | macOS 13 or later on Apple Silicon | D2 |

## Rust crates

| Crate | Used for |
|---|---|
| `tauri` (with `macos-private-api`) | The window, the right-click menu, window effects, and events to the page. |
| `serde`, `serde_json`, `toml` | Reading plugin output, manifests, themes, config and state. |
| `sysinfo` (in `stats-probe`) | Temperature sensors, GPU utilisation and network totals. |
| `libc` | In `stats-probe`: CPU time, memory and disk statistics. In the app: stopping a timed-out plugin and everything it started. |
| `core-foundation` | Reading the macOS accent colour setting. |
| `tauri-plugin-log`, `log` | Log output in debug builds. |
| `tempfile` (tests only) | Scratch folders for engine tests. |

## The two binaries

- **The app** (`src-tauri/`). Finds plugins and themes, runs plugins on a schedule, turns counters into rates, keeps 60 seconds of history in memory, and sends samples to the page. It collects no metrics itself.
- **`stats-probe`** (`stats-probe/`). Reports any of the seven built-in metrics as one JSON object keyed by name. All metric code that is specific to an operating system is here, so supporting another system means extending this binary (D2).

The app has one piece of system-specific code of its own: reading the macOS accent colour (D58).

## Tauri settings the app depends on

| Setting | Purpose |
|---|---|
| `transparent: true` | Lets the theme decide what shows behind the page, and lets the window be round. |
| `macOSPrivateApi: true` | Required for transparent windows on macOS. Rules out the Mac App Store, which is not a goal (D13). |
| `decorations: false` | No title bar (D23). The page starts a window drag on mouse-down. |
| `alwaysOnTop: true` | The window floats above others (D48). |
| `visible: false` | The window is shown once its size and corner are known (D49). |
| Window effects, set at run time | The native material behind a theme (D11, D30). |
| Accessory activation policy, and `LSUIElement` in `Info.plist` | No Dock icon (D23). |
| `withGlobalTauri: true` | Lets the page call the app without a bundler. |

## Metric sources

All read without administrator rights (D6).

| Metric | Source |
|---|---|
| CPU | The macOS host statistics call, reported as cumulative percent-seconds (D33). |
| Memory | The macOS host VM statistics: app memory, wired and compressed; reported with the memory installed as its maximum (D37, D52). |
| Storage | `statvfs` on the startup volume: free space, reported with the disk's size as its maximum (D37, D52). |
| Temperature | `sysinfo` components; the average of the die sensors (D44). |
| GPU | `sysinfo`, which reads "Device Utilization %" from the GPU's IORegistry entry. |
| Network | `sysinfo` network totals, summed over Wi-Fi and Ethernet interfaces. |

Temperature and GPU were confirmed on a MacBook Air M2 only. On another Mac they may not be readable, in which case they show as unavailable.

## Files

| File | Purpose | Decision |
|---|---|---|
| `plugin.toml` | A plugin's manifest | D26, D34, D45, D46, D53, D55 |
| Plugin output | `{"value": 42.1}`, optionally with `min` and `max`; or `{"value": null, "reason": "…"}` | D27, D52 |
| `theme.toml` | A theme's window settings: effect, radius, width, layout | D30, D50 |
| `theme.css` | A theme's colours, fonts and spacing, as CSS custom properties | D19 |
| `config.toml` | The user's settings; optional, and never written by the app | D35, D47, D57 |
| `state.toml` | Written by the app: the theme and accent picked from the menu | D40, D57 |

User files are in `~/.config/excalibur/` (D20). Built-in plugins and themes are inside the app, in `Contents/Resources` (D21). These formats are described for users in [docs/](docs/).

## Costs measured on the MacBook Air M2

- Launching any process costs about 3 ms of CPU time, and `stats-probe` about 8 ms, rising to about 20 ms when it reads all seven metrics; temperature alone is about 9 ms.
- Launching one process per built-in metric each second cost about 5.5 % of one core, which is why the built-ins share one run (D43).
- The engine and its helper runs together, at the built-in intervals, use about 1.7 % of one core.
- Every drop-in plugin costs one process per refresh.

## Distribution

- Source and releases: <https://github.com/gaberighter/excalibur>, public.
- `scripts/build-release.sh` builds `Excalibur.app` with the helper, plugins and themes inside, signs it ad hoc and zips it.
- The tap, <https://github.com/gaberighter/homebrew-tap>, holds a cask that downloads the zip from the release, installs the app and clears the quarantine flag (D25).
- Builds are not signed with a Developer ID and not notarised (D25).
- Releases are made by hand; the steps are in `CLAUDE.md`.

## Development prerequisites

- Rust toolchain (`rustup`).
- Xcode Command Line Tools.
- Tauri CLI (`cargo install tauri-cli --version "^2" --locked`).
- Node.js, only to run the page's geometry tests.
