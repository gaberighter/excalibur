# Configuration

Excalibur works with no configuration at all. Everything here is optional.

## The config folder

Your files live in `~/.config/excalibur/`:

| Path | What it is |
|---|---|
| `config.toml` | Your settings. Excalibur never changes this file. |
| `plugins/` | Plugins of your own; see [plugins.md](plugins.md). |
| `themes/` | Themes of your own; see [themes.md](themes.md). |
| `state.toml` | Written by Excalibur: the theme and accent last picked from the menu. |

Choose **Open Config Folder** from the window's right-click menu. If the folder does not exist, it is created with empty `plugins` and `themes` folders and a `config.toml` in which every setting is present but commented out. Until then, Excalibur writes nothing to your home folder, apart from `state.toml` the first time you pick a theme or accent.

After changing `config.toml`, choose **Reload Plugins** for the plugin settings to take effect. `theme` and `accent` are read when Excalibur starts.

## `config.toml`

```toml
theme    = "frosted"
accent   = "orange"
interval = 2.0
timeout  = 5.0

plugins  = ["cpu", "memory", "temperature", "gpu", "net-down", "net-up", "storage"]
disabled = []
path     = []

[intervals]
cpu     = 1.0
storage = 30.0
```

| Setting | Default | Meaning |
|---|---|---|
| `theme` | `"frosted"` | The theme to start with, by its folder name. The built-in ones are `frosted`, `frosted-compact`, `frosted-round` and `plain`. |
| `accent` | follows the system | The colour of sparklines and arcs: `blue`, `teal`, `green`, `yellow`, `orange`, `red`, `pink`, `purple` or `graphite`; `"system"` for the accent chosen in System Settings; `"theme"` for each theme's own colour; or any colour written like `"#ff8800"`. |
| `interval` | `2.0` | Seconds between refreshes, for plugins that don't set their own. At least 0.1. |
| `timeout` | `5.0` | Seconds before a plugin that is still running is stopped. |
| `plugins` | the built-in order | The order of the rows. Plugins not listed follow in alphabetical order. |
| `disabled` | none | Plugins to hide. |
| `path` | none | Extra folders to search for the commands your plugins run. May start with `~`. |
| `[intervals]` | none | Refresh intervals for individual plugins, in seconds. These override what a plugin asks for. |

A mistake in the file, such as a misspelt setting, makes Excalibur ignore the whole file and use the defaults. The window then shows a "1 warning" line; hover over it to see what is wrong.

## The menu and `config.toml`

A theme or accent picked from the right-click menu is remembered in `state.toml` and takes precedence over `theme` and `accent` in `config.toml`. So once you have picked a theme from the menu, changing `theme` in the config has no visible effect.

To go back to what the config says:

- **Accent:** when `config.toml` sets `accent`, the Accent Color submenu has a "From config.toml" item.
- **Theme:** pick the theme from the menu, or delete `state.toml`.

If you keep your dotfiles in sync between Macs, sync `config.toml`, `plugins/` and `themes/`, and leave `state.toml` out.

## Trying things without touching your config

Set `EXCALIBUR_CONFIG_DIR` to use another folder in place of `~/.config/excalibur`:

```sh
EXCALIBUR_CONFIG_DIR=/tmp/excalibur-test /Applications/Excalibur.app/Contents/MacOS/excalibur
```

To check plugins from a terminal, without a window, print what the engine produces for a few seconds:

```sh
/Applications/Excalibur.app/Contents/MacOS/excalibur --print-samples 5
```
