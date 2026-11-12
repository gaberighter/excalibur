# Writing a theme

A theme decides how Excalibur's window looks: its colours, fonts and spacing, the material behind it, its width and corners, and which layout the metrics are arranged in. A theme is a folder with two files.

Themes of your own go in `~/.config/excalibur/themes/`, one folder each. Choose **Open Config Folder** from the window's right-click menu to get there.

## A first theme

Create this layout:

```
~/.config/excalibur/themes/
  midnight/
    theme.toml
    theme.css
```

`theme.toml`:

```toml
name   = "Midnight"
effect = "none"
layout = "rows"
radius = 14
width  = 220
```

`theme.css`:

```css
:root {
  --surface: #0d1b2a;
  --surface-solid: #0d1b2a;
  --text: #e0e6ed;
  --text-muted: #8da2b8;
  --spark-line: var(--accent, #f4a261);
  --spark-fill: color-mix(in srgb, var(--spark-line) 18%, transparent);
  --track: rgb(255 255 255 / 0.12);

  --font: 13px/1.3 "Avenir Next", -apple-system, sans-serif;
  --value-weight: 600;
  --pad: 14px;
  --row-gap: 12px;
  --col-gap: 8px;
  --spark-height: 22px;
  --spark-gap: 4px;
  --spark-width: 1.5px;
}
```

Right-click the window. "Midnight" is in the list of themes; there is no need to restart, because the list is read each time the menu opens. After editing a theme's files, pick it from the menu again to see the change.

## `theme.toml`: the window

Every setting is optional. A misspelt setting is reported as a warning and the theme is skipped.

| Setting | Default | Meaning |
|---|---|---|
| `name` | the folder's name | The name shown in the menu. |
| `effect` | `"none"` | The material behind the page. `"none"` gives a plain window, coloured only by the stylesheet. |
| `layout` | `"rows"` | How the metrics are arranged; see below. |
| `radius` | `0` | Corner radius of the window, in points, from 0 to 500. |
| `width` | `220` | Width of the window, in points, from 120 to 1000. |

The folder's name is the theme's id. It is what `theme = "midnight"` in `config.toml` refers to. A theme of yours whose folder has the same name as a built-in one (`frosted`, `frosted-compact`, `frosted-round`, `plain`) replaces it.

The window's height is not set anywhere: it follows the content.

### Effects

On macOS these give the translucent, blurred materials the system uses. The built-in frosted themes use `hudWindow`.

`hudWindow`, `sidebar`, `popover`, `menu`, `sheet`, `titlebar`, `headerView`, `selection`, `tooltip`, `windowBackground`, `contentBackground`, `underWindowBackground`, `underPageBackground`, `fullScreenUI`

With an effect, make `--surface` partly transparent so the material shows through, as the frosted themes do with `rgb(18 20 26 / 0.3)`. An opaque `--surface` hides the effect completely.

### Layouts

| Layout | What it draws |
|---|---|
| `rows` | One row per metric: label and value, with the sparkline underneath. |
| `compact` | One line per metric: short label, sparkline and value side by side. |
| `arcs` | One concentric arc per metric, with its label curved along it. No sparklines. |

The `arcs` layout uses a square window, as tall as it is wide. Set `radius` to half the `width` to make the window a circle, as Frosted Round does with `width = 180` and `radius = 90`.

## `theme.css`: the page

The stylesheet sets custom properties on `:root`. Excalibur's own stylesheet handles layout only and takes every colour, font and measurement from these. A property you leave out has no fallback, so set them all.

| Property | Used for |
|---|---|
| `--surface` | The window's background. |
| `--surface-solid` | An opaque colour close to the surface. The dot that follows the pointer along a sparkline is ringed in it. |
| `--text` | Values. |
| `--text-muted` | Labels, units and the warnings line. |
| `--spark-line` | Sparkline lines, the hover dot and the filled part of arcs. |
| `--spark-fill` | The area under a sparkline. |
| `--track` | The unfilled part of an arc. |
| `--font` | The font, in CSS shorthand: size, line height and family. |
| `--value-weight` | Font weight of values. |
| `--pad` | Space between the window's edge and its content. |
| `--row-gap` | Space between metrics. |
| `--col-gap` | In the `compact` layout, space between the label, sparkline and value. |
| `--spark-height` | Height of a sparkline. |
| `--spark-gap` | In the `rows` layout, space between a row's text and its sparkline. |
| `--spark-width` | Thickness of a sparkline's line. |

### The accent colour

The user can pick an accent colour from the right-click menu, and by default it follows the accent chosen in System Settings. Excalibur hands it to the page as two properties:

| Property | Meaning |
|---|---|
| `--accent` | The accent, in a shade suited to a dark surface. |
| `--accent-on-light` | The accent, in a darker shade suited to a light surface. |

A theme uses the accent by asking for it, with its own colour as the fallback for when the user chooses "Theme Default":

```css
--spark-line: var(--accent, #f4a261);
--spark-fill: color-mix(in srgb, var(--spark-line) 18%, transparent);
```

A theme with a light surface should use `--accent-on-light`. A theme that never mentions either keeps its own colours whatever the user picks.

### Light and dark

A stylesheet can follow the system's appearance with a media query, as the Plain theme does:

```css
@media (prefers-color-scheme: light) {
  :root {
    --surface: #f5f6f8;
    --surface-solid: #f5f6f8;
    --text: #15181e;
    --text-muted: #5d6675;
    --spark-line: var(--accent-on-light, #2563eb);
    --spark-fill: color-mix(in srgb, var(--spark-line) 12%, transparent);
    --track: rgb(0 0 0 / 0.1);
  }
}
```

Most window effects do not change with the system's appearance; `hudWindow` is always dark.

### Readability

Lines and arcs are thin, so they need to stand out. Aim for a contrast of at least 3:1 between `--spark-line` and the surface, and more for `--text`. With a translucent effect the surface is whatever is behind the window, so check the theme over both a bright and a dark desktop.

### Going further

A stylesheet is ordinary CSS and can style the page's elements directly. The rows are `.metric`, with `.metric-label`, `.metric-value`, `.metric-unit` and `.spark` inside; arcs are `.arc-track` and `.arc-fill`. The page's root element carries `data-theme` and `data-layout` attributes. These names are not a promise: they may change between versions, where the properties in the tables above are meant to stay.

## When something is wrong

- **The theme is not in the menu:** the folder has no `theme.toml` or no `theme.css`, or a setting is invalid. There is a "1 warning" line in the window; hover over it for the file and the reason.
- **The window has no colour at all:** the stylesheet is missing properties, or has a syntax error near the top.
- **The effect does not show:** `--surface` is opaque.
- **The configured theme is not used:** if its folder is missing, Excalibur falls back to Frosted and says so in a warning. A theme picked from the menu also takes precedence over `theme` in `config.toml`; see [configuration.md](configuration.md).
