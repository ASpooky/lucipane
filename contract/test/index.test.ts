import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  Annotation,
  Board,
  BoardSnapshot,
  CaptureOutput,
  ClearEvent,
  CONTRACT_VERSION,
  Stroke,
} from "../src/index.ts";

// The fixtures are shared with the Rust tests in native/src-tauri, so both sides are
// checked against the same documents.
const fixture = (name: string): unknown =>
  JSON.parse(readFileSync(new URL(`../fixtures/${name}`, import.meta.url), "utf8"));

const box = { kind: "box", author: "user", from: { x: 0, y: 0 }, to: { x: 10, y: 5 } };

describe("contract", () => {
  it("exposes a contract version", () => {
    expect(CONTRACT_VERSION).toBe(3);
  });
});

describe("Board", () => {
  it("accepts every stroke kind", () => {
    const board = {
      strokes: [
        { kind: "freehand", author: "user", points: [{ x: 1, y: 2 }] },
        box,
        { kind: "arrow", author: "agent", from: { x: 0, y: 0 }, to: { x: 5, y: 5 } },
        { kind: "text", author: "agent", at: { x: 3, y: 4 }, text: "here" },
      ],
      current: null,
    };
    expect(Board.parse(board)).toEqual(board);
  });

  it("rejects an unknown stroke kind", () => {
    expect(Board.safeParse({ strokes: [{ kind: "circle" }], current: null }).success).toBe(false);
  });

  it("requires an author", () => {
    expect(
      Stroke.safeParse({ kind: "box", from: { x: 0, y: 0 }, to: { x: 1, y: 1 } }).success,
    ).toBe(false);
    expect(Stroke.safeParse({ ...box, author: "someone" }).success).toBe(false);
  });

  it("rejects empty text", () => {
    const text = { kind: "text", author: "agent", at: { x: 0, y: 0 }, text: "" };
    expect(Stroke.safeParse(text).success).toBe(false);
  });
});

describe("BoardSnapshot (overlay app -> board file -> CLI)", () => {
  it("accepts the shared fixture", () => {
    expect(BoardSnapshot.safeParse(fixture("board-snapshot.json")).success).toBe(true);
  });

  it("rejects another contract version", () => {
    const snapshot = {
      contractVersion: CONTRACT_VERSION + 1,
      scale: 1,
      submission: 0,
      strokes: [],
    };
    expect(BoardSnapshot.safeParse(snapshot).success).toBe(false);
  });

  it("rejects a non-positive scale", () => {
    const snapshot = { contractVersion: CONTRACT_VERSION, scale: 0, submission: 0, strokes: [] };
    expect(BoardSnapshot.safeParse(snapshot).success).toBe(false);
  });

  it("requires a non-negative integer submission count", () => {
    const base = { contractVersion: CONTRACT_VERSION, scale: 1, strokes: [] };
    expect(BoardSnapshot.safeParse(base).success).toBe(false);
    expect(BoardSnapshot.safeParse({ ...base, submission: -1 }).success).toBe(false);
    expect(BoardSnapshot.safeParse({ ...base, submission: 1.5 }).success).toBe(false);
    expect(BoardSnapshot.safeParse({ ...base, submission: 7 }).success).toBe(true);
  });
});

describe("CaptureOutput (CLI stdout -> agent)", () => {
  it("accepts the shared fixture", () => {
    expect(CaptureOutput.safeParse(fixture("capture-output.json")).success).toBe(true);
  });

  it("rejects a box with negative size", () => {
    const negative = { kind: "box", author: "user", x: 0, y: 0, width: -1, height: 5 };
    expect(Annotation.safeParse(negative).success).toBe(false);
  });

  it("rejects fractional pixel coordinates", () => {
    const fractional = { kind: "freehand", author: "user", points: [[0.5, 1]] };
    expect(Annotation.safeParse(fractional).success).toBe(false);
  });

  it("carries the author of each annotation", () => {
    const arrow = { kind: "arrow", author: "agent", from: [0, 0], to: [10, 10] };
    expect(Annotation.parse(arrow)).toEqual(arrow);
  });
});

describe("ClearEvent (Rust -> overlay WebView)", () => {
  it("says whether to clear only the agent's strokes", () => {
    const event = { agentOnly: true, allDesktops: false };
    expect(ClearEvent.parse(event)).toEqual(event);
    expect(ClearEvent.safeParse({ agentOnly: true }).success).toBe(false);
    expect(ClearEvent.safeParse({}).success).toBe(false);
  });
});
