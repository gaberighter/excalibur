// Sparkline geometry. Pure functions: no DOM, no clock.

const Sparkline = (() => {
  // Every sparkline covers the same time window, whatever the refresh rate (D29).
  const WINDOW_MS = 60_000;
  // Drawing space; the SVG stretches it to the row's size.
  const WIDTH = 100;
  const HEIGHT = 24;
  // Keeps the line's thickness inside the box at the extremes.
  const PAD = 2;

  /** Points inside the window ending at `now`, oldest first. */
  function visible(points, now) {
    return points.filter((p) => p.time_ms >= now - WINDOW_MS && p.time_ms <= now);
  }

  /**
   * The vertical range to draw against. It always fits the data, so small
   * movements stay visible; a plugin's `min` and `max` are not used here (D54).
   */
  function range(points) {
    const values = points.map((p) => p.value).filter((v) => v !== null);
    const lo = Math.min(...values);
    const hi = Math.max(...values);
    if (!Number.isFinite(lo) || !Number.isFinite(hi)) return [0, 1];
    if (hi <= lo) {
      // A flat line: sit on the floor at zero, otherwise in the middle.
      const margin = Math.abs(lo) * 0.1 || 1;
      return lo === 0 ? [0, 1] : [lo - margin, lo + margin];
    }
    return [lo, hi];
  }

  function x(timeMs, now) {
    return ((timeMs - (now - WINDOW_MS)) / WINDOW_MS) * WIDTH;
  }

  function y(value, [lo, hi]) {
    const share = Math.min(1, Math.max(0, (value - lo) / (hi - lo)));
    return PAD + (1 - share) * (HEIGHT - 2 * PAD);
  }

  /** Runs of consecutive points that have a value; a missing value is a gap. */
  function segments(points) {
    const runs = [];
    let run = [];
    for (const point of points) {
      if (point.value === null) {
        if (run.length) runs.push(run);
        run = [];
      } else {
        run.push(point);
      }
    }
    if (run.length) runs.push(run);
    return runs;
  }

  /** SVG path data for the line and the area under it. */
  function paths(points, now) {
    const shown = visible(points, now);
    const scale = range(shown);
    let line = "";
    let area = "";
    for (const run of segments(shown)) {
      const coords = run.map((p) => [x(p.time_ms, now), y(p.value, scale)]);
      // A lone point is drawn as a zero-length stroke, which the round
      // line cap turns into a dot.
      if (coords.length === 1) coords.push(coords[0]);
      const stroke = coords.map(([px, py]) => `${px.toFixed(2)} ${py.toFixed(2)}`).join(" L ");
      const first = coords[0][0].toFixed(2);
      const last = coords[coords.length - 1][0].toFixed(2);
      line += `M ${stroke} `;
      area += `M ${first} ${HEIGHT} L ${stroke} L ${last} ${HEIGHT} Z `;
    }
    return { line: line.trim(), area: area.trim() };
  }

  /**
   * The point with a value nearest to a horizontal position, given as a share
   * of the sparkline's width. Returns its position as shares of width and height.
   */
  function nearest(points, now, xShare) {
    const shown = visible(points, now).filter((p) => p.value !== null);
    if (!shown.length) return null;
    const scale = range(visible(points, now));
    const target = xShare * WIDTH;
    let best = shown[0];
    for (const point of shown) {
      if (Math.abs(x(point.time_ms, now) - target) < Math.abs(x(best.time_ms, now) - target)) {
        best = point;
      }
    }
    return {
      point: best,
      left: x(best.time_ms, now) / WIDTH,
      top: y(best.value, scale) / HEIGHT,
    };
  }

  return { WINDOW_MS, WIDTH, HEIGHT, visible, range, segments, paths, nearest };
})();

if (typeof module !== "undefined") module.exports = Sparkline;
