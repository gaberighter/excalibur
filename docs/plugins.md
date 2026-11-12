# Writing a plugin

Every row in Excalibur's window is a plugin. A plugin is a folder holding two things: a small program that prints one number each time it is run, and a `plugin.toml` file that says how to show it. Any language will do, including a shell script.

Plugins of your own go in `~/.config/excalibur/plugins/`, one folder each. Choose **Open Config Folder** from the window's right-click menu to get there; it creates the folder if you don't have one yet.

## A first plugin: battery charge

Create this layout:

```
~/.config/excalibur/plugins/
  battery/
    plugin.toml
    run
```

`plugin.toml`:

```toml
label    = "Battery"
short    = "Batt"
unit     = "%"
command  = ["run"]
display  = "value"
interval = 30.0
decimals = 0
min      = 0
max      = 100
```

`run`:

```sh
#!/bin/sh
# Prints the battery's charge as a percentage.
percent=$(pmset -g batt | grep -Eo '[0-9]+%' | head -1 | tr -d '%')
if [ -n "$percent" ]; then
  echo "{\"value\": $percent}"
else
  echo '{"value": null, "reason": "no battery found"}'
fi
```

Make the script executable, or Excalibur will not find it:

```sh
chmod +x ~/.config/excalibur/plugins/battery/run
```

Then choose **Reload Plugins** from the right-click menu. A "Battery" row appears at the bottom of the window.

Before reloading, you can check the script on its own. It should print one line:

```
$ ~/.config/excalibur/plugins/battery/run
{"value": 87}
```

## What the program prints

One JSON object on standard output, then it exits.

| Output | Meaning |
|---|---|
| `{"value": 42.1}` | The reading. |
| `{"value": null, "reason": "sensor not found"}` | No reading this time. The row shows a dash, and the reason on hover. |
| `{"value": 5.8, "max": 8}` | The reading, with the scale it sits on. `min` works the same way. |

`min` and `max` in the output are for scales that depend on the machine, such as the memory installed. They take precedence over the ones in `plugin.toml`.

The row also shows a dash if the program exits with a non-zero status, prints something that is not this JSON, cannot be found, or runs past the timeout.

The value is shown as printed. Excalibur does not convert units, so print megabytes if you want megabytes.

## `plugin.toml`

Only `command` is required. A misspelt setting is reported as a warning and the plugin is skipped.

| Setting | Default | Meaning |
|---|---|---|
| `command` | (required) | The program to run and its arguments, as a list: `["run"]` or `["python3", "probe.py"]`. |
| `name` | the folder's name | What `config.toml` calls the plugin. Letters, digits, `-` and `_` only. |
| `label` | `name` | The text shown beside the value. |
| `short` | `label` | A shorter label, used by the compact and round layouts. |
| `unit` | none | Shown after the value. |
| `display` | `"value"` | `"value"` for a number only, or `"sparkline"` for a number with a graph of the last 60 seconds. |
| `kind` | `"gauge"` | `"gauge"` shows the number as printed. `"counter"` is for running totals; see below. |
| `interval` | the global interval | Seconds between runs. At least 0.1. |
| `decimals` | `1` | Decimal places shown, from 0 to 6. |
| `min`, `max` | none | The scale. Values outside it are clamped, and the round layout fills its arcs against it. |
| `invert` | `false` | Set to `true` if the value counts what is left, such as free space. The round layout then fills its arc with the rest, so it shows what is used. Needs a `max`. |

A sparkline always covers the last 60 seconds. A plugin that runs every 30 seconds has only two points to draw, so give slow plugins `display = "value"`.

## Rates: `kind = "counter"`

A plugin cannot remember anything between runs, so it cannot work out a rate, such as bytes per second, by itself. Print the running total and set `kind = "counter"`; Excalibur subtracts each reading from the next and divides by the time between them.

This plugin shows packets received per second:

`plugin.toml`:

```toml
label    = "Packets in"
short    = "Pkts"
unit     = "/s"
command  = ["run"]
kind     = "counter"
display  = "sparkline"
interval = 1.0
decimals = 0
```

`run`:

```sh
#!/bin/sh
# Prints the number of packets received on en0 since the Mac started.
total=$(netstat -ib | awk '$1 == "en0" && $3 ~ /Link/ { print $5; exit }')
echo "{\"value\": ${total:-null}}"
```

A counter shows a dash for its first reading, because it needs two. It also shows a dash for one reading if the total goes down, which happens when a counter is reset.

Print the total in the unit you want per second. For megabytes per second, print megabytes.

## How the program is run

- **Where the command is looked for.** A bare name, such as `run` or `jq`, is looked for in the plugin's own folder, then among Excalibur's own files, then on the `PATH`. A name containing a `/` is a path from the plugin's folder.
- **The `PATH`.** Apps opened from Finder or Homebrew get a short `PATH`. Excalibur adds `/opt/homebrew/bin` and `/usr/local/bin`. For anything installed elsewhere, list the folder under `path` in `config.toml`; see [configuration.md](configuration.md).
- **The working folder** is the plugin's own folder, so the program can read files beside it.
- **Once per interval.** If a run is still going when the next is due, that one is skipped; two copies never run together.
- **Timeout.** A run still going after 5 seconds is stopped, along with anything it started. The time is `timeout` in `config.toml`.
- **Never as administrator.** Excalibur does not ask for a password, so a plugin cannot use `sudo`.

Each run starts a process. A handful of plugins running every second or two is fine; dozens will show up in Excalibur's own CPU use.

## The built-in plugins

Excalibur ships seven: `cpu`, `memory`, `temperature`, `gpu`, `net-down`, `net-up` and `storage`. They are ordinary plugins, collected by a helper called `stats-probe`.

- **Hide one:** list it under `disabled` in `config.toml`.
- **Reorder them:** list names under `plugins` in `config.toml`.
- **Replace one:** make a plugin of your own with the same `name`. Yours is used in its place.
- **Change how one is shown:** copy its settings into a plugin of your own with the same `name`, and use `batch` in place of `command` to keep the built-in reading:

```toml
# ~/.config/excalibur/plugins/cpu/plugin.toml
label    = "Processor"
short    = "CPU"
unit     = "%"
batch    = "cpu"        # the built-in reading of this name
kind     = "counter"
display  = "value"
interval = 2.0
min      = 0
max      = 100
```

`batch` takes one of the seven names above. A plugin has either `command` or `batch`, never both. The built-in CPU and network readings are running totals, so keep `kind = "counter"` for `cpu`, `net-down` and `net-up`.

## When something is wrong

- **"1 warning" at the bottom of the window** (or a count in the middle of the round one): hover over it. It names the file and what is wrong with it.
- **A dash where the value should be:** hover over the row for the reason.
- **The row is not there at all:** the folder has no `plugin.toml`, the plugin is listed under `disabled`, or its manifest is invalid, in which case there is a warning.
- **"command 'run' was not found":** the script is not executable (`chmod +x`), or the command is not on the `PATH` Excalibur uses.
