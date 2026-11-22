import { describe, expect, it } from "vitest";
import { beginStroke, emptyBoard, endStroke, extendStroke } from "../src/board.ts";

const p = (x: number, y: number) => ({ x, y });

describe("board", () => {
  it("starts empty", () => {
    expect(emptyBoard()).toEqual({ strokes: [], current: null });
  });

  it("begins a stroke at the given point", () => {
    const b = beginStroke(emptyBoard(), p(1, 2));
    expect(b.current).toEqual({ points: [p(1, 2)] });
    expect(b.strokes).toEqual([]);
  });

  it("extends the current stroke", () => {
    const b = extendStroke(beginStroke(emptyBoard(), p(1, 2)), p(3, 4));
    expect(b.current).toEqual({ points: [p(1, 2), p(3, 4)] });
  });

  it("ignores extend when no stroke is in progress", () => {
    const b = emptyBoard();
    expect(extendStroke(b, p(3, 4))).toBe(b);
  });

  it("commits the current stroke on end", () => {
    const b = endStroke(extendStroke(beginStroke(emptyBoard(), p(1, 2)), p(3, 4)));
    expect(b).toEqual({ strokes: [{ points: [p(1, 2), p(3, 4)] }], current: null });
  });

  it("ignores end when no stroke is in progress", () => {
    const b = emptyBoard();
    expect(endStroke(b)).toBe(b);
  });

  it("does not mutate the previous board", () => {
    const before = beginStroke(emptyBoard(), p(1, 2));
    extendStroke(before, p(3, 4));
    endStroke(before);
    expect(before).toEqual({ strokes: [], current: { points: [p(1, 2)] } });
  });

  it("drops the in-progress stroke when a new one begins", () => {
    const b = beginStroke(beginStroke(emptyBoard(), p(1, 2)), p(5, 6));
    expect(b).toEqual({ strokes: [], current: { points: [p(5, 6)] } });
  });
});
