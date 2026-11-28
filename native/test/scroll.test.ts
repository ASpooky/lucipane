import type { ScrollEvent, Stroke } from "@lucipane/contract";
import { describe, expect, it } from "vitest";
import { type Anchor, bounds, isVisible, scrollStrokes, translate } from "../src/scroll.ts";

const p = (x: number, y: number) => ({ x, y });
const freehand = (...points: { x: number; y: number }[]): Stroke => ({
  kind: "freehand",
  author: "user",
  points,
});
const win = { x: 0, y: 100, width: 500, height: 400 };
const scroll = (dy: number, window = 7, rect = win): ScrollEvent => ({ window, rect, dy });
const none = () => undefined;

describe("bounds", () => {
  it("spans the points of a stroke", () => {
    expect(bounds(freehand(p(10, 20), p(40, 5), p(30, 60)))).toEqual({
      x: 10,
      y: 5,
      width: 30,
      height: 55,
    });
  });

  it("covers a text label's box", () => {
    const text: Stroke = { kind: "text", author: "agent", at: p(5, 6), text: "ab" };
    const b = bounds(text);
    expect([b.x, b.y]).toEqual([5, 6]);
    expect(b.width).toBeGreaterThan(0);
    expect(b.height).toBeGreaterThan(0);
  });
});

describe("translate", () => {
  it("moves every kind of stroke down by dy", () => {
    expect(translate(freehand(p(1, 2), p(3, 4)), 10)).toEqual(freehand(p(1, 12), p(3, 14)));
    const box: Stroke = { kind: "box", author: "user", from: p(0, 0), to: p(5, 5) };
    expect(translate(box, -5)).toEqual({ ...box, from: p(0, -5), to: p(5, 0) });
    const text: Stroke = { kind: "text", author: "user", at: p(2, 2), text: "x" };
    expect(translate(text, 3)).toEqual({ ...text, at: p(2, 5) });
  });
});

describe("scrollStrokes", () => {
  it("moves strokes inside the scrolled window and anchors them to it", () => {
    const inside = freehand(p(100, 200), p(120, 220));
    const outside = freehand(p(600, 200), p(620, 220));
    const result = scrollStrokes([inside, outside], none, scroll(-100));
    const [moved, kept] = result.strokes;
    expect(moved).toEqual(freehand(p(100, 100), p(120, 120)));
    expect(kept).toBe(outside);
    expect(result.moved.get(inside)).toBe(moved);
    expect(moved && result.anchors.get(moved)).toEqual({ window: 7, clip: win });
  });

  it("keeps moving anchored strokes with their window, even outside its area", () => {
    const anchor: Anchor = { window: 7, clip: win };
    const gone = freehand(p(100, 0), p(120, 20)); // scrolled above the window already
    const result = scrollStrokes([gone], () => anchor, scroll(150));
    expect(result.strokes[0]).toEqual(freehand(p(100, 150), p(120, 170)));
  });

  it("leaves strokes anchored to another window alone", () => {
    const other: Anchor = { window: 9, clip: win };
    const stroke = freehand(p(100, 200), p(120, 220));
    const result = scrollStrokes([stroke], () => other, scroll(-100));
    expect(result.strokes[0]).toBe(stroke);
    expect(result.moved.size).toBe(0);
  });

  it("returns the same array when nothing follows the scroll", () => {
    const strokes = [freehand(p(600, 200), p(620, 220))];
    expect(scrollStrokes(strokes, none, scroll(-100)).strokes).toBe(strokes);
  });

  it("updates the clip area to where the window is now", () => {
    const stroke = freehand(p(100, 200), p(120, 220));
    const moved = { x: 10, y: 110, width: 500, height: 400 };
    const result = scrollStrokes([stroke], () => ({ window: 7, clip: win }), scroll(0, 7, moved));
    expect(result.strokes[0]).toBe(stroke);
    expect(result.anchors.get(stroke)).toEqual({ window: 7, clip: moved });
  });
});

describe("isVisible", () => {
  const anchor: Anchor = { window: 7, clip: win };

  it("shows strokes that follow no window", () => {
    expect(isVisible(freehand(p(900, 900)), none)).toBe(true);
  });

  it("hides an anchored stroke only once it is entirely outside its window", () => {
    expect(isVisible(freehand(p(10, 90), p(10, 110)), () => anchor)).toBe(true);
    expect(isVisible(freehand(p(10, 20), p(10, 90)), () => anchor)).toBe(false);
  });
});
