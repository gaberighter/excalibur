# Decisions

A record of decisions made while scoping the project, to refer back to during implementation. Add new entries at the end and do not renumber. If a decision is reversed, mark the old entry as superseded and link to the new one.

All entries below were decided on 2026-11-11.

## Summary

| # | Area | Decision |
|---|---|---|
| D1 | Form factor | Small floating window |
| D2 | Portability | Cross-platform stack, macOS metric sources only for now |
| D3 | Stack | Rust with Tauri |
| D4 | Frontend | Plain HTML/CSS/JS, no framework |
| D5 | Metrics | CPU, memory, storage, temperature, network, GPU |
| D6 | Sensors | Best effort without sudo |
| D7 | Display | Current values plus in-memory sparklines |
| D8 | Extensibility | Drop-in plugins loaded at runtime |
| D9 | Built-in metrics | Use the same plugin interface as drop-ins |
| D10 | Plugin display | Plugins control how they are drawn, to an extent |
| D11 | Appearance | Frosted glass by default, delivered as a switchable theme |
| D12 | Refresh rate | Configurable, 1–2 s baseline |
| D13 | Installation | Own Homebrew tap |
| D14 | Plugin model | Every plugin is an executable plus a manifest, built-ins included |
| D15 | Plugin execution | Run once per refresh, one JSON object out, no state |
| D16 | Rates | Plugin reports a raw counter; the app computes the rate |
| D17 | Presentations | Value only, or value with sparkline |
| D18 | Refresh rate setting | Global default with per-plugin override |
| D19 | Theme format | Folder with a stylesheet and a settings file |
| D20 | File layout | `~/.config/excalibur` |
| D21 | Built-in delivery | Built-ins ship in the app bundle; the user folder overrides them |
| D22 | Built-in implementation | One Rust helper binary with a subcommand per metric |
| D23 | Window behaviour | Borderless, hidden from the Dock |
| D24 | Controls | Right-click menu on the window |
| D25 | Signing | Unsigned; the cask clears the quarantine flag |
| D26 | Manifest | `plugin.toml` in a folder per plugin |
| D27 | Plugin output | `{"value": …}`, or null with an optional reason |
| D28 | Slow plugins | Skip the tick; kill at a timeout |
| D29 | Sparkline history | Fixed 60 s window |
| D30 | Theme files | `theme.toml` (effect, radius, width) plus `theme.css` |
| D31 | Units | Fixed unit per plugin, shown as printed |
| D32 | Helper name | `stats-probe` |
| D33 | CPU usage | Helper prints cumulative percent-seconds as a counter |
| D34 | Decimal places | Optional `decimals` manifest field, default 1 |
| D35 | `config.toml` | Ordered `plugins` list, `disabled` list, per-plugin intervals |
| D36 | Built-in plugin settings | Unit, decimals, display and interval for each of the seven built-ins |
| D37 | Memory and storage | Memory used; storage free |
| D38 | Command resolution | Plugin folder, then bundled helper, then `PATH` |
| D39 | Second theme | A plain opaque theme ships alongside frosted glass |
| D40 | Remembering the theme | Separate state file; the app never writes `config.toml` |
| D41 | Open config folder | Creates the folders and a fully commented-out `config.toml` |
| D42 | Project name | Excalibur |

## Details

### D1. Form factor: small floating window

A standalone window that can be left open while working.

- **Rejected:** menu bar app, desktop widget, terminal dashboard.
- **Why:** it is the form that ports most easily to other operating systems, and it is a real GUI window as the scope asks for.

### D2. Portability: Mac first, portable later

Use a cross-platform stack, but build only the macOS metric sources now.

- **Rejected:** portable from day one; native macOS only (Swift/SwiftUI).
- **Why:** portability is a bonus in the scope, not a requirement. This keeps the door open without paying for Linux/Windows support before it is needed.
- **Implication:** OS-specific code must stay behind the metric source boundary. The window, plugin interface and themes must not assume macOS.

### D3. Stack: Rust with Tauri

- **Rejected:** Python + Qt (heavy, awkward to package), Electron (a monitor that uses 150+ MB of memory is a poor fit), Go + Wails.
- **Why:** Rust was the stated preference. Tauri gives a small binary, low idle resource use, and supports the transparent and frosted windows needed for D11.

### D4. Frontend: plain HTML/CSS/JS

- **Rejected for now:** Svelte, React or any other framework.
- **Why:** start small. Revisit only if the UI outgrows hand-written code.

### D5. First-version metrics

CPU usage, memory usage, storage, temperature, network throughput, GPU usage.

- **Deferred:** battery, top processes. Both can arrive later as plugins (D8).

### D6. Sensors: best effort without sudo

Temperature and GPU have no official public API on Apple Silicon. Try to read them without administrator rights; if that fails, show the metric as unavailable.

- **Rejected:** `powermetrics` with sudo (needs a password prompt or privileged helper); relying on undocumented APIs as a hard requirement.
- **Implication:** "unavailable" is a normal state that every part of the UI and plugin interface must handle. The app must never prompt for a password.

### D7. Display: current values plus sparklines

Each metric shows its current value and a small graph of recent history, held in memory only.

- **Rejected:** current values only; history persisted to disk (considerably more scope).
- **Implication:** no database and no files written for readings. History is lost on quit, by design.

### D8. Extensibility: drop-in plugins

Metrics load at runtime from a plugins folder, so new ones can be added without rebuilding the app.

- **Rejected:** code-only extensibility (requires a rebuild); config-only (cannot add new metrics).

### D9. Built-in metrics use the same plugin interface

The six first-version metrics are not special-cased. They go through the same interface that a drop-in plugin uses.

- **Why:** one code path to maintain, and the built-ins prove the interface is sufficient.
- **Implication:** if a built-in needs something the interface cannot express, the interface is extended, not bypassed.

### D10. Plugins control how they are drawn, to an extent

A plugin declares how its metric should be presented. The motivating example: some metrics do not need a sparkline and should be able to opt out.

- **Boundary:** plugins choose among presentations the app offers. They do not supply their own arbitrary UI. The available presentations are listed in D17.

### D11. Appearance: frosted glass, as a theme

The default look is frosted glass. The look is abstracted into themes so that a new one can be made later and switched to quickly and easily.

- **Rejected:** hard-coding the frosted look into the app.
- **Implication:** no colours, blur or window styling in the app's own code or base stylesheet; all of it belongs to the active theme. Switching themes should not need a rebuild.

### D12. Refresh rate: configurable, 1–2 s baseline

- **Rejected:** a single fixed rate.
- **Implication:** the sparkline code cannot assume a fixed interval between samples.

### D13. Installation: own Homebrew tap

`brew install` from a personal tap, updates through `brew upgrade`.

- **Rejected:** downloadable `.dmg` only; build from source on each machine.
- **Implication:** needs a GitHub repo for the app, a second repo for the tap, and release builds. Mac App Store distribution is not a goal, which is what allows Tauri's macOS private API setting (see [stack.md](stack.md)).

### D14. Plugin model: every plugin is an executable plus a manifest

A plugin is an executable together with a manifest describing it (name, unit, presentation, suggested refresh rate, whether the value is a counter). Built-ins are plugins of exactly this kind (D9). Resolves O1.

- **Rejected:** one interface with in-process Rust for built-ins and executables for drop-ins; dynamic libraries or WASM modules.
- **Why:** the purest reading of "same interface". Built-ins can be replaced or disabled like any other plugin.
- **Accepted cost:** more processes and slightly higher overhead than in-process collection.

### D15. Plugin execution: run once per refresh

On each tick the app runs the plugin's executable and reads one JSON object from its output. The plugin then exits.

- **Rejected:** long-running plugins that stream one line per sample; supporting both styles.
- **Why:** a plugin can be a few lines of shell script.
- **Implication:** plugins keep no state between runs (see D16). The app must cope with a plugin that is slow, fails or prints something invalid, and show the metric as unavailable (D6).

### D16. Rates: the app computes them

For metrics that are rates, such as CPU usage and network throughput, the plugin prints a raw running total and its manifest marks the value as a counter. The app subtracts consecutive samples and divides by the elapsed time.

- **Rejected:** the plugin pausing briefly to take two readings; the plugin keeping its own state file.
- **Implication:** the first sample of a counter plugin produces no value. The app must handle a counter that resets or goes backwards.

### D17. Presentations: value only, or value with sparkline

A plugin's manifest chooses one of two presentations. Resolves O2.

- **Not in the first version:** value with a fill bar; several named values from one plugin.
- **Implication:** one plugin reports one number. Network is therefore two plugins, upload and download. Storage and memory are shown as a number without a bar.

### D18. Refresh rate: global default with per-plugin override

There is one app-wide default. A plugin's manifest can suggest its own rate, and the user's config can override either. Resolves O3 and refines D12.

- **Rejected:** global only; per plugin only.
- **Why:** slow-changing metrics such as storage should not be polled as often as CPU.

### D19. Theme format: a folder with a stylesheet and a settings file

Each theme is a folder containing a stylesheet (colours, fonts and spacing as CSS variables) and a small settings file for window-level things CSS cannot do, such as the native blur effect. Resolves O4.

- **Rejected:** a single CSS file (cannot control native blur); a settings file only (themes limited to what the app anticipated).

### D20. File layout: `~/.config/excalibur`

One folder holding `config.toml`, `plugins/` and `themes/`. Resolves O5.

- **Rejected:** `~/Library/Application Support`.
- **Why:** easy to find and edit, can be synced between Macs with a dotfiles repo, and the same convention works on Linux.

### D21. Built-ins ship in the app bundle; the user folder overrides them

Built-in plugins and the default theme are loaded from inside the app, so `brew upgrade` updates them. `~/.config/excalibur` holds only the user's additions. A user plugin or theme with the same name as a built-in replaces it, and config can disable any built-in.

- **Rejected:** copying built-ins into the user folder on first launch, with or without overwriting on upgrade.
- **Why:** upgrades never fight with the user's edits.
- **Implication:** there are two places plugins and themes are loaded from, and the user folder wins.

### D22. Built-ins: one Rust helper binary with subcommands

A single small helper binary provides all built-in metrics, one subcommand per metric. Each built-in plugin's manifest points at the helper with the right subcommand.

- **Rejected:** one binary per metric; shell scripts (fragile, macOS-specific, cannot read temperature or GPU).
- **Implication:** this binary is where OS-specific code lives, and where future ports are made (D2). The helper's name is not yet chosen.

### D23. Window behaviour: borderless, hidden from the Dock

No title bar or traffic-light buttons, and no Dock icon or app-switcher entry. Resolves O6.

- **Not in the first version:** always on top; remembering position and size.
- **Implication:** the window must be draggable by its body, and needs another way to quit (D24).

### D24. Controls: right-click menu on the window

Right-clicking the window offers theme switching, reloading plugins, opening the config folder and quitting.

- **Rejected:** a menu bar icon, alone or in addition.
- **Accepted cost:** if the window is buried under others there is no other way to reach it.

### D25. Signing: unsigned, quarantine cleared by the cask

Builds are not signed or notarised. The Homebrew cask removes the quarantine flag after install so the app opens without a Gatekeeper warning. Resolves O7.

- **Rejected:** approving manually on each Mac; paying for an Apple Developer account.
- **Revisit if:** anyone else is going to install the app.

### D26. Manifest: `plugin.toml` in a folder per plugin

Each plugin is a folder containing `plugin.toml` and, if it has one, its own script. Resolves O8.

```
plugins/
  cpu/
    plugin.toml
    run            (optional script)
```

```toml
name     = "cpu"
label    = "CPU"
unit     = "%"
command  = ["stats-probe", "cpu"]
display  = "sparkline"   # or "value"
kind     = "gauge"       # or "counter"
interval = 1.0           # seconds, optional
```

- **Rejected:** `plugin.json`; metadata in header comments of a single script file.
- **Why:** matches `config.toml`, allows comments, and mirrors the theme layout (D30).

### D27. Plugin output: a minimal JSON object

```
{"value": 42.1}
{"value": null, "reason": "sensor not found"}
```

A non-zero exit or invalid output is also treated as unavailable. Label, unit and display stay in the manifest. Resolves O9.

- **Rejected:** per-run overrides of unit or display text; a bare number.

### D28. Slow plugins: skip the tick, kill at a timeout

The app never runs two copies of the same plugin. If one is still running at its next tick, that tick is skipped. If it exceeds a timeout (default 5 s, configurable) it is killed and the metric shows as unavailable until the next successful run. Resolves O10.

- **Rejected:** killing at the next tick (a plugin that is occasionally slightly late would flicker); disabling a plugin after repeated failures.
- **Accepted cost:** a permanently broken plugin is relaunched on every tick until it is removed or disabled in config.

### D29. Sparkline history: fixed 60 s window

Every sparkline shows the last 60 seconds regardless of refresh rate. Slow plugins have fewer points. Resolves O12.

- **Rejected:** a fixed number of points; a configurable window.
- **Implication:** a plugin with an interval of 30 s or more has too few points for a useful sparkline and should use `display = "value"`.

### D30. Theme files: `theme.toml` plus `theme.css`

```
themes/
  frosted/
    theme.toml
    theme.css
```

```toml
name   = "Frosted"
effect = "hudWindow"   # native blur, or "none"
radius = 12            # window corner radius
width  = 220           # default width in px
```

Resolves O11 and completes D19.

- **Rejected:** a settings file limited to the window effect and radius.
- **Why:** a theme can then be compact or roomy as well as differently coloured.

### D31. Units: fixed per plugin, shown as printed

The manifest's `unit` is a label. The app shows the number the plugin produces followed by that label, and does no scaling.

- **Rejected:** the app recognising units such as bytes and scaling them to KB, MB or GB.
- **Implication:** each plugin must print its value already in the unit it wants shown. For counters (D16) the running total must be in that unit too: a network plugin showing MB/s prints megabytes so far, as a decimal.
- **Accepted cost:** one unit must suit the whole range, so small throughput may read as 0.0 MB/s.

### D32. Helper binary name: `stats-probe`

Used as `stats-probe cpu`, `stats-probe memory` and so on. Resolves O13 and completes D22.

- **Rejected:** `stats-gui-probe`; a terse name such as `sgp`.

### D33. CPU usage: cumulative percent-seconds as a counter

`stats-probe cpu` prints cumulative busy time, averaged across cores and scaled so that its change per second of wall-clock time is a percentage. The plugin is an ordinary counter (D16) with `unit = "%"`. Resolves O14.

```
Run at t=0s   {"value": 81234.0}
Run at t=1s   {"value": 81257.5}
App shows     (81257.5 - 81234.0) / 1 s = 23.5 %
```

- **Rejected:** a third `ratio` kind with two running totals (widens the interface); the helper pausing 200 ms to measure (the approach rejected in D16).
- **Accepted cost:** timing jitter can put a reading off by a point or two, including slightly above 100.
- **Implication:** the result needs clamping at 100. The helper cannot do this because it is stateless, so the manifest needs a way to state an upper bound, for example an optional `max` field. This field was not part of D26 and is to be confirmed during implementation.

### D34. Decimal places: optional `decimals` manifest field, default 1

`plugin.toml` may set `decimals`. Temperature can show 48 °C, CPU 23.5 %, throughput 0.42 MB/s. Resolves O15 and extends D26.

- **Rejected:** always one decimal place; automatic selection by the size of the number (the value's width would shift as it changes).

### D35. `config.toml` contents

```toml
# ~/.config/excalibur/config.toml
theme    = "frosted"
interval = 2.0        # global default, seconds
timeout  = 5.0        # plugin timeout, seconds

plugins  = ["cpu", "memory", "temperature",
            "gpu", "net-down", "net-up",
            "storage"]
disabled = []

[intervals]           # per-plugin overrides
cpu     = 1.0
storage = 30.0
```

The `plugins` list sets the order in the window. Plugins not listed are appended alphabetically, so a newly dropped-in plugin appears without editing config. `disabled` hides a plugin, built-in or not. Resolves O16.

- **Rejected:** treating the list as an allow-list; an `order` field in each manifest.
- **Implication:** the app must work with no `config.toml` at all, using these defaults. Interval precedence is: `[intervals]` in config, then the manifest's `interval`, then the global `interval` (D18).

### D36. Built-in plugin settings

| Plugin | Kind | Unit | Decimals | Display | Interval |
|---|---|---|---|---|---|
| `cpu` | counter | % | 1 | sparkline | 1 s |
| `memory` | gauge | GB | 1 | sparkline | default |
| `storage` | gauge | GB | 0 | value | 30 s |
| `temperature` | gauge | °C | 0 | sparkline | default |
| `gpu` | gauge or counter, depending on what the sensor exposes | % | 1 | sparkline | default |
| `net-down` | counter | MB/s | 2 | sparkline | 1 s |
| `net-up` | counter | MB/s | 2 | sparkline | 1 s |

- **Rejected:** memory and storage as percentages; network in Mbit/s.

### D37. Memory reports used; storage reports free

Memory shows the amount in use, as close to Activity Monitor's "Memory Used" as possible. Storage shows the space remaining.

- **Rejected:** used for both; free for both.
- **Why:** for a disk, the usual question is how much room is left.
- **Implication:** the two rows run in opposite directions, so the storage label must say "free" explicitly.

### D38. Command resolution: plugin folder, bundled helper, then `PATH`

A bare command name in a manifest is looked up in the plugin's own folder first, then beside the app's bundled helper, then on `PATH`. Built-ins write `["stats-probe", "cpu"]`; a user plugin writes `["run"]` or `["python3", "probe.py"]`.

- **Rejected:** explicit paths only (plugins using `python3` or `jq` become less portable between Macs); running the command through the shell (an extra process per run, and quoting pitfalls).
- **Implication:** a GUI app launched from Finder or Homebrew gets a minimal `PATH` that usually excludes `/opt/homebrew/bin`. The app needs to account for this, or user plugins relying on Homebrew-installed tools will not be found.

### D39. A plain opaque theme ships alongside frosted glass

A simple solid-background theme with no blur.

- **Rejected:** light and dark frosted variants (the no-blur path would stay untested); frosted only.
- **Why:** it proves nothing about the look is hard-coded, gives the menu something to switch to, and is the simplest example to copy.

### D40. The chosen theme is remembered in a separate state file

Switching theme from the menu writes the choice to the app's own file, `~/.config/excalibur/state.toml`. The app never writes to `config.toml`. The `theme` setting in `config.toml` is the default until a theme is picked from the menu.

- **Rejected:** updating `theme` in `config.toml`; not remembering the choice.
- **Why:** `config.toml` stays hand-edited only, so comments and formatting are safe.
- **Implication:** the state file takes precedence over `config.toml`, so editing `theme` in config has no visible effect once a theme has been chosen from the menu. This needs to be stated in the documentation, and the state file should be left out of any dotfiles sync.

### D41. "Open config folder" creates the folders and a commented config

If `~/.config/excalibur` does not exist, the menu action creates it with empty `plugins/` and `themes/` folders and a `config.toml` in which every setting is present but commented out, showing the defaults.

- **Rejected:** creating empty folders only; creating the folder at first launch (writes to the home folder without being asked).
- **Implication:** an existing `config.toml` is never overwritten. This one-time creation is the single exception to the app not writing `config.toml` (D40).

### D42. Project name: Excalibur

The project was renamed from its working name, `stats_gui`, during phase 0. Earlier entries in this document were updated to the new name.

| Where | Name |
|---|---|
| App and window title | Excalibur |
| Rust package / library | `excalibur` / `excalibur_lib` |
| Bundle identifier | `com.gaberighter.excalibur` |
| Config folder | `~/.config/excalibur` |
| GitHub repository | `gaberighter/excalibur` |
| Helper binary | `stats-probe`, unchanged (D32) |

- **Not renamed:** the local working folder, which is still `stats_gui`.

## Open questions

O1–O16 are all resolved (D14–D35). One point is left to confirm during implementation:

| # | Question | Notes |
|---|---|---|
| O17 | Optional `max` field in `plugin.toml` | Needed to clamp CPU usage at 100 (D33). Not yet agreed as part of the manifest. |
