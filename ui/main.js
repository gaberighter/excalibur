// Draws one row per plugin from the samples the engine sends.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const SVG_NS = "http://www.w3.org/2000/svg";
const NO_VALUE = "—";
// Sparklines are redrawn on this beat so they keep sliding between samples.
const REDRAW_MS = 1000;

const metricsEl = document.getElementById("metrics");
const warningsEl = document.getElementById("warnings");

/** name -> { plugin, points, reason, hover, els } */
const rows = new Map();

function el(tag, className, parent) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (parent) parent.append(node);
  return node;
}

function svgEl(tag, className, parent) {
  const node = document.createElementNS(SVG_NS, tag);
  node.setAttribute("class", className);
  parent.append(node);
  return node;
}

function buildRow(plugin, points) {
  const root = el("section", "metric", metricsEl);
  root.setAttribute("role", "listitem");
  const head = el("div", "metric-head", root);
  el("span", "metric-label", head).textContent = plugin.label;
  const age = el("span", "metric-age", head);
  const reading = el("span", "metric-reading", head);
  const value = el("span", "metric-value", reading);
  el("span", "metric-unit", reading).textContent = plugin.unit;

  const row = { plugin, points, reason: null, hover: null, els: { root, age, value } };

  if (plugin.display === "sparkline") {
    const spark = el("div", "spark", root);
    const svg = document.createElementNS(SVG_NS, "svg");
    svg.setAttribute("viewBox", `0 0 ${Sparkline.WIDTH} ${Sparkline.HEIGHT}`);
    svg.setAttribute("preserveAspectRatio", "none");
    svg.setAttribute("aria-hidden", "true");
    spark.append(svg);
    row.els.area = svgEl("path", "spark-area", svg);
    row.els.line = svgEl("path", "spark-line", svg);
    row.els.dot = el("i", "spark-dot", spark);
    row.els.dot.hidden = true;

    spark.addEventListener("mousemove", (event) => {
      const box = spark.getBoundingClientRect();
      row.hover = (event.clientX - box.left) / box.width;
      drawRow(row);
    });
    spark.addEventListener("mouseleave", () => {
      row.hover = null;
      drawRow(row);
    });
  }
  return row;
}

function format(value, decimals) {
  return value.toFixed(decimals);
}

function drawRow(row) {
  const { plugin, points, els } = row;
  const now = Date.now();
  const latest = points[points.length - 1];
  let shown = latest ? latest.value : null;
  let hovered = null;

  if (els.line) {
    const { line, area } = Sparkline.paths(points, now, plugin.min, plugin.max);
    els.line.setAttribute("d", line);
    els.area.setAttribute("d", area);
    if (row.hover !== null) {
      hovered = Sparkline.nearest(points, now, plugin.min, plugin.max, row.hover);
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
  els.value.textContent = unavailable ? NO_VALUE : format(shown, plugin.decimals);
  els.root.classList.toggle("unavailable", unavailable);
  els.root.title = unavailable && row.reason ? row.reason : "";
}

function prune(points, now) {
  while (points.length && points[0].time_ms < now - Sparkline.WINDOW_MS) points.shift();
}

function showWarnings(warnings) {
  warningsEl.hidden = warnings.length === 0;
  warningsEl.textContent = warnings.length === 1 ? "1 warning" : `${warnings.length} warnings`;
  warningsEl.title = warnings.join("\n");
}

/** Rebuilds every row from the engine's current state. */
async function refresh() {
  const snapshot = await invoke("snapshot");
  metricsEl.replaceChildren();
  rows.clear();
  for (const plugin of snapshot.plugins) {
    const row = buildRow(plugin, snapshot.history[plugin.name] ?? []);
    rows.set(plugin.name, row);
    drawRow(row);
  }
  showWarnings(snapshot.warnings);
  await fitWindow();
}

/** Asks for a window exactly as tall as the rows, so none are cut off. */
async function fitWindow() {
  const style = getComputedStyle(document.body);
  const padding = parseFloat(style.paddingTop) + parseFloat(style.paddingBottom);
  let height = metricsEl.scrollHeight + padding;
  if (!warningsEl.hidden) {
    height += warningsEl.offsetHeight + parseFloat(getComputedStyle(warningsEl).marginTop);
  }
  await invoke("fit_height", { height: Math.ceil(height) });
}

function onSample(sample) {
  const row = rows.get(sample.plugin);
  if (!row) return;
  const latest = row.points[row.points.length - 1];
  // Already present if it arrived while the snapshot was being fetched.
  if (latest && sample.time_ms <= latest.time_ms) return;
  row.points.push({ time_ms: sample.time_ms, value: sample.value });
  row.reason = sample.reason;
  prune(row.points, sample.time_ms);
  drawRow(row);
}

async function start() {
  await listen("engine", ({ payload }) => {
    if (payload.type === "sample") onSample(payload);
    // Plugins were loaded or reloaded.
    else refresh();
  });
  await refresh();
  setInterval(() => {
    if (document.hidden) return;
    for (const row of rows.values()) if (row.els.line) drawRow(row);
  }, REDRAW_MS);
}

start();
