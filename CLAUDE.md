# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project state

Phase 0 of `implementation_plan.md` is complete: the repository builds an empty Tauri window and an empty `stats-probe` helper. No metrics, plugin engine or themes exist yet.

## Commands

```sh
cargo tauri dev                          # run the app (needs tauri-cli v2)
cargo run -p stats-probe -- <metric>     # run the helper
cargo build --workspace
cargo test --workspace                   # all tests
cargo test -p stats-probe <test_name>    # a single test in one crate
cargo clippy --workspace
cargo fmt --all
```

Layout: a Cargo workspace with two members, `src-tauri/` (the app; package `excalibur`, library `excalibur_lib`) and `stats-probe/` (the helper). The frontend is static files in `ui/`, served directly by Tauri with no build step.

The project, Excalibur, is a small floating desktop window for macOS (Apple Silicon first) that shows system metrics: CPU, memory, storage, temperature, network and GPU. Its working name was `stats_gui`, which is still the name of the local folder (D42).

## Source of truth

- `project_scope.md`: the problem, the solution and what is out of scope for the first version.
- `decisions.md`: numbered decisions (D1–D42) with what was rejected and why, plus open questions (O-numbers). Read the relevant entries before implementing anything they cover.
- `stack.md`: the stack, with each item marked confirmed, proposed or to verify.
- `implementation_plan.md`: the build order in phases 0–8, each with a "done when" check. Phase 1 (risk prototypes) must finish before the plugin interface is built on.

When a design choice is made or changed, record it in `decisions.md`: append a new D-number, never renumber, and mark a reversed decision as superseded instead of deleting it. Keep `stack.md` and `project_scope.md` consistent with it. Items marked "proposed" or "to verify" in `stack.md` have not been agreed or tested; do not treat them as settled.

## Planned architecture

Stack: Rust with Tauri v2, and a plain HTML/CSS/JS frontend with no framework and no bundler.

Two Rust binaries:

- **The Tauri app.** Discovers plugins, runs them on a schedule, turns counters into rates, keeps 60 s of history in memory for sparklines, loads themes, and sends samples to the frontend. It contains no metric-collection code of its own.
- **`stats-probe`.** A helper with one subcommand per built-in metric (`stats-probe cpu`, `stats-probe memory`, …). All OS-specific code lives here, which is what keeps a future port to another OS contained.

Plugin model (D14–D18, D26–D28, D31, D33–D34):

- Every metric is a plugin, built-ins included. A plugin is a folder with a `plugin.toml` manifest and an executable. Built-in manifests point at `stats-probe`.
- The app runs the executable once per refresh and reads one JSON object from stdout: `{"value": 42.1}`, or `{"value": null, "reason": "…"}`. A non-zero exit or invalid output also means unavailable.
- Plugins are stateless and report a single number. For rates, the manifest sets `kind = "counter"`, the plugin prints a running total, and the app computes the change per second. CPU usage uses this path too, by printing cumulative percent-seconds.
- The app does no unit scaling. A plugin prints its value in the unit it wants displayed.
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
- **Keep OS-specific code out of the app, frontend, plugin interface and themes.**
- **First version exclusions:** fill bars, multi-value plugins (network is two plugins, `net-down` and `net-up`), always-on-top, remembered window position, a menu bar icon, code signing.

## Unverified risks

These should be prototyped on the MacBook Air M2 before the plugin interface is built out, because they could change it:

- Reading temperature and GPU usage without sudo.
- Getting raw cumulative CPU time for the percent-seconds counter; the `sysinfo` crate may only expose a computed percentage.
- The app's own CPU cost of launching roughly seven processes per refresh cycle at a 1 s interval.
- Changing Tauri window effects at runtime for theme switching, and a right-click menu on a borderless, Dock-less window.

## Distribution

A personal Homebrew tap with a cask pointing at unsigned builds on GitHub Releases; the cask clears the quarantine flag. `macOSPrivateApi` is enabled in Tauri for window transparency, which is acceptable because Mac App Store distribution is not a goal.
