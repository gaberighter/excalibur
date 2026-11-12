// Run with: node --test ui/arcs.test.js

const test = require("node:test");
const assert = require("node:assert/strict");
const Arcs = require("./arcs.js");

const near = (actual, expected, tolerance = 0.01) =>
  assert.ok(Math.abs(actual - expected) <= tolerance, `${actual} is not near ${expected}`);

test("seven rings fill a 180 px disc from the rim to the inner limit", () => {
  const { radii, stroke, fontSize } = Arcs.rings(7, 180);
  assert.equal(radii.length, 7);
  near(radii[0], 82);
  near(radii[6], 31.5);
  assert.ok(stroke < radii[0] - radii[1], "arcs must not touch");
  assert.ok(fontSize <= radii[0] - radii[1], "labels must not overlap");
});

test("a few rings keep a sensible spacing instead of spreading out", () => {
  const { radii, stroke } = Arcs.rings(2, 180);
  near(radii[0] - radii[1], 9.5);
  near(stroke, 6);
});

test("one ring sits at the rim", () => {
  assert.deepEqual(Arcs.rings(1, 180).radii, [82]);
});

test("many rings get thinner but never go inside the inner limit", () => {
  const { radii, stroke } = Arcs.rings(12, 180);
  near(radii[11], 31.5);
  assert.ok(stroke < radii[0] - radii[1]);
});

test("share is the value's place between min and max", () => {
  assert.equal(Arcs.share(50, 0, 100, null), 0.5);
  assert.equal(Arcs.share(40, 20, 100, null), 0.25);
});

test("share is clamped to the scale", () => {
  assert.equal(Arcs.share(120, 0, 100, null), 1);
  assert.equal(Arcs.share(10, 20, 100, null), 0);
});

test("without a maximum, share is measured against the recent peak", () => {
  assert.equal(Arcs.share(0.6, 0, null, 2.4), 0.25);
  assert.equal(Arcs.share(2.4, undefined, undefined, 2.4), 1);
});

test("share is zero when there is no value or no usable scale", () => {
  assert.equal(Arcs.share(null, 0, 100, null), 0);
  assert.equal(Arcs.share(5, 0, null, null), 0);
  assert.equal(Arcs.share(0, 0, null, 0), 0);
});

test("an inverted metric fills with what is left of its scale", () => {
  assert.equal(Arcs.share(50, 0, 200, null, true), 0.75);
  assert.equal(Arcs.share(200, 0, 200, null, true), 0);
});

test("inverting needs a real maximum", () => {
  assert.equal(Arcs.share(0.6, 0, null, 2.4, true), 0.25);
  assert.equal(Arcs.share(null, 0, 200, null, true), 0);
});

test("peak ignores gaps", () => {
  assert.equal(Arcs.peak([{ value: 1 }, { value: null }, { value: 3 }]), 3);
  assert.equal(Arcs.peak([{ value: null }]), null);
  assert.equal(Arcs.peak([]), null);
});

test("the track covers three quarters of the circle and the fill its share of that", () => {
  const { track, fill } = Arcs.dashes(50, 0.5);
  const around = 2 * Math.PI * 50;
  assert.equal(track, `${(0.75 * around).toFixed(2)} ${around.toFixed(2)}`);
  assert.equal(fill, `${(0.375 * around).toFixed(2)} ${around.toFixed(2)}`);
});

test("the label path runs from 9 o'clock to just short of 12 o'clock", () => {
  const path = Arcs.labelPath(90, 60);
  assert.ok(path.startsWith("M 30.00 90 A 60 60 0 0 1 "), path);
  const [x, y] = path.split(" ").slice(-2).map(Number);
  assert.ok(x < 90 && x > 80, `ends just left of centre, got ${x}`);
  near(Math.hypot(x - 90, y - 90), 60, 0.02);
});

test("inner rings have less room for a label than outer ones", () => {
  assert.ok(Arcs.labelRoom(31.5) > 40);
  assert.ok(Arcs.labelRoom(82) > Arcs.labelRoom(31.5));
});
