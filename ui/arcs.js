// Geometry for the arcs layout: one concentric arc per metric. Pure
// functions: no DOM, no clock.

const Arcs = (() => {
  // Each arc sweeps three quarters of a turn, clockwise from 12 o'clock.
  // The open quarter, from 9 to 12 o'clock, holds the labels.
  const SWEEP = 0.75;
  // Gap between the edge of the disc and the middle of the outermost arc.
  const EDGE = 8;
  // The innermost arc's radius as a share of the diameter. Smaller than
  // this and its label has too little of the open quarter to fit in.
  const INNER_SHARE = 0.175;
  const MAX_PITCH = 9.5;
  const MAX_STROKE = 6;
  const MAX_FONT = 8;
  // Room left before 12 o'clock for the rounded end of the arc.
  const LABEL_GAP = 6;

  /** Radii, outermost first, with the stroke and text size that suit them. */
  function rings(count, diameter) {
    const outer = diameter / 2 - EDGE;
    const inner = diameter * INNER_SHARE;
    const pitch = count > 1 ? Math.min(MAX_PITCH, (outer - inner) / (count - 1)) : MAX_PITCH;
    return {
      radii: Array.from({ length: count }, (_, i) => outer - i * pitch),
      stroke: Math.min(MAX_STROKE, pitch * 0.64),
      fontSize: Math.min(MAX_FONT, pitch * 0.9),
    };
  }

  /**
   * How full an arc is, from 0 to 1. A metric with no maximum of its own
   * is measured against `peak`, the highest value it has shown recently.
   */
  function share(value, min, max, peak) {
    if (value === null || value === undefined) return 0;
    const lo = min ?? 0;
    const hi = max ?? peak;
    if (!(hi > lo)) return 0;
    return Math.min(1, Math.max(0, (value - lo) / (hi - lo)));
  }

  /** The highest value among recent points, or null if there is none. */
  function peak(points) {
    const values = points.map((p) => p.value).filter((v) => v !== null);
    return values.length ? Math.max(...values) : null;
  }

  /** Dash lengths that draw the track, and the filled part of it. */
  function dashes(radius, filled) {
    const around = 2 * Math.PI * radius;
    const span = SWEEP * around;
    return {
      track: `${span.toFixed(2)} ${around.toFixed(2)}`,
      fill: `${(filled * span).toFixed(2)} ${around.toFixed(2)}`,
    };
  }

  /**
   * A path through the open quarter at this radius, from 9 o'clock to just
   * short of 12 o'clock. Text set on it, anchored at its end, sits beside
   * the start of the arc and cannot leave the disc.
   */
  function labelPath(centre, radius) {
    const end = -Math.PI / 2 - LABEL_GAP / radius;
    const x = centre + radius * Math.cos(end);
    const y = centre + radius * Math.sin(end);
    return `M ${(centre - radius).toFixed(2)} ${centre} A ${radius} ${radius} 0 0 1 ${x.toFixed(2)} ${y.toFixed(2)}`;
  }

  /** The length of that path: how much room a label has. */
  function labelRoom(radius) {
    return (Math.PI / 2) * radius - LABEL_GAP;
  }

  return { SWEEP, rings, share, peak, dashes, labelPath, labelRoom };
})();

if (typeof module !== "undefined") module.exports = Arcs;
