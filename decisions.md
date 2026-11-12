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
| D43 | Built-in collection | One `stats-probe` run per refresh cycle for all built-ins (amends D9, D14, D15, D22) |
| D44 | Temperature value | Average of the die temperature sensors |
| D45 | Batched `stats-probe` | Names in, keyed object out; manifests opt in with `batch` |
| D46 | Value bounds | Optional `min` and `max` in the manifest; clamp and sparkline scale |
| D47 | Plugin `PATH` | Common folders appended, plus a `path` setting in config |
| D48 | Always in front | The window floats above other windows (amends D23) |
| D49 | Starting position | Bottom right corner of the screen |
| D50 | Layout | Part of the theme: `layout = "rows"` or `"arcs"` in `theme.toml` |
| D51 | Round window | A circular window of concentric arcs with curved labels, as the Frosted Round theme |
| D52 | Scales | A plugin may report `min` and `max` with a reading; built-ins get fixed scales (extends D27) |
| D53 | Short labels | Optional `short` in the manifest, for layouts with little room |
| D54 | Sparkline scale | Sparklines always fit their own data (amends D46) |
| D55 | Inverted fill | Optional `invert` in the manifest; the storage arc fills with used space |
| D56 | Compact layout | One line per metric, as the Frosted Compact theme |
| D57 | Accent colour | Presets in the right-click menu; one accent for all themes; colours lines and arcs |
| D58 | System accent | An accent option that follows the colour chosen in System Settings |
| D59 | Default accent | A fresh install starts on System Accent |

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
- **Amended by D43:** built-ins are now collected in one batched `stats-probe` run, a path drop-in plugins do not use. Everything after collection is still shared.

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
- **Amended by D43:** the cost was measured in phase 1 and judged too high for the built-ins, which now share one process per cycle. Drop-in plugins are unchanged.

### D15. Plugin execution: run once per refresh

On each tick the app runs the plugin's executable and reads one JSON object from its output. The plugin then exits.

- **Rejected:** long-running plugins that stream one line per sample; supporting both styles.
- **Why:** a plugin can be a few lines of shell script.
- **Implication:** plugins keep no state between runs (see D16). The app must cope with a plugin that is slow, fails or prints something invalid, and show the metric as unavailable (D6).
- **Amended by D43:** for built-ins, one run returns several metrics. It is still run-once and stateless.

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

- **Not in the first version:** remembering position and size. Always on top was also left out here, and was added later by D48.
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
| `gpu` | gauge (confirmed in phase 1: macOS exposes an instantaneous percentage) | % | 1 | sparkline | default |
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

### D43. Built-ins are collected in one `stats-probe` run per refresh cycle

Decided on 2026-11-11 after the phase 1 measurements. The app no longer launches one process per built-in metric. On each cycle it runs `stats-probe` once and gets back the values for every built-in that is due.

- **Why:** launching seven processes per second cost about 5.5 % of one core on the M2 (2.8 % at a 2 s interval), with the processes themselves accounting for almost all of it. That is too much for a monitor to spend on itself.
- **Rejected:** keeping one process per metric; moving all plugins to long-running processes.
- **Amends:** D9, D14, D15 and D22. D22's "one subcommand per metric" becomes one run that can return several metrics.
- **Unchanged:** drop-in plugins still run one executable per plugin and print one D27 object. Built-ins keep their own manifests, names, display, kind, intervals, ordering, and the ability to be disabled or replaced by a user plugin of the same name. Counter-to-rate conversion, history and unavailable handling are still shared by all plugins.
- **Cost:** D9 no longer holds in full. Built-ins reach the app by a batch path that drop-ins cannot use, so the built-ins no longer prove that the drop-in interface is sufficient on its own. The drop-in path needs its own tests.
- **Implication:** batching only saves work if built-ins fall due together, so the scheduler should align intervals to a common tick. A replaced or disabled built-in must be left out of the batch.
- **To be designed:** the exact invocation and output shape (O18).

### D44. Temperature is the average of the die sensors

The M2 exposes 34 temperature sensors and no single "CPU temperature". The `temperature` built-in reports the average of the die sensors (labelled `tdie`).

- **Rejected:** the hottest die sensor.
- **Implication:** if no die sensors are found, as may happen on other Mac models, the metric is unavailable (D6). Sensor labels on other chips need checking before the app is used on another Mac.

### D45. Batched `stats-probe`: names in, keyed object out

The app passes the names of the metrics due this cycle and gets back one JSON object keyed by name, each entry in the D27 shape. Resolves O18 and completes D43.

```
$ stats-probe cpu net-down net-up
{"cpu":      {"value": 1533027.25},
 "net-down": {"value": 4821.6},
 "net-up":   {"value": 310.2}}
```

A manifest opts in with a `batch` field naming its metric, in place of `command`:

```toml
name  = "cpu"
batch = "cpu"      # collected via stats-probe
kind  = "counter"
```

- **Rejected:** `stats-probe` always returning all seven metrics (collects storage every second and metrics that are disabled); a general batch feature for any plugin (more to design, document and test).
- **Implication:** the output is always keyed, even for one name. A user plugin that wants a built-in metric with a different label, unit handling or interval uses `batch` in its own manifest; it does not call `stats-probe` through `command`. A metric that cannot be read is returned as `{"value": null, "reason": "…"}` without affecting the others, and an unknown name exits non-zero.
- **Implication:** a manifest must have exactly one of `command` or `batch`.

### D46. Optional `min` and `max` in the manifest

`plugin.toml` may set `min` and `max`. The app clamps the displayed value to them, and when both are present the sparkline uses them as a fixed vertical scale. The CPU and GPU built-ins set `min = 0` and `max = 100`. Resolves O17 and extends D26.

- **Rejected:** `max` only; no clamping.
- **Why:** CPU readings can land slightly above 100 from timing jitter (D33), and a CPU graph should always span 0–100 instead of stretching to fit its own data.
- **Implication:** without `min` and `max`, a sparkline scales to the data in its 60 s window.

### D47. `PATH` for drop-in plugins: common folders plus a config setting

The app appends `/opt/homebrew/bin` and `/usr/local/bin` to the `PATH` it gives plugins, and `config.toml` can list extra folders in a `path` setting. Resolves O19 and completes D38.

- **Rejected:** asking the login shell for its `PATH` at startup (slower, depends on shell config); a config setting only (Homebrew tools fail until configured).
- **Why:** apps launched from Finder or Homebrew get a minimal `PATH`, so a plugin calling `jq` or a Homebrew `python3` would otherwise not be found.
- **Implication:** tools installed elsewhere, such as under `~/.cargo/bin` or a version manager, need adding to `path` in config. This extends the `config.toml` contents in D35.

### D48. The window is always in front

Decided on 2026-11-11. The window floats above the windows of other apps. Amends D23, which left this out of the first version.

- **Why:** a monitor that gets covered by whatever is being worked on has to be dug out to be read. It also answers the cost accepted in D24, that a buried window could not be reached.
- **Implication:** it is not a setting; the window is always in front. It can cover part of another app, so where it sits matters (D49), and it can be dragged elsewhere.

### D49. The window starts in the bottom right corner

Decided on 2026-11-11. On launch the window appears in the bottom right corner of the screen's usable area (clear of the Dock and the menu bar), 16 px from the edges.

- **Unchanged:** the position is still not remembered between launches (D23). The window can be dragged anywhere, and returns to the corner at the next launch.
- **Implication:** the window is created hidden and shown once its size is known, so it does not appear in one place and jump to another. When the window changes size later (a theme switch or a reload), it is moved only as far as needed to stay fully on the screen.

### D50. The layout belongs to the theme

Decided on 2026-11-11, while looking for ways to make the window take up less of the screen. `theme.toml` gains a `layout` setting: `"rows"` (the default: one row per metric, as built in phase 4) or `"arcs"` (D51).

- **Rejected:** one layout for every theme; a separate layout setting in the menu.
- **Why:** it keeps the rule that the look is never hard-coded (D11), and a roomy theme can sit alongside a compact one in the same menu.
- **Implication:** a layout is code in the app, not something a theme folder can add. A theme chooses among the layouts the app offers, in the same way a plugin chooses among presentations (D10).

### D51. A round window of concentric arcs

Decided on 2026-11-11 from mockups. In the `arcs` layout each metric is one arc, the arcs are concentric, and each sweeps three quarters of a turn clockwise from 12 o'clock. The open quarter holds the labels, and each label follows the curve of its own ring so that it cannot leave the circle. It ships as a third built-in theme, Frosted Round, 180 px across.

- **Considered and not chosen:** one line per metric; two columns of tiles; a grid, a strip or a cluster of small rings; a large CPU dial with the others round its rim; and, for the labels, short straight text, half-turn arcs, and a legend in the middle.
- **How the window is round:** the window stays square. A theme with `radius` set to half its `width` gives both the native material and the page a circular shape, and the corners are transparent. The limit on `radius` was raised from 64 to 500 to allow this.
- **Accepted cost:** this layout shows no history. A plugin's `display` setting is ignored in it.
- **Implication:** with more plugins the arcs and their labels get thinner, and with many they become hard to read. The inner rings have the least room for a label (D53).

### D52. Scales: a plugin may report `min` and `max` with a reading

Decided on 2026-11-11. An arc needs a full mark to fill against. A plugin's output may now carry `min` and `max` beside `value`, and these take precedence over the manifest's:

```
{"value": 5.8, "max": 8.0}
```

- **Why:** some scales depend on the machine, such as the memory installed or the size of the disk, and so cannot be written in a manifest.
- **Extends D27**, which kept the output to `value` and `reason`. Unit, label and display still belong to the manifest only.
- **Built-in scales:** memory from 0 to the memory installed, and storage from 0 to the size of the disk (both reported by `stats-probe`); temperature from 20 to 100 °C, and CPU and GPU from 0 to 100 (in their manifests).
- **Network has no ceiling.** In the arcs layout a metric with no maximum fills against the highest value it has shown in the last 60 seconds.
- **Side effect on the rows layout:** by D46 a sparkline uses a fixed scale when it has both bounds. Memory and temperature now have both, so their sparklines are drawn against the whole scale and look much flatter than before, when they were stretched to fit their own small variations.
- **Storage reads the other way round:** it reports free space (D37), so its arc is fullest when the disk is emptiest.

### D53. Optional `short` label in the manifest

Decided on 2026-11-11. `plugin.toml` may set `short`, a shorter label used by layouts with little room. It defaults to `label`. The built-ins use CPU, Mem, Temp, GPU, Down, Up and Disk. Extends D26.

- **Why:** the innermost ring of the arcs layout has about 43 px for its label and value together.

### D54. Sparklines always fit their own data

Decided on 2026-11-11. A sparkline's vertical scale is always the range of the values it is showing. A plugin's `min` and `max` no longer fix it. Amends D46, where both bounds gave the sparkline a fixed scale.

- **Why:** once memory and temperature were given scales for the arcs (D52), their sparklines were drawn against 0–8 GB and 20–100 °C and became flat lines.
- **Unchanged from D46:** values are still clamped to `min` and `max`, and arcs fill against them.
- **Side effect:** this applies to every sparkline, so the CPU and GPU sparklines, which D46 pinned to 0–100, now stretch to fit as well. A CPU drifting between 1 % and 3 % looks as busy as one swinging between 20 % and 80 %; the number beside it tells them apart.

### D55. Optional `invert` in the manifest

Decided on 2026-11-11. `plugin.toml` may set `invert = true` for a value that counts what is left of its scale, such as free space. A layout that shows how full something is then fills with the rest of the scale. Extends D26.

- **Why:** storage reports free space (D37), so its arc was fullest when the disk was emptiest, the opposite of every other arc.
- **Used by:** the storage built-in. Its arc now fills with the space used, while its number is still the space free, and its short label is "Free" so the two are not confused.
- **Rejected:** showing used space as the number in the arcs layout, which would need a second label and a second value for one plugin.
- **Unchanged:** the rows layout, which shows "Storage free" and a number.
- **Implication:** `invert` needs a maximum. Against a recent peak it is ignored.

### D56. A compact layout: one line per metric

Decided on 2026-11-11, as a second compact option beside the round window (D51), which stays as it is. `layout = "compact"` puts each metric on one line: label, sparkline and value side by side. It ships as a fourth built-in theme, Frosted Compact, 220 px wide.

- **Why:** it keeps everything the rows layout shows, including history, in less than half the height.
- **Details:** labels are the short ones (D53), so storage reads "Free". The label and value columns are as wide as their widest entry, and the sparklines share what is left, so they all line up. While a sparkline is pointed at, the value shows that point; the "12 s ago" note of the rows layout is left out for lack of room.
- **Measured:** 220 × 159 with the seven built-ins, about 43 % of the area of the rows layout.
- **Implementation:** it reuses the rows layout's code and differs only in the stylesheet.

### D57. An accent colour, picked from the menu

Decided on 2026-11-11. The right-click menu gains an "Accent Color" submenu: Theme Default, then nine presets (Blue, Teal, Green, Yellow, Orange, Red, Pink, Purple, Graphite). The four built-in themes (Frosted, Frosted Compact, Frosted Round, Plain) were agreed as the default set at the same time.

- **One accent for all themes.** It follows the user from theme to theme. Rejected: remembering an accent per theme.
- **It colours lines and arcs:** the sparkline and its faint fill, the hover dot, and the filled part of each arc. Text keeps the theme's own colours. Rejected: colouring the values too.
- **Rejected for the picker:** the macOS colour panel, alone or as a "Custom…" item.
- **Any other colour** can be set by hand as `accent = "#ff8800"` in `config.toml`, which also accepts a preset's name. When config sets one, the submenu shows a "From config.toml" item.
- **Remembered in `state.toml`**, where it takes precedence over config, as the theme does (D40).
- **How themes use it:** the app gives the page two custom properties, `--accent` and `--accent-on-light`, and a theme's stylesheet uses them where it wants the accent, falling back to its own colour: `--spark-line: var(--accent, #8ab8ff)`. A theme that ignores them is unaffected.
- **Two colours per preset:** a lighter one for dark surfaces and a darker one for light surfaces, such as Plain in light mode. A test checks that every preset has at least 3:1 contrast against the built-in themes' surfaces. A colour set by hand is used as it is everywhere, and is not checked.

### D58. An accent option that follows the system

Decided on 2026-11-11. The Accent Color submenu gains "System Accent", below Theme Default, and `accent = "system"` is accepted in `config.toml`. It uses the accent colour chosen in the operating system's settings. The choices made for D57 were agreed at the same time.

- **How:** the app reads the setting itself and sends the colour to the page. On macOS that is the `AppleAccentColor` preference, mapped to the eight colours macOS offers; "Multicolour", the default, is blue. It is read again every 2 seconds while System Accent is in use, so a change in System Settings shows up without a restart.
- **Corrected on 2026-11-11.** It was first built on the web's own keyword for the system accent (`AccentColor`), so that the app would need no macOS-specific code. That did not work: web content is given a fixed blue whatever the setting is, which went unnoticed because the machine's accent was blue at the time. The keyword is now only the fallback on systems where the setting cannot be read.
- **This is OS-specific code in the app**, in `system_accent.rs`, which D2 asked to keep out. It is a few lines behind one function, and other systems fall back to the keyword until they get their own reader.
- **Lightened on dark surfaces.** System accents are mid-tones meant for buttons, and are too dark for a thin line on a dark surface, so there the colour is mixed with white (55 % colour). On a light surface it is used as it is.
- **Contrast:** at that mix every macOS accent has at least 3:1 against the frosted surface at its lightest, except purple at about 2.7:1.
- **Checked** from captures with the system accent set to orange: arcs on Frosted Round and sparklines on Frosted are drawn in a light orange. Not checked: that a change made in System Settings is picked up while the app is running.
- **Whether it is the default** was left open here and settled by D59.

### D59. A fresh install starts on System Accent

Decided on 2026-11-11. When no accent has been picked from the menu and none is set in `config.toml`, the accent follows the system (D58). Before this, each theme used its own colour.

- **Theme Default is still in the menu**, and `accent = "theme"` is accepted in config, for each theme's own colour.
- **Implication:** the colours written in the built-in themes' stylesheets are now seen only when Theme Default is chosen, or by a page that does not support the system accent keyword.

## Phase 1 results

Prototypes run on the MacBook Air M2 (macOS 27.2) on 2026-11-11.

| # | Question | Result |
|---|---|---|
| 1a | Temperature without sudo | Works. The `sysinfo` crate returned 34 sensors. See D44. |
| 1b | GPU usage without sudo | Works. The IORegistry entry for the GPU publishes "Device Utilization %". Read with the `ioreg` command; reading it from Rust is still to be written. |
| 1c | Raw CPU time for percent-seconds (D33) | Works, using the macOS host statistics call from the `libc` crate. Readings were in the same range as `top`. The tick counter is 32-bit and wraps after roughly 62 days of uptime, which D16 already handles as one missed sample. |
| 1d | Cost of launching seven processes per second | About 5.5 % of one core at 1 s, 2.8 % at 2 s; slowest cycle 34 ms. Led to D43. |
| 1e | Runtime window effects and right-click menu on a borderless, Dock-less window | Works. Checked by hand: frosted borderless panel with rounded corners, no Dock or app-switcher entry, draggable by its body, right-click menu, switching between a frosted and a plain theme (blur, colour and width) without a restart, and quit from the menu. |

## Phase 2 results

`stats-probe` built and checked on the MacBook Air M2 on 2026-11-11.

| Metric | Source | Check |
|---|---|---|
| `cpu` | Host statistics call (`libc`) | Five one-second differences read 6.8–11.6 %, against 6.5–9.1 % from `top` over the same period. The two were not sampled at the same instants. |
| `memory` | Host VM statistics (`libc`): app memory + wired + compressed | 5.62 GB, matching the same sum computed from `vm_stat`. Not compared against Activity Monitor's window by eye. |
| `storage` | `statvfs` on `/` | 31.3 GB free, matching `df`. |
| `temperature` | `sysinfo` components, average of `tdie` sensors | 33 °C. No independent tool to compare against. |
| `gpu` | `sysinfo`, which reads "Device Utilization %" from the IORegistry | Read 5 % when `ioreg` read 0 % a moment later; the figure is instantaneous and moves between reads, so this is not an exact check. |
| `net-down`, `net-up` | `sysinfo` network totals, summed over `en*` interfaces | Matched `netstat -ib` totals for `en*`. |

Implementation choices made in phase 2 and not previously agreed:

- **Memory is in binary gigabytes** (1024³ bytes), as Activity Monitor uses. **Storage and network are decimal** (10⁹ bytes per GB, 10⁶ per MB), as Finder and most network tools use.
- **Storage free space is what `statvfs` reports.** Finder's "available" figure is larger because it adds space macOS can reclaim on demand.
- **Network counts `en*` interfaces only** (Wi-Fi, Ethernet, tethering). Loopback and VPN tunnels are excluded so VPN traffic is not counted twice.
- **GPU comes from `sysinfo`**, which already reads the IORegistry value found in phase 1, so no reader of our own was needed.

Cost, measured with the release build:

| What | CPU time per run |
|---|---|
| Launching any process (`/usr/bin/true`) | about 3 ms |
| `stats-probe` with one cheap metric | about 8 ms |
| `stats-probe` with all seven metrics | about 20 ms, of which temperature is about 9 ms |

One batched run per second with all seven metrics costs about 2.3 % of one core, against 5.5 % for seven separate processes. Temperature is the expensive metric and its run took up to 85 ms of wall-clock time; refreshing it less often than the others would lower the cost further.

## Phase 3 results

The plugin engine was built on 2026-11-11 as `src-tauri/src/engine/`, with 67 tests. It is not yet connected to the window.

Run from a terminal with the seven built-in plugins at their D36 intervals, the engine and its helper runs together used 0.50 s of CPU time in 30 s, about 1.7 % of one core (release build).

Implementation choices made in phase 3, agreed on 2026-11-11:

- **Manifest defaults.** `name` defaults to the folder name, `label` to the name, `unit` to empty, `display` to `"value"`, `kind` to `"gauge"` and `decimals` to 1. A manifest with an unknown field is rejected, so a misspelt setting is reported instead of silently ignored.
- **Shortest interval is 0.1 s**, in manifests and in config.
- **A `command` containing a `/` is a path** relative to the plugin's folder; D38's lookup order applies to bare names only.
- **An invalid `config.toml` is ignored as a whole**, with a warning, and the defaults are used.
- **`path` entries in config may start with `~`.**
- **Plugins first run immediately at start**, then at multiples of their interval counted from that moment, so plugins sharing an interval fall due together.
- **A timed-out plugin is killed along with anything it started.** The same clean-up happens after a normal exit, so a background process left behind by a script cannot hold a run open.
- **A counter starts again after being unavailable**: the next reading produces no value, as at start-up.
- **Moments with no value are kept in the history as gaps**, so a sparkline can show a break instead of joining across it.
- **Reload keeps the history** of plugins that still exist and resets their counters.
- **Warnings** (invalid manifests, invalid config, duplicate names) are collected and handed to the window. Nothing shows them yet.

## Phase 4 results

The window was connected to the engine on 2026-11-11. Checked from captures of the app's own window: seven rows with live values and units, sparklines on six of them, and a hover readout.

- **The window's height follows its content.** A fixed height cut off the last row, because a title bar takes part of the window. The page now reports what it needs and what it has, and the window grows or shrinks by the difference.
- **Warnings are shown** as a "2 warnings" line at the bottom of the window, with the details on hover.
- **CPU agrees with `top`**: 22.2 % against 22.3 % averaged over the same 20 seconds.
- **Not resolved:** memory was reported as reading a little lower than Activity Monitor. The helper's figure matches the documented sum computed from `vm_stat`; the two have not been compared at the same moment.

## Phase 5 results

Themes were built on 2026-11-11. Checked from captures of the app's own window:

| Case | Result |
|---|---|
| No config | Frosted theme: translucent material, 220 px wide. |
| `theme = "plain"` | Plain theme: solid dark panel, 260 px wide, roomier spacing. |
| `theme = "does-not-exist"` | Falls back to frosted and shows "1 warning". |

**Changed on 2026-11-11:** the Plain theme was given the same width and spacing as Frosted, so both windows are now 220 px wide and the same height. The 260 px figure above is how Plain was first built.

Switching themes while the app is running goes through the right-click menu. It was checked by hand on 2026-11-11: the menu lists both themes with the current one ticked, and switching in either direction changes the material, colours, width and spacing without a restart, with all rows still fitting.

Implementation choices made in phase 5 and not previously agreed:

- **The theme part of the right-click menu was brought forward from phase 6**, because without it there is no way to switch themes while the app is running. The rest of the menu is still phase 6, and the choice is not remembered between launches until then (D40).
- **A theme's id is its folder name.** `theme` in config and the menu refer to it; `name` in `theme.toml` is only what the menu shows.
- **`effect` accepts any window material Tauri knows**, written as in Tauri's own configuration (for example `hudWindow`, `sidebar`, `underWindowBackground`), or `none`. Defaults: no effect, radius 0, width 220.
- **Limits:** radius 0–64, width 120–1000. An unknown setting in `theme.toml` is rejected, as in manifests.
- **The material stays translucent when another app is in front.** By default macOS dulls it when the window is inactive, which is how a monitor window spends most of its time.
- **The frosted theme is also compiled into the app**, so there is still a look if no theme folder can be read.
- **Themes are re-read each time the menu opens**, so a new theme folder appears without a restart.
- **The radius in `theme.toml` is handed to the page** as `--window-radius`, so the page's corners and the window's cannot disagree.
- **`EXCALIBUR_CONFIG_DIR`** overrides `~/.config/excalibur`. It was added so themes and config can be tried without touching the real folder.
- **Theme warnings** are shown together with plugin warnings.

The custom properties a theme's stylesheet sets are: `--surface`, `--surface-solid`, `--text`, `--text-muted`, `--spark-line`, `--spark-fill`, `--font`, `--value-weight`, `--pad`, `--row-gap`, `--spark-height`, `--spark-gap` and `--spark-width`.

## Phase 6 results

Window behaviour and the menu were built on 2026-11-11. Checked from captures of the app's own window and from what macOS reports about it:

| Check | Result |
|---|---|
| Borderless | No title bar; rounded corners in both themes. |
| Hidden from the Dock | macOS reports the app as a background-style app with no Dock icon. |
| Always in front (D48) | The window is on the floating layer. |
| Starting position (D49) | Bottom right corner, 16 px from the right and bottom edges, in both themes. |
| Remembered theme (D40) | With `theme = "frosted"` in config and `theme = "plain"` in `state.toml`, the app starts in Plain. |
| Nothing written unasked | Launching with an empty folder leaves it empty. |

Dragging, the menu items (Reload Plugins, Open Config Folder, Quit Excalibur) and remembering a theme picked from the menu need a right-click and have to be checked by hand. The user tried the window on 2026-11-11 and reported that it looks good, without reporting on each of these individually.

Implementation choices made in phase 6, agreed "for now" on 2026-11-11:

- **The whole window is the drag handle**, including the sparklines.
- **The window cannot be resized by hand.** Its width comes from the theme and its height from its rows.
- **If the page has not reported its size 3 s after launch, the window is shown anyway**, so a page that fails to load cannot leave an app with no Dock icon and no window.
- **Reload Plugins re-reads `config.toml` and the plugin folders.** It does not change the theme.
- **`state.toml` is created the first time a theme is picked from the menu**, along with `~/.config/excalibur` if it does not exist.
- **The commented `config.toml`** written by Open Config Folder is kept in `src-tauri/config.template.toml`; a test checks that it matches the real defaults.

A bug found and fixed: the page's surface was painted on `<body>`, which browsers spread over the whole window, so the corners outside the rounded shape were filled in. It is now painted on an inner panel.

## Compact layout results

The arcs layout and the Frosted Round theme were built on 2026-11-11 (D50–D53). Checked from a capture of the app's own window: a circular frosted window 180 px across, in the bottom right corner, with seven arcs and their curved labels inside the circle. At 180 × 180 it covers about 31 % of the area of the rows layout (220 × 374).

Checked by hand on 2026-11-11: clicks on the transparent corners of the square window pass through to what is behind, and dragging and the right-click menu work on the round window.

Implementation choices not previously agreed:

- **Frosted Round is a new theme**; Frosted and Plain keep the rows layout, and the default is still Frosted.
- **Units are always shown in the arcs layout**, joined to the value with a space when the unit is a word ("5.8 GB") and without one otherwise ("9.4%", "38°C").
- **An unavailable metric** shows its label and a dash, with an empty track.
- **Pointing at an arc** names it in full in the middle of the disc, with its value, and dims the other arcs. This replaced a system tooltip, which did not appear when tried by hand.
- **Warnings** are shown in the arcs layout as a count in the middle of the disc, with the details on hover.
- **A new theme variable, `--track`**, is the colour of the unfilled part of an arc.

## Accent colour results

Built on 2026-11-11 (D57). Checked from captures of the app's own window: the Orange preset on Frosted Round, a hand-written colour (`#ff3ea5`) on Frosted Compact, and a remembered Green taking precedence over Red in config on Frosted. The submenu itself needs a right-click and has not been checked by hand.

## Phase 7 results: packaging

The app was packaged on 2026-11-11 with `scripts/build-release.sh`, and version 0.1.0 was published on 2026-11-12 (see "Phase 7 results: release and tap" below).

| Check | Result |
|---|---|
| Contents | `Excalibur.app` holds the app binary, and as resources the `stats-probe` helper (still executable), the seven built-in plugins and the four built-in themes. |
| Size | 11.3 MB as an app, 3.4 MB zipped. |
| Runs away from the source tree | Unzipped into another folder, the engine loads all seven built-ins and reports values, and the window appears in the configured theme. |
| No Dock icon | macOS reports the packaged app as a background-style app. |
| Signature | Valid ad hoc signature over the whole bundle. |
| Nothing written unasked | The config folder it was pointed at was left as it was. |

Implementation choices made in packaging and not previously agreed:

- **The helper is a resource of the app**, at `Contents/Resources/stats-probe`, not one of Tauri's "sidecar" binaries. A sidecar has to exist before the app can be compiled at all, which would make a fresh checkout unable to build or run its tests without an extra step.
- **Bundling settings are in a second config file**, `src-tauri/tauri.bundle.conf.json`, applied only by the release script. Building the app without it gives an app with no built-in plugins.
- **A release build never looks in the source tree.** Without its resources it has no built-in plugins and only the frosted theme compiled into it.
- **The bundle is ad hoc signed.** This is not the Developer ID signing that D25 left out: it costs nothing, identifies no one, and does not satisfy Gatekeeper. It makes the bundle internally consistent, which Apple Silicon expects.
- **`LSUIElement` is set in `Info.plist`**, so there is no Dock icon even for the moment before the app's own code runs (D23).
- **The release archive is a zip** of the app, named `Excalibur-<version>-<architecture>.zip`. No `.dmg` is built.
- **The app's icon is still Tauri's default.** It is seen only in Finder, since the app has no Dock icon.

## Phase 7 results: release and tap

Version 0.1.0 was published on 2026-11-12.

- **The app repository is public.** Homebrew cannot download a release from a private repository without a token on every Mac. Rejected: keeping the source private and attaching the app to the tap's releases; keeping both private.
- **The tap is `gaberighter/homebrew-tap`**, a general tap for other tools later, chosen over one named for this app. Its working copy is in `~/dev/homebrew-tap`.
- **Install command:** `brew install --cask gaberighter/tap/excalibur`.

| Check | Result |
|---|---|
| Release | `v0.1.0` on GitHub with `Excalibur-0.1.0-arm64.zip`; the published file's checksum matches the local build. |
| Install | `brew install --cask` from the tap put `Excalibur.app` in `/Applications` on the MacBook Air M2. |
| Quarantine | The installed app has no quarantine flag (D25). |
| Runs | Opened from `/Applications`, it shows its window with live values, using the theme and accent remembered in `~/.config/excalibur`. |
| Cask quality | `brew audit` and `brew style` report nothing. |

Not checked: installing on a second Mac, and `brew upgrade` picking up a later release. Both were in the plan's "done when" for this phase. Not done: building releases automatically with GitHub Actions.

Choices made in the cask and not previously agreed:

- **Apple Silicon only**, and **macOS 13 (Ventura) or later**, which is the oldest release whose web view has the colour features the themes use. Neither has been tested on anything but this Mac.
- **`brew uninstall --zap` removes `~/.config/excalibur`**; a plain uninstall leaves it.
- **The quarantine flag is cleared with a `postflight_steps` block**, the form current Homebrew asks for.

## Phase 8 results: documentation

Written on 2026-11-12: a README for users, and three guides in `docs/` (configuration, writing a plugin, writing a theme). `stack.md` was rewritten to describe the stack as built; it no longer has anything marked proposed or to verify.

The plan's check for this phase was that a plugin and a theme can each be written from the guides alone. The guides' examples were created exactly as written, in a scratch config folder, and run:

| Example | Result |
|---|---|
| Battery plugin (a gauge) | Loads and reports the charge. |
| Packets plugin (a counter) | Shows a dash for its first reading, then a rate. |
| A user plugin replacing the built-in `cpu` through `batch` | Replaces it, with its own label and interval. |
| Midnight theme | Appears and draws as described, with the example plugins. |
| `config.toml` settings (`disabled`, `path`, `[intervals]`) | Behave as documented. |

One thing the guides promised turned out not to be true, and was fixed in the app:

- **Explanations are drawn inside the window.** The reason for a missing value, and the text of warnings, were system tooltips, which do not appear on a window that is not the active one. They are now a small note drawn inside the window, at the end away from the pointer. Checked by rendering the page with the note forced on; not checked with a real pointer.
- **This is not in version 0.1.0**, which was published before it. Until a later release, the installed app shows a dash or a warning count with no explanation.

## Version 0.1.1

Published on 2026-11-12, with the in-window explanation notes and the guides.

| Check | Result |
|---|---|
| Release | `v0.1.1` on GitHub; the published file's checksum matches the local build. |
| Upgrade | After the cask was updated, `brew outdated` listed Excalibur and `brew upgrade --cask excalibur` replaced 0.1.0 with 0.1.1 in `/Applications`. This is the first time an upgrade has been tried. |
| Runs | The upgraded app opens and shows its window in the remembered theme, with no quarantine flag. |

Homebrew warned during the upgrade that the app's signer had changed and that macOS may prompt at the next launch. Each build's ad hoc signature is different, so this warning will come with every upgrade. The app opened when launched from a terminal straight afterwards; whether a prompt appeared on screen was not observed.

Still not checked: an install on a second Mac.

## Open questions

None. O1–O19 are all resolved: O1–O16 by D14–D35, O17 by D46, O18 by D45 and O19 by D47.
