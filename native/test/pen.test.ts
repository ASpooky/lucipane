import { describe, expect, it } from "vitest";
import { beginStroke, emptyBoard } from "../src/board.ts";
import { applyPenEvent } from "../src/pen.ts";

describe("applyPenEvent", () => {
  it("maps down/move/up to a committed stroke", () => {
    let b = emptyBoard();
    b = applyPenEvent(b, { kind: "down", x: 1, y: 2 });
    b = applyPenEvent(b, { kind: "move", x: 3, y: 4 });
    b = applyPenEvent(b, { kind: "up" });
    expect(b).toEqual({
      strokes: [
        {
          kind: "freehand",
          points: [
            { x: 1, y: 2 },
            { x: 3, y: 4 },
          ],
        },
      ],
      current: null,
    });
  });

  it("keeps the stroke in progress on move", () => {
    const b = applyPenEvent(beginStroke(emptyBoard(), { x: 0, y: 0 }), {
      kind: "move",
      x: 5,
      y: 5,
    });
    expect(b.current).toEqual({
      kind: "freehand",
      points: [
        { x: 0, y: 0 },
        { x: 5, y: 5 },
      ],
    });
  });
});
