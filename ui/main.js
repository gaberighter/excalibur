// Draws the plugins from the samples the engine sends, in the layout the
// active theme asks for.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const SVG_NS = "http://www.w3.org/2000/svg";
const NO_VALUE = "—";
// The view is redrawn on this beat so it keeps moving between samples.
const REDRAW_MS = 1000;

const metricsEl = document.getElementById("metrics");
const warningsEl = document.getElementById("warnings");
const panelEl = document.getElementById("panel");
const themeEl = document.getElementById("theme");
const tipEl = document.getElementById("tip");

/**
 * name -> { plugin, points, reason, min, max, els }
 * `min` and `max` are the scale in force, which arcs fill against; a plugin
 * can report its own with a reading. `els` belongs to whichever view built it.
 */
const metrics = new Map();
/** The theme in use, as sent by the app. */
let theme = null;
// Warnings come from two places and are shown together.
let engineWarnings = [];
let themeWarnings = [];
// Set once the theme and the metrics are both in place. The first fit also
// decides where the window appears, so it must not run on a half-built page.
let laidOut = false;

function el(tag, className, parent) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (parent) parent.append(node);
  return node;
}

function svgEl(tag, className, parent) {
  const node = document.createElementNS(SVG_NS, tag);
  if (className) node.setAttribute("class", className);
  if (parent) parent.append(node);
  return node;
}

function latestValue(metric) {
  const latest = metric.points[metric.points.length - 1];
  return latest ? latest.value : null;
}

/** "23.4%", "38°C", "5.8 GB": a space before units that are words. */
function withUnit(plugin, value) {
  if (value === null) return NO_VALUE;
  const number = value.toFixed(plugin.decimals);
  if (!plugin.unit) return number;
  return /^[A-Za-z]/.test(plugin.unit) ? `${number} ${plugin.unit}` : `${number}${plugin.unit}`;
}

function allWarnings() {
  return [...engineWarnings, ...themeWarnings];
}

function warningCount(warnings) {
  return warnings.length === 1 ? "1 warning" : `${warnings.length} warnings`;
}

/**
 * One row per metric: label, value and an optional sparkline. The `rows`
 * and `compact` layouts share it; the stylesheet puts the sparkline under
 * the label or beside it.
 */
const RowsView = {
  build() {
    let row = 1;
    for (const metric of metrics.values()) this.buildRow(metric, row++);
  },

  buildRow(metric, row) {
    const { plugin } = metric;
    const root = el("section", "metric", metricsEl);
    root.setAttribute("role", "listitem");
    // The compact layout is one grid, and places each part by its row.
    root.style.setProperty("--row", row);
    const head = el("div", "metric-head", root);
    const compact = theme.layout === "compact";
    el("span", "metric-label", head).textContent = compact ? plugin.short : plugin.label;
    const age = el("span", "metric-age", head);
    const reading = el("span", "metric-reading", head);
    const value = el("span", "metric-value", reading);
    el("span", "metric-unit", reading).textContent = plugin.unit;
    metric.els = { root, age, value, hover: null };

    if (plugin.display !== "sparkline") return;
    const spark = el("div", "spark", root);
    const svg = svgEl("svg", "", spark);
    svg.setAttribute("viewBox", `0 0 ${Sparkline.WIDTH} ${Sparkline.HEIGHT}`);
    svg.setAttribute("preserveAspectRatio", "none");
    svg.setAttribute("aria-hidden", "true");
    metric.els.area = svgEl("path", "spark-area", svg);
    metric.els.line = svgEl("path", "spark-line", svg);
    metric.els.dot = el("i", "spark-dot", spark);
    metric.els.dot.hidden = true;

    spark.addEventListener("mousemove", (event) => {
      const box = spark.getBoundingClientRect();
      metric.els.hover = (event.clientX - box.left) / box.width;
      this.draw(metric);
    });
    spark.addEventListener("mouseleave", () => {
      metric.els.hover = null;
      this.draw(metric);
    });
  },

  draw(metric) {
    const { plugin, points, els } = metric;
    const now = Date.now();
    let shown = latestValue(metric);
    let hovered = null;

    if (els.line) {
      const { line, area } = Sparkline.paths(points, now);
      els.line.setAttribute("d", line);
      els.area.setAttribute("d", area);
      if (els.hover !== null) {
        hovered = Sparkline.nearest(points, now, els.hover);
      }
      els.dot.hidden = !hovered;
      if (hovered) {
        els.dot.style.left = `${hovered.left * 100}%`;
        els.dot.style.top = `${hovered.top * 100}%`;
        shown = hovered.point.value;
      }
    }

    // While a point is hovered the readout shows that point and how old it is.
    els.age.textContent = hovered
      ? `${Math.max(0, Math.round((now - hovered.point.time_ms) / 1000))} s ago`
      : "";
    const unavailable = shown === null;
    els.value.textContent = unavailable ? NO_VALUE : shown.toFixed(plugin.decimals);
    els.root.classList.toggle("unavailable", unavailable);
    els.root.dataset.tip = unavailable && metric.reason ? `${plugin.label}: ${metric.reason}` : "";
  },

  /** Sparklines slide with time, so they are redrawn between samples. */
  tick() {
    for (const metric of metrics.values()) if (metric.els.line) this.draw(metric);
  },

  showWarnings(warnings) {
    warningsEl.hidden = warnings.length === 0;
    warningsEl.textContent = warningCount(warnings);
    warningsEl.dataset.tip = warnings.join("\n\n");
  },

  height() {
    const style = getComputedStyle(panelEl);
    let height = metricsEl.scrollHeight + parseFloat(style.paddingTop) + parseFloat(style.paddingBottom);
    if (!warningsEl.hidden) {
      height += warningsEl.offsetHeight + parseFloat(getComputedStyle(warningsEl).marginTop);
    }
    return Math.ceil(height);
  },
};

/** One concentric arc per metric, in a square window. */
const ArcsView = {
  warningEl: null,

  build() {
    const size = theme.width;
    const centre = size / 2;
    const { radii, stroke, fontSize } = Arcs.rings(metrics.size, size);
    const svg = svgEl("svg", "arcs", metricsEl);
    svg.setAttribute("viewBox", `0 0 ${size} ${size}`);
    svg.setAttribute("width", size);
    svg.setAttribute("height", size);
    const defs = svgEl("defs", "", svg);

    // Wide enough that neighbouring rings' hover areas meet.
    const hitWidth = radii.length > 1 ? radii[0] - radii[1] : stroke * 1.5;
    this.svg = svg;
    this.hovered = null;

    let index = 0;
    for (const metric of metrics.values()) {
      const radius = radii[index];
      const group = svgEl("g", "arc", svg);
      // An invisible ring, easier to point at than the arc itself.
      const hit = svgEl("circle", "arc-hit", group);
      hit.setAttribute("cx", centre);
      hit.setAttribute("cy", centre);
      hit.setAttribute("r", radius);
      hit.setAttribute("stroke-width", hitWidth);
      group.addEventListener("mouseenter", () => this.hover(metric));
      group.addEventListener("mouseleave", () => this.hover(null));
      const circle = (className) => {
        const node = svgEl("circle", className, group);
        node.setAttribute("cx", centre);
        node.setAttribute("cy", centre);
        node.setAttribute("r", radius);
        node.setAttribute("stroke-width", stroke);
        // Dashes start at 3 o'clock; turn them back to 12.
        node.setAttribute("transform", `rotate(-90 ${centre} ${centre})`);
        return node;
      };
      const track = circle("arc-track");
      track.setAttribute("stroke-dasharray", Arcs.dashes(radius, 0).track);
      const fill = circle("arc-fill");

      const path = svgEl("path", "", defs);
      path.id = `arc-label-${index}`;
      path.setAttribute("d", Arcs.labelPath(centre, radius));
      const text = svgEl("text", "arc-text", group);
      text.setAttribute("font-size", fontSize);
      // Centres the text on the ring instead of standing it on top.
      text.setAttribute("dy", fontSize * 0.36);
      const onPath = svgEl("textPath", "", text);
      onPath.setAttribute("href", `#${path.id}`);
      onPath.setAttribute("startOffset", "100%");
      onPath.setAttribute("text-anchor", "end");
      svgEl("tspan", "arc-label", onPath).textContent = `${metric.plugin.short} `;
      const value = svgEl("tspan", "arc-value", onPath);

      metric.els = { radius, fill, value, group };
      index += 1;
    }

    // The middle of the disc is free. It names the arc being pointed at,
    // in full, and otherwise says so when something is wrong.
    const centreText = (className, size, y) => {
      const node = svgEl("text", className, svg);
      node.setAttribute("x", centre);
      node.setAttribute("y", y);
      node.setAttribute("text-anchor", "middle");
      node.setAttribute("font-size", size);
      return node;
    };
    this.readoutLabel = centreText("arc-readout-label", fontSize, centre - fontSize * 0.5);
    this.readoutValue = centreText("arc-readout-value", fontSize * 1.5, centre + fontSize * 1.1);
    this.warningEl = centreText("arc-warning", fontSize, centre + fontSize * 0.36);
    this.warningText = svgEl("tspan", "", this.warningEl);
  },

  /** Points at one metric, or at none. */
  hover(metric) {
    if (this.hovered) this.hovered.els.group.classList.remove("hovered");
    this.hovered = metric;
    this.svg.classList.toggle("hovering", metric !== null);
    if (metric) metric.els.group.classList.add("hovered");
    this.drawReadout();
  },

  drawReadout() {
    const metric = this.hovered;
    this.readoutLabel.textContent = metric ? metric.plugin.label : "";
    this.readoutValue.textContent = metric ? withUnit(metric.plugin, latestValue(metric)) : "";
  },

  draw(metric) {
    if (metric === this.hovered) this.drawReadout();
    const { plugin, els } = metric;
    const value = latestValue(metric);
    const filled = Arcs.share(value, metric.min, metric.max, Arcs.peak(metric.points), plugin.invert);
    els.fill.setAttribute("stroke-dasharray", Arcs.dashes(els.radius, filled).fill);
    els.fill.style.display = value === null ? "none" : "";
    els.value.textContent = withUnit(plugin, value);
    els.group.dataset.tip = value === null && metric.reason ? `${plugin.label}: ${metric.reason}` : "";
  },

  /** Arcs measured against a recent peak change as old points leave the window. */
  tick() {
    const now = Date.now();
    for (const metric of metrics.values()) {
      if (metric.max !== null && metric.max !== undefined) continue;
      prune(metric.points, now);
      this.draw(metric);
    }
  },

  showWarnings(warnings) {
    warningsEl.hidden = true;
    if (!this.warningEl) return;
    this.warningText.textContent = warnings.length ? `${warnings.length} !` : "";
    this.warningEl.dataset.tip = warnings.join("\n\n");
  },

  /** The window is square. */
  height() {
    return theme.width;
  },
};

/**
 * The explanation attached to whatever the pointer is over, if any. The
 * system's own tooltips are not used: they do not appear on a window that
 * is not the active one, which is how this window spends its time.
 */
function tipFor(target) {
  for (let node = target; node && node.dataset; node = node.parentNode) {
    if (node.dataset.tip) return node.dataset.tip;
  }
  return "";
}

/** Shows the explanation in the half of the window the pointer is not in. */
function showTip(text, pointerY) {
  tipEl.hidden = !text;
  if (!text) return;
  tipEl.textContent = text;
  tipEl.classList.toggle("at-top", pointerY > window.innerHeight / 2);
}

function view() {
  return theme && theme.layout === "arcs" ? ArcsView : RowsView;
}

function prune(points, now) {
  // The newest point is kept however old, since it is the current reading.
  while (points.length > 1 && points[0].time_ms < now - Sparkline.WINDOW_MS) points.shift();
}

/** Draws everything again from scratch, in the current layout. */
function rebuild() {
  metricsEl.replaceChildren();
  view().build();
  for (const metric of metrics.values()) view().draw(metric);
  view().showWarnings(allWarnings());
}

/** Asks for a window exactly tall enough for its content, so nothing is cut off. */
async function fitWindow() {
  if (!laidOut) return;
  await invoke("fit_height", { content: view().height(), viewport: window.innerHeight });
}

/** Loads the plugins and their history from the engine. */
async function refresh() {
  const snapshot = await invoke("snapshot");
  metrics.clear();
  for (const plugin of snapshot.plugins) {
    metrics.set(plugin.name, {
      plugin,
      points: snapshot.history[plugin.name] ?? [],
      reason: null,
      min: plugin.min,
      max: plugin.max,
      els: null,
    });
  }
  engineWarnings = snapshot.warnings;
  rebuild();
  laidOut = true;
  await fitWindow();
}

/** Puts on a theme: its stylesheet, its layout and the radius from theme.toml. */
async function applyTheme(next) {
  theme = next;
  themeEl.textContent = theme.css;
  const root = document.documentElement;
  root.style.setProperty("--window-radius", `${theme.radius}px`);
  root.dataset.theme = theme.id;
  root.dataset.layout = theme.layout;
  // The user's accent, for themes to use in place of their own colour. A
  // theme on a light surface takes the darker of the two.
  if (theme.accent) {
    root.style.setProperty("--accent", theme.accent.on_dark);
    root.style.setProperty("--accent-on-light", theme.accent.on_light);
  } else {
    root.style.removeProperty("--accent");
    root.style.removeProperty("--accent-on-light");
  }
  themeWarnings = theme.warnings;
  rebuild();
  // A theme can change the size of everything.
  await fitWindow();
}

function onSample(sample) {
  const metric = metrics.get(sample.plugin);
  if (!metric) return;
  const latest = metric.points[metric.points.length - 1];
  // Already present if it arrived while the snapshot was being fetched.
  if (latest && sample.time_ms <= latest.time_ms) return;
  metric.points.push({ time_ms: sample.time_ms, value: sample.value });
  metric.reason = sample.reason;
  metric.min = sample.min;
  metric.max = sample.max;
  prune(metric.points, sample.time_ms);
  view().draw(metric);
}

async function start() {
  await listen("theme", ({ payload }) => applyTheme(payload));
  await applyTheme(await invoke("theme"));
  window.addEventListener("contextmenu", (event) => {
    event.preventDefault();
    invoke("show_menu");
  });
  // There is no title bar, so the whole window is the handle for moving it.
  window.addEventListener("mousedown", (event) => {
    if (event.button === 0) window.__TAURI__.window.getCurrentWindow().startDragging();
  });
  window.addEventListener("mousemove", (event) => showTip(tipFor(event.target), event.clientY));
  document.documentElement.addEventListener("mouseleave", () => showTip(""));
  // The window's width changes with the theme, which can re-wrap the rows.
  window.addEventListener("resize", () => fitWindow());

  await listen("engine", ({ payload }) => {
    if (payload.type === "sample") onSample(payload);
    // Plugins were loaded or reloaded.
    else refresh();
  });
  await refresh();
  setInterval(() => {
    if (!document.hidden) view().tick();
  }, REDRAW_MS);
}

start();
