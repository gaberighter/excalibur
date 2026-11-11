# Implementation Plan

How the first version gets built. Decision numbers (D…) and open questions (O…) refer to [decisions.md](decisions.md); stack details are in [stack.md](stack.md).

## Approach

1. **Test the risky parts first.** Five things in the stack are unverified, and some could change the plugin interface. They are prototyped in phase 1, before anything is built on top of them.
2. **Then build from the bottom up.** The helper (`stats-probe`), then the plugin engine, then the window, then themes and controls, then packaging.
3. **Each phase ends with something that can be checked.** The "done when" line of each phase says what.

## Phases at a glance

| # | Phase | Result |
|---|---|---|
| 0 | Project setup | A repository that builds an empty Tauri window and an empty helper |
| 1 | Risk prototypes | A go/no-go answer for each unverified item |
| 2 | `stats-probe` helper | Seven subcommands printing valid plugin output |
| 3 | Plugin engine | The app runs plugins and produces values and history, tested without a UI |
| 4 | Window and metric display | All metrics visible in a window, updating live |
| 5 | Themes | Frosted default theme, switchable at runtime |
| 6 | Window behaviour and menu | Borderless, Dock-less, right-click menu |
| 7 | Packaging and Homebrew tap | `brew install` works on a second Mac |
| 8 | Documentation | README, plugin and theme guides, commands in CLAUDE.md |

## Phase 0: Project setup

- Initialise git and create the GitHub repository.
- Install prerequisites: Rust toolchain, Xcode Command Line Tools, Tauri CLI.
- Create a Cargo workspace with two members: the Tauri app and `stats-probe` (D22, D32).
- Scaffold the Tauri v2 app with a static HTML/CSS/JS frontend and no bundler (D4).
- Add a `.gitignore` and a minimal README.
- Record the build, run and test commands in `CLAUDE.md`.

**Done when:** `cargo tauri dev` opens an empty window, `cargo run -p stats-probe` runs, and `cargo test` passes with no tests.

## Phase 1: Risk prototypes

Throwaway code on the MacBook Air M2. Each item gets a short written result in `decisions.md`.

| # | Question | If it fails |
|---|---|---|
| 1a | Can temperature be read without sudo? Try `sysinfo` components first, then the IOKit HID sensor interface. | Temperature ships as unavailable (D6). No design change. |
| 1b | Can GPU usage be read without sudo, through IOReport as `macmon` does? | GPU ships as unavailable (D6). No design change. |
| 1c | Can raw cumulative CPU time be read, to print percent-seconds (D33)? Try `sysinfo`, then the macOS host statistics call. | Revisit D33; the rejected alternatives are a `ratio` kind or a short in-plugin pause. |
| 1d | What does it cost to launch about seven processes per second? Measure the app's own CPU use with dummy plugins at a 1 s interval. | Revisit D14/D15. The cheapest change is one `stats-probe` run returning all built-in metrics; the larger one is long-running plugins. |
| 1e | Can Tauri change window effects at runtime, and does a right-click menu work on a borderless, Dock-less window? | Theme switching requires a restart of the window, or the menu moves to a keyboard shortcut. Revisit D24 if needed. |

Also settle here:

- **O17:** confirm the optional `max` field in `plugin.toml`, needed to clamp CPU usage at 100.
- **GPU kind:** whether `gpu` is a gauge or a counter, which depends on what 1b finds (D36).
- **`PATH` for plugins:** how the app finds Homebrew-installed tools when launched outside a terminal (D38).

**Done when:** every row of the risk table has a recorded answer, and any resulting changes to decisions are written down before phase 2 starts.

## Phase 2: `stats-probe` helper

- One subcommand per built-in metric: `cpu`, `memory`, `storage`, `temperature`, `gpu`, `net-down`, `net-up`, with the units, decimals, display and intervals fixed in D36.
- Memory reports the amount used; storage reports the space free, and its label says so (D37).
- Each prints exactly one line in the D27 format and exits. Unavailable sensors print `{"value": null, "reason": "…"}` with exit code 0.
- Values are printed in the display unit (D31); counters print running totals in that unit (D16, D33).
- OS-specific code sits behind one internal boundary per metric, so another OS can be added later without touching the subcommand layer (D2).
- No state, no files written, no sudo (D6, D15).

Tests:

- Every subcommand's output parses as the D27 format.
- Counter subcommands never decrease between two consecutive runs.
- An unknown subcommand exits non-zero.

**Done when:** all seven subcommands run on the M2, and running `cpu` twice a second apart gives a difference that matches Activity Monitor to within a few points.

## Phase 3: Plugin engine

The core of the app, written as plain Rust modules that can be tested without a window.

| Part | Responsibility | Decisions |
|---|---|---|
| Manifest | Parse and validate `plugin.toml`; reject bad manifests with a clear message | D26, D34, O17 |
| Config | Load `~/.config/excalibur/config.toml`; run on defaults if it is missing or invalid | D20, D35 |
| Discovery | Find plugins in the app bundle and the user folder; user folder wins by name; apply `disabled` and ordering | D21, D35 |
| Command resolution | Resolve a manifest's `command`: plugin folder, then bundled helper, then `PATH` | D38 |
| Runner | Launch the executable, capture stdout, parse the D27 object; non-zero exit or bad output means unavailable | D15, D27 |
| Scheduler | Run each plugin at its interval; skip a tick if the previous run is still going; kill at the timeout | D18, D28 |
| Rates | For counters, compute change per second; no value on the first sample; no value if the counter goes backwards | D16 |
| History | Keep the last 60 s of samples per plugin, with timestamps, in memory | D7, D29 |
| Reload | Re-run discovery and restart the scheduler on request | D24 |

Tests, using small shell scripts as fake plugins:

- A command is found in the plugin folder before the bundled helper, and the bundled helper before `PATH`.

- A gauge plugin produces its value.
- A counter plugin produces no value on the first run, then the correct rate.
- A counter that goes backwards produces no value for that sample.
- A plugin that exits non-zero, prints nothing, or prints invalid JSON is unavailable.
- A plugin that sleeps past its interval causes skipped ticks, not overlapping runs.
- A plugin that sleeps past the timeout is killed and shown as unavailable.
- A user plugin with the same name as a built-in replaces it.
- A disabled plugin is not run.
- Ordering follows `plugins`, with unlisted plugins appended alphabetically.
- Interval precedence: config `[intervals]`, then manifest, then global.
- History drops samples older than 60 s.
- A missing `config.toml` gives the D35 defaults.

**Done when:** the tests pass, and a debug command prints live values for all built-in plugins to the terminal.

## Phase 4: Window and metric display

- The backend sends the plugin list (label, unit, display, decimals) and then each new sample to the frontend.
- The frontend draws one row per plugin, in config order: label, value with unit, and a sparkline if the manifest asks for one (D10, D17).
- Sparklines are hand-drawn, cover a fixed 60 s on the horizontal axis, and place points by timestamp so uneven intervals draw correctly (D12, D29).
- An unavailable metric shows a neutral placeholder, with the plugin's reason on hover (D6).
- Values use the manifest's `decimals` (D34) and a fixed-width number style so the layout does not shift.
- The base stylesheet contains layout only, using CSS variables for everything a theme controls (D11).

**Done when:** the window shows all seven metrics updating live, pulling the network cable or disabling a plugin behaves sensibly, and a deliberately broken plugin shows as unavailable without affecting the others.

## Phase 5: Themes

- Parse `theme.toml` (name, effect, radius, width) and load `theme.css` (D19, D30).
- Discover themes in the app bundle and the user folder; user folder wins by name (D21).
- Apply a theme: inject the stylesheet, set the native window effect, corner radius and width.
- Write the default frosted glass theme and ship it in the bundle (D11).
- Write a second, plain opaque theme with no blur and ship it in the bundle (D39).
- Switch themes at runtime, without a restart if 1e allowed it.
- Fall back to the default theme if the configured one is missing or invalid.

**Done when:** switching between the two themes changes blur, colours, radius and width, and an unknown theme name in config falls back cleanly.

## Phase 6: Window behaviour and menu

- Borderless window with a drag region covering the body (D23).
- Hidden from the Dock and the app switcher (D23).
- Right-click menu: switch theme (listing discovered themes), reload plugins, open config folder, quit (D24).
- "Open config folder" creates `~/.config/excalibur/` if it does not exist, with empty `plugins/` and `themes/` folders and a `config.toml` in which every setting is commented out and shows its default. An existing `config.toml` is never overwritten (D41).
- The chosen theme is remembered in `~/.config/excalibur/state.toml`, which takes precedence over `theme` in `config.toml`. The app never writes `config.toml` otherwise (D40).

**Done when:** the app can be moved, re-themed, reloaded and quit using only the window itself, the chosen theme survives a restart, and `config.toml` is byte-for-byte unchanged after a theme switch.

## Phase 7: Packaging and Homebrew tap

- Bundle `stats-probe`, the built-in plugin manifests and the built-in themes inside the `.app` (D21, D22).
- Enable `macOSPrivateApi` and window transparency in the Tauri config.
- Build a release `.app` for Apple Silicon and attach it to a GitHub Release, by hand first.
- Create the tap repository (`homebrew-<name>`) with a cask that installs the app and removes the quarantine flag (D13, D25).
- Automate the release build with GitHub Actions once the manual process works.

**Done when:** on a second Mac, `brew install` from the tap installs the app, it opens without a Gatekeeper warning, all metrics appear, and `brew upgrade` picks up a new release.

## Phase 8: Documentation

- README: what it is, a screenshot, how to install, how to configure.
- Plugin guide: the manifest fields, the output format, gauge versus counter, and a complete example plugin written as a shell script.
- Theme guide: the two files, the CSS variables a theme can set, and the available window effects.
- Update `CLAUDE.md` with the real commands and anything learned during implementation.
- Bring `decisions.md` and `stack.md` up to date: every "proposed" and "to verify" item in `stack.md` becomes confirmed or is replaced.

**Done when:** a new plugin and a new theme can each be written from the guides alone.

## Dependencies between phases

- Phase 1 must finish before phase 2, because its results can change the plugin interface.
- Phases 2 and 3 can overlap: the engine is tested with fake plugins and does not need the helper.
- Phase 4 needs phase 3. Phases 5 and 6 need phase 4.
- Phase 7 can start any time after phase 4, and is worth trying early in a rough form, since bundling a helper binary and resources inside the app is the part most likely to hold surprises.

## Deliberately left out

Everything in the "Out of scope for the first version" list in [project_scope.md](project_scope.md), in particular: battery and top-process plugins, fill bars, multi-value plugins, persisted history, always-on-top, remembered window position, a menu bar icon, code signing, and Linux or Windows support.
