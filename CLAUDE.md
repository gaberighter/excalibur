# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project state

All phases (0–8) of `implementation_plan.md` are built, plus the compact and round layouts and the accent colour added along the way (results for each are in `decisions.md`). Version 0.1.1 is published and installable with `brew install --cask gaberighter/tap/excalibur`. User documentation is `README.md` and `docs/` (configuration, plugins, themes); keep it in step with the code, and try any example you change. Not done: automatic release builds, and testing on a second Mac.

## Commands

```sh
cargo tauri dev                          # run the app (needs tauri-cli v2)
cargo run -p stats-probe -- cpu memory   # run the helper; prints one JSON object keyed by metric
cargo build --workspace                  # build first: the app looks for stats-probe beside its own binary
cargo run -p excalibur -- --print-samples 10   # run the engine without a window and print samples for 10 s
EXCALIBUR_CONFIG_DIR=/some/dir cargo tauri dev # use another folder in place of ~/.config/excalibur
cargo test --workspace                   # all tests
cargo test -p excalibur engine::sample   # one module's tests
cargo test -p stats-probe <test_name>    # a single test in one crate
node --test ui/sparkline.test.js ui/arcs.test.js   # geometry tests (Node is needed for these only)
cargo clippy --workspace
cargo fmt --all
scripts/build-release.sh                 # build Excalibur.app with everything inside, and zip it
```

Layout: a Cargo workspace with two members, `src-tauri/` (the app; package `excalibur`, library `excalibur_lib`) and `stats-probe/` (the helper). The frontend is static files in `ui/`, served directly by Tauri with no build step.

The engine is `src-tauri/src/engine/` and has no Tauri dependency. `mod.rs` holds the scheduler thread and the `Engine` handle (`start`, `reload`, `snapshot`); it reports `Event::Loaded` and `Event::Sample` through a callback. The other modules are one concern each: `manifest`, `config`, `discovery`, `command` (lookup and `PATH`), `runner` (processes, timeout, output parsing), `sample` (counter rates, clamping) and `history` (60 s window). Built-in plugin manifests are in `src-tauri/plugins/`. Engine tests use shell scripts as fake plugins and real timing, so they need a Unix shell.

App to window: `src-tauri/src/lib.rs` starts the engine and emits each `Event` to the page as an `engine` event; the page calls the `snapshot` command on load and after every `loaded` event, then appends `sample` events. `ui/sparkline.js` is pure geometry (no DOM), `ui/main.js` builds the rows, and `ui/style.css` is layout only, reading every colour, font and spacing from CSS custom properties. The window's height follows its content through the `fit_height` command.

Themes: `src-tauri/src/theme.rs` parses and discovers theme folders; built-in themes are in `src-tauri/themes/`. `lib.rs` applies the window part of a theme (material, radius, width) and sends the stylesheet to the page, which puts it in `<style id="theme">`. There is no stylesheet for colours in `ui/`; without a theme the page has no look. A theme also picks the layout (D50): `rows`, `compact` (one line per metric, D56) or `arcs` (a round window of concentric arcs, D51). In `ui/main.js` each layout is drawn by a view object (`RowsView` for `rows` and `compact`, which differ only in `style.css`; `ArcsView` for `arcs`) with `build`, `draw`, `tick`, `showWarnings` and `height`; `ui/arcs.js` is the arcs geometry, pure like `sparkline.js`. A round window is a square one whose theme sets `radius` to half its `width`. The user's accent colour (D57, `accent.rs`) reaches the page as `--accent` and `--accent-on-light`; themes opt in with `var(--accent, <own colour>)`. Like the theme, the accent picked from the menu is kept in `state.toml` and wins over `config.toml`.

Finding its own files: `app_paths` in `lib.rs`. The packaged app has its plugins, themes and the `stats-probe` helper as resources (`Contents/Resources`), added by `src-tauri/tauri.bundle.conf.json`, which only the release script applies. A debug build with no resources falls back to the source tree; a release build never does.

Window and menu, all in `lib.rs`: the window is created hidden and `fit_height` shows it on its first call, after sizing it and placing it in the corner (`placement.rs` is the geometry). The page must not call `fit_height` before both the theme and the rows are in place, or the window is placed for the wrong height. System tooltips (`title`) do not appear on this window, because it is rarely the active one; explanations go in a `data-tip` attribute, which `main.js` shows in `#tip`, a note drawn inside the window. The page's surface is painted on `#panel`, never on `<body>`, because a body background spreads to the whole transparent window and fills in the rounded corners. `state.rs` is `state.toml` (the theme picked from the menu); `scaffold.rs` creates the user's folder from `config.template.toml` when Open Config Folder is chosen.

Checking the window by eye, without asking the user: launch with `cargo tauri dev`, find the window id of the process named `excalibur` through `CGWindowListCopyWindowInfo` (a few lines of Swift), and capture only that window with `screencapture -l <id>`. Capture the app's window only, never the whole screen. Right-clicks cannot be sent from a terminal, so anything behind the menu needs the user. Layout alone can also be checked by rendering `ui/` in a headless Chromium-based browser with `window.__TAURI__` stubbed.

The project, Excalibur, is a small floating desktop window for macOS (Apple Silicon first) that shows system metrics: CPU, memory, storage, temperature, network and GPU. Its working name was `stats_gui`, which is still the name of the local folder (D42).

## Source of truth

- `project_scope.md`: the problem, the solution and what is out of scope for the first version.
- `decisions.md`: numbered decisions (D1–D59) with what was rejected and why, plus open questions (O-numbers). Read the relevant entries before implementing anything they cover.
- `stack.md`: the stack, as built.
- `implementation_plan.md`: the build order in phases 0–8, each with a "done when" check. Phase 1 (risk prototypes) must finish before the plugin interface is built on.

When a design choice is made or changed, record it in `decisions.md`: append a new D-number, never renumber, and mark a reversed decision as superseded instead of deleting it. Keep `stack.md` and `project_scope.md` consistent with it.

## Planned architecture

Stack: Rust with Tauri v2, and a plain HTML/CSS/JS frontend with no framework and no bundler.

Two Rust binaries:

- **The Tauri app.** Discovers plugins, runs them on a schedule, turns counters into rates, keeps 60 s of history in memory for sparklines, loads themes, and sends samples to the frontend. It contains no metric-collection code of its own.
- **`stats-probe`.** A helper that returns any of the built-in metrics. The app runs it once per refresh cycle for all built-ins that are due, not once per metric (D43, D45). All OS-specific code lives here, which is what keeps a future port to another OS contained.

Plugin model (D14–D18, D26–D28, D31, D33–D34):

- Every metric is a plugin, built-ins included. A plugin is a folder with a `plugin.toml` manifest and an executable. Built-in manifests point at `stats-probe`. Collection differs: each drop-in plugin is its own process, while all built-ins share one `stats-probe` run per cycle (D43).
- The app runs the executable once per refresh and reads one JSON object from stdout: `{"value": 42.1}`, or `{"value": null, "reason": "…"}`. A non-zero exit or invalid output also means unavailable.
- Plugins are stateless and report a single number. For rates, the manifest sets `kind = "counter"`, the plugin prints a running total, and the app computes the change per second. CPU usage uses this path too, by printing cumulative percent-seconds.
- The app does no unit scaling. A plugin prints its value in the unit it wants displayed.
- `min` and `max` clamp the value and are what arcs fill against; sparklines ignore them and always fit their data (D54). `invert = true` marks a value that counts what is left, so arcs fill with the rest (D55).
- A plugin may print `min` and `max` beside `value` when its scale depends on the machine; these win over the manifest's (D52). `stats-probe` does this for memory and storage.
- A plugin still running at its next tick causes that tick to be skipped; it is killed after a timeout (default 5 s).
- The manifest picks the presentation: `display = "value"` or `"sparkline"`.

Themes (D11, D19, D30): a folder with `theme.css` (colours, fonts, spacing) and `theme.toml` (native blur effect, corner radius, default width). The app's base stylesheet holds layout only; no colours or window styling belong in app code.

File locations (D20, D21, D35): built-in plugins and the default theme ship inside the app bundle. User files live in `~/.config/excalibur/` (`config.toml`, `plugins/`, `themes/`). A user plugin or theme with the same name as a built-in replaces it. `config.toml` is optional and the app must run on defaults without it. The app never edits `config.toml`; the theme chosen from the menu goes in `state.toml` in the same folder, which takes precedence (D40).

Command lookup (D38): a bare `command` name in a manifest is resolved in the plugin's own folder, then beside the bundled helper, then on `PATH`.

## Constraints that are easy to violate

- **"Unavailable" is a normal state.** Temperature and GPU are best effort on Apple Silicon. Every layer must handle a metric with no value.
- **Never require sudo** or prompt for a password.
- **No persistence of readings.** History is in memory only and is lost on quit by design.
- **Do not special-case built-in metrics.** If a built-in needs something the plugin interface cannot express, extend the interface.
- **Keep OS-specific code out of the app, frontend, plugin interface and themes.** The one exception is `src-tauri/src/system_accent.rs`, which reads the macOS accent colour, because the page cannot (web content gets a fixed blue for `AccentColor`).
- **First version exclusions:** fill bars, multi-value plugins (network is two plugins, `net-down` and `net-up`), remembered window position, a menu bar icon, code signing.

## Phase 1 findings worth knowing

- Temperature, GPU usage and raw CPU time are all readable without sudo on the M2: `sysinfo` components for temperature, the GPU's IORegistry entry ("Device Utilization %") for GPU, and the host statistics call in `libc` for CPU. The GPU reader has only been tried through the `ioreg` command, not from Rust.
- Launching one process per built-in metric each second cost about 5.5 % of a core, which is why built-ins are batched (D43). Each drop-in plugin still costs a process per refresh.
- The local branch `spike/phase1-window` holds the throwaway prototype of the borderless frosted window. Phases 5 and 6 have replaced it; it can be deleted.

## Distribution

The app repository (`gaberighter/excalibur`) is public. Releases are unsigned builds attached to GitHub Releases, installed through the tap `gaberighter/homebrew-tap` (working copy: `~/dev/homebrew-tap`), whose cask clears the quarantine flag. `macOSPrivateApi` is enabled in Tauri for window transparency, which is acceptable because Mac App Store distribution is not a goal.

Releasing a new version, by hand:

1. Set the new version in `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`, commit and push.
2. Run `scripts/build-release.sh`; it prints the archive's path and checksum.
3. `gh release create v<version> target/release/Excalibur-<version>-arm64.zip --target main`.
4. In the tap, set `version` and `sha256` in `Casks/excalibur.rb`, commit and push.

Publishing a release and pushing to the tap are outward-facing; ask before doing either.
