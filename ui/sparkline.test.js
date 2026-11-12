// Run with: node --test ui/sparkline.test.js

const test = require("node:test");
const assert = require("node:assert/strict");
const Sparkline = require("./sparkline.js");

const NOW = 100_000;
const POINTS = [
  { time_ms: 30_000, value: 1 }, // older than the window
  { time_ms: 50_000, value: 0 },
  { time_ms: 70_000, value: null },
  { time_ms: 90_000, value: 10 },
  { time_ms: 100_000, value: 5 },
];

test("points older than 60 s are not drawn", () => {
  assert.equal(Sparkline.visible(POINTS, NOW).length, 4);
});

test("range fits the data when the plugin sets no bounds", () => {
  assert.deepEqual(Sparkline.range(Sparkline.visible(POINTS, NOW)), [0, 10]);
});

test("range uses the plugin's min and max when set", () => {
  assert.deepEqual(Sparkline.range(Sparkline.visible(POINTS, NOW), 0, 100), [0, 100]);
});

test("a flat line sits on the floor at zero and in the middle otherwise", () => {
  assert.deepEqual(Sparkline.range([{ value: 0 }, { value: 0 }]), [0, 1]);
  assert.deepEqual(Sparkline.range([{ value: 5 }, { value: 5 }]), [4.5, 5.5]);
});

test("range of no data is a placeholder", () => {
  assert.deepEqual(Sparkline.range([]), [0, 1]);
  assert.deepEqual(Sparkline.range([{ value: null }], 0), [0, 1]);
});

test("a missing value splits the line into separate runs", () => {
  const runs = Sparkline.segments(Sparkline.visible(POINTS, NOW));
  assert.deepEqual(runs.map((run) => run.length), [1, 2]);
});

test("line places points by time and value, with a gap between runs", () => {
  const { line, area } = Sparkline.paths(POINTS, NOW);
  assert.equal(line, "M 16.67 22.00 L 16.67 22.00 M 83.33 2.00 L 100.00 12.00");
  assert.ok(area.startsWith("M 16.67 24 L 16.67 22.00"));
  assert.ok(area.endsWith("L 100.00 24 Z"));
});

test("values outside the plugin's bounds stay inside the drawing", () => {
  const { line } = Sparkline.paths([{ time_ms: NOW, value: 250 }], NOW, 0, 100);
  assert.equal(line, "M 100.00 2.00 L 100.00 2.00");
});

test("nearest finds the closest point that has a value", () => {
  const hit = Sparkline.nearest(POINTS, NOW, undefined, undefined, 0.7);
  assert.equal(hit.point.time_ms, 90_000);
  assert.ok(Math.abs(hit.left - 0.8333) < 0.001);
});

test("nearest is null when nothing has a value", () => {
  const gap = [{ time_ms: 99_000, value: null }];
  assert.equal(Sparkline.nearest(gap, NOW, undefined, undefined, 0.5), null);
});
