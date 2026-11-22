import { describe, expect, it } from "vitest";
import {
  beginStroke,
  clearBoard,
  emptyBoard,
  endStroke,
  eraseAt,
  extendStroke,
} from "../src/board.ts";

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

describe("clearBoard", () => {
  it("removes all strokes and the stroke in progress", () => {
    const b = beginStroke(endStroke(beginStroke(emptyBoard(), p(0, 0))), p(1, 1));
    expect(clearBoard(b)).toEqual(emptyBoard());
  });
});

describe("eraseAt", () => {
  const line = (...pts: [number, number][]) => ({ points: pts.map(([x, y]) => p(x, y)) });
  const board = (...strokes: ReturnType<typeof line>[]) => ({ strokes, current: null });

  it("removes a stroke whose segment passes within the radius", () => {
    // Horizontal line from (0,0) to (100,0); eraser at (50,3) is 3px away.
    expect(eraseAt(board(line([0, 0], [100, 0])), p(50, 3), 5).strokes).toEqual([]);
  });

  it("keeps strokes farther than the radius", () => {
    const b = board(line([0, 0], [100, 0]));
    expect(eraseAt(b, p(50, 10), 5)).toBe(b);
  });

  it("measures distance to the segment ends, not the infinite line", () => {
    // (150,0) lies on the line's extension but 50px beyond its end.
    const b = board(line([0, 0], [100, 0]));
    expect(eraseAt(b, p(150, 0), 5)).toBe(b);
  });

  it("hits single-point strokes", () => {
    expect(eraseAt(board(line([10, 10])), p(12, 10), 5).strokes).toEqual([]);
  });

  it("removes only the strokes that are hit", () => {
    const keep = line([0, 50], [100, 50]);
    const b = board(line([0, 0], [100, 0]), keep);
    expect(eraseAt(b, p(50, 0), 5).strokes).toEqual([keep]);
  });

  it("leaves the stroke in progress alone", () => {
    const b = beginStroke(emptyBoard(), p(0, 0));
    expect(eraseAt(b, p(0, 0), 5)).toBe(b);
  });
});
