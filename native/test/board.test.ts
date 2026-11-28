import { BoardSnapshot, CONTRACT_VERSION } from "@lucipane/contract";
import { describe, expect, it } from "vitest";
import {
  addStroke,
  beginArrow,
  beginBox,
  beginStroke,
  clearAuthor,
  clearBoard,
  emptyBoard,
  endStroke,
  eraseAt,
  extendStroke,
  outline,
  TEXT_LINE_HEIGHT,
  TEXT_SIZE,
  textBounds,
  toSnapshot,
  userStrokesChanged,
} from "../src/board.ts";

const p = (x: number, y: number) => ({ x, y });

describe("board", () => {
  it("starts empty", () => {
    expect(emptyBoard()).toEqual({ strokes: [], current: null });
  });

  it("begins a stroke at the given point", () => {
    const b = beginStroke(emptyBoard(), p(1, 2));
    expect(b.current).toEqual({ kind: "freehand", author: "user", points: [p(1, 2)] });
    expect(b.strokes).toEqual([]);
  });

  it("extends the current stroke", () => {
    const b = extendStroke(beginStroke(emptyBoard(), p(1, 2)), p(3, 4));
    expect(b.current).toEqual({ kind: "freehand", author: "user", points: [p(1, 2), p(3, 4)] });
  });

  it("ignores extend when no stroke is in progress", () => {
    const b = emptyBoard();
    expect(extendStroke(b, p(3, 4))).toBe(b);
  });

  it("commits the current stroke on end", () => {
    const b = endStroke(extendStroke(beginStroke(emptyBoard(), p(1, 2)), p(3, 4)));
    expect(b).toEqual({
      strokes: [{ kind: "freehand", author: "user", points: [p(1, 2), p(3, 4)] }],
      current: null,
    });
  });

  it("ignores end when no stroke is in progress", () => {
    const b = emptyBoard();
    expect(endStroke(b)).toBe(b);
  });

  it("does not mutate the previous board", () => {
    const before = beginStroke(emptyBoard(), p(1, 2));
    extendStroke(before, p(3, 4));
    endStroke(before);
    expect(before).toEqual({
      strokes: [],
      current: { kind: "freehand", author: "user", points: [p(1, 2)] },
    });
  });

  it("drops the in-progress stroke when a new one begins", () => {
    const b = beginStroke(beginStroke(emptyBoard(), p(1, 2)), p(5, 6));
    expect(b).toEqual({
      strokes: [],
      current: { kind: "freehand", author: "user", points: [p(5, 6)] },
    });
  });
});

describe("clearBoard", () => {
  it("removes all strokes and the stroke in progress", () => {
    const b = beginStroke(endStroke(beginStroke(emptyBoard(), p(0, 0))), p(1, 1));
    expect(clearBoard(b)).toEqual(emptyBoard());
  });
});

describe("eraseAt", () => {
  const line = (...pts: [number, number][]) => ({
    kind: "freehand" as const,
    author: "user" as const,
    points: pts.map(([x, y]) => p(x, y)),
  });
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

describe("box", () => {
  it("begins a zero-size box at the given point", () => {
    expect(beginBox(emptyBoard(), p(1, 2)).current).toEqual({
      kind: "box",
      author: "user",
      from: p(1, 2),
      to: p(1, 2),
    });
  });

  it("moves the opposite corner on extend", () => {
    const b = extendStroke(extendStroke(beginBox(emptyBoard(), p(1, 2)), p(5, 5)), p(9, 8));
    expect(b.current).toEqual({ kind: "box", author: "user", from: p(1, 2), to: p(9, 8) });
  });

  it("commits like any other stroke", () => {
    const b = endStroke(extendStroke(beginBox(emptyBoard(), p(0, 0)), p(10, 10)));
    expect(b).toEqual({
      strokes: [{ kind: "box", author: "user", from: p(0, 0), to: p(10, 10) }],
      current: null,
    });
  });

  it("is erased by touching an edge", () => {
    const b = endStroke(extendStroke(beginBox(emptyBoard(), p(0, 0)), p(100, 100)));
    expect(eraseAt(b, p(100, 50), 5).strokes).toEqual([]);
  });

  it("is not erased from its empty interior", () => {
    const b = endStroke(extendStroke(beginBox(emptyBoard(), p(0, 0)), p(100, 100)));
    expect(eraseAt(b, p(50, 50), 5)).toBe(b);
  });
});

describe("outline", () => {
  it("is the points of a freehand stroke", () => {
    expect(outline({ kind: "freehand", author: "user", points: [p(0, 0), p(1, 1)] })).toEqual([
      p(0, 0),
      p(1, 1),
    ]);
  });

  it("is the closed rectangle of a box", () => {
    expect(outline({ kind: "box", author: "user", from: p(0, 0), to: p(4, 2) })).toEqual([
      p(0, 0),
      p(4, 0),
      p(4, 2),
      p(0, 2),
      p(0, 0),
    ]);
  });
});

describe("toSnapshot", () => {
  it("keeps finished strokes only and records the scale and contract version", () => {
    const finished = endStroke(beginStroke(emptyBoard(), p(1, 1)));
    const b = beginStroke(finished, p(9, 9));
    expect(toSnapshot(b, 1.25, 4)).toEqual({
      contractVersion: CONTRACT_VERSION,
      scale: 1.25,
      submission: 4,
      strokes: finished.strokes,
    });
  });

  it("produces a document the contract accepts", () => {
    const b = endStroke(beginBox(emptyBoard(), p(0, 0)));
    expect(BoardSnapshot.safeParse(toSnapshot(b, 2, 0)).success).toBe(true);
  });
});

describe("userStrokesChanged", () => {
  const user = endStroke(beginStroke(emptyBoard(), p(0, 0)));
  const agentArrow = { kind: "arrow", author: "agent", from: p(0, 0), to: p(5, 5) } as const;

  it("is false for the same strokes", () => {
    expect(userStrokesChanged(user.strokes, user.strokes)).toBe(false);
  });

  it("ignores strokes the agent added or removed", () => {
    const withAgent = addStroke(user, agentArrow);
    expect(userStrokesChanged(user.strokes, withAgent.strokes)).toBe(false);
    expect(userStrokesChanged(withAgent.strokes, clearAuthor(withAgent, "agent").strokes)).toBe(
      false,
    );
  });

  it("is true when the user draws, erases or clears", () => {
    const more = endStroke(beginBox(user, p(9, 9)));
    expect(userStrokesChanged(user.strokes, more.strokes)).toBe(true);
    expect(userStrokesChanged(more.strokes, user.strokes)).toBe(true);
    expect(userStrokesChanged(user.strokes, clearBoard(user).strokes)).toBe(true);
  });
});

describe("authors", () => {
  it("attributes strokes to the user by default", () => {
    expect(beginStroke(emptyBoard(), p(0, 0)).current?.author).toBe("user");
    expect(beginBox(emptyBoard(), p(0, 0)).current?.author).toBe("user");
    expect(beginArrow(emptyBoard(), p(0, 0)).current?.author).toBe("user");
  });
});

describe("arrow", () => {
  it("begins at a point and moves its head on extend", () => {
    const b = extendStroke(extendStroke(beginArrow(emptyBoard(), p(1, 2)), p(5, 5)), p(9, 8));
    expect(b.current).toEqual({ kind: "arrow", author: "user", from: p(1, 2), to: p(9, 8) });
  });

  it("outlines as its shaft and can be erased by touching it", () => {
    const arrow = { kind: "arrow" as const, author: "user" as const, from: p(0, 0), to: p(100, 0) };
    expect(outline(arrow)).toEqual([p(0, 0), p(100, 0)]);
    expect(eraseAt({ strokes: [arrow], current: null }, p(50, 3), 5).strokes).toEqual([]);
  });
});

describe("text", () => {
  const text = (s: string) => ({
    kind: "text" as const,
    author: "agent" as const,
    at: p(100, 100),
    text: s,
  });

  it("outlines as a rectangle that grows with the text", () => {
    const short = textBounds(text("ab"));
    const long = textBounds(text("abcdef"));
    expect(short.x).toBe(100);
    expect(short.y).toBe(100);
    expect(short.height).toBe(TEXT_SIZE * TEXT_LINE_HEIGHT);
    expect(long.width).toBeGreaterThan(short.width);
  });

  it("treats wide (CJK) characters as wider than ASCII", () => {
    expect(textBounds(text("見えてる")).width).toBeGreaterThan(textBounds(text("abcd")).width);
  });

  it("is erased by touching anywhere on it, and not from afar", () => {
    const board = { strokes: [text("hello")], current: null };
    expect(eraseAt(board, p(110, 110), 2).strokes).toEqual([]);
    expect(eraseAt(board, p(500, 500), 2)).toBe(board);
  });

  it("is not changed by extend", () => {
    const b = { strokes: [], current: text("hi") };
    expect(extendStroke(b, p(0, 0))).toBe(b);
  });
});

describe("agent strokes", () => {
  const agentBox = {
    kind: "box" as const,
    author: "agent" as const,
    from: p(0, 0),
    to: p(10, 10),
  };

  it("adds a finished stroke without touching the one in progress", () => {
    const drawing = beginStroke(emptyBoard(), p(1, 1));
    const b = addStroke(drawing, agentBox);
    expect(b.strokes).toEqual([agentBox]);
    expect(b.current).toBe(drawing.current);
  });

  it("clears only one author's strokes", () => {
    const user = endStroke(beginStroke(emptyBoard(), p(5, 5)));
    const b = addStroke(user, agentBox);
    expect(clearAuthor(b, "agent").strokes).toEqual(user.strokes);
    expect(clearAuthor(b, "user").strokes).toEqual([agentBox]);
  });

  it("returns the same board when the author has no strokes", () => {
    const b = endStroke(beginStroke(emptyBoard(), p(5, 5)));
    expect(clearAuthor(b, "agent")).toBe(b);
  });
});
