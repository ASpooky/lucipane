import { describe, expect, it } from "vitest";
import { beginStroke, emptyBoard, endStroke, extendStroke } from "../src/board.ts";
import { emptyPane, type Pane, switchPane } from "../src/workspaces.ts";

const p = (x: number, y: number) => ({ x, y });

function paneWithStroke(): Pane {
  const board = endStroke(extendStroke(beginStroke(emptyBoard(), p(0, 0)), p(5, 5)));
  return { board, submitted: board.strokes };
}

describe("workspaces", () => {
  it("gives a workspace seen for the first time an empty pane", () => {
    const { pane } = switchPane(new Map(), "a", paneWithStroke(), "b");
    expect(pane).toEqual(emptyPane());
  });

  it("brings back what was drawn when returning", () => {
    const drawn = paneWithStroke();
    const away = switchPane(new Map(), "a", drawn, "b");
    const back = switchPane(away.parked, "b", away.pane, "a");
    expect(back.pane).toEqual(drawn);
    expect(back.parked.has("a")).toBe(false);
    expect(back.parked.get("b")).toEqual(emptyPane());
  });

  it("finishes a stroke in progress before parking it", () => {
    const board = extendStroke(beginStroke(emptyBoard(), p(0, 0)), p(5, 5));
    const leaving: Pane = { board, submitted: [] };
    const away = switchPane(new Map(), "a", leaving, "b");
    const parked = away.parked.get("a");
    expect(parked?.board.current).toBeNull();
    expect(parked?.board.strokes).toHaveLength(1);
  });

  it("does not modify the map it is given", () => {
    const parked = new Map<string, Pane>();
    switchPane(parked, "a", paneWithStroke(), "b");
    expect(parked.size).toBe(0);
  });
});
