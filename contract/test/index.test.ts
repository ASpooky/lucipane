import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { Annotation, Board, BoardSnapshot, CaptureOutput, CONTRACT_VERSION } from "../src/index.ts";

// The fixtures are shared with the Rust tests in native/src-tauri, so both sides are
// checked against the same documents.
const fixture = (name: string): unknown =>
  JSON.parse(readFileSync(new URL(`../fixtures/${name}`, import.meta.url), "utf8"));

const box = { kind: "box", from: { x: 0, y: 0 }, to: { x: 10, y: 5 } };

describe("contract", () => {
  it("exposes a contract version", () => {
    expect(CONTRACT_VERSION).toBe(1);
  });
});

describe("Board", () => {
  it("accepts freehand strokes and boxes", () => {
    const board = { strokes: [{ kind: "freehand", points: [{ x: 1, y: 2 }] }, box], current: null };
    expect(Board.parse(board)).toEqual(board);
  });

  it("rejects an unknown stroke kind", () => {
    expect(Board.safeParse({ strokes: [{ kind: "circle" }], current: null }).success).toBe(false);
  });
});

describe("BoardSnapshot (overlay app -> board file -> CLI)", () => {
  it("accepts the shared fixture", () => {
    expect(BoardSnapshot.safeParse(fixture("board-snapshot.json")).success).toBe(true);
  });

  it("rejects another contract version", () => {
    const snapshot = { contractVersion: CONTRACT_VERSION + 1, scale: 1, strokes: [] };
    expect(BoardSnapshot.safeParse(snapshot).success).toBe(false);
  });

  it("rejects a non-positive scale", () => {
    const snapshot = { contractVersion: CONTRACT_VERSION, scale: 0, strokes: [] };
    expect(BoardSnapshot.safeParse(snapshot).success).toBe(false);
  });
});

describe("CaptureOutput (CLI stdout -> agent)", () => {
  it("accepts the shared fixture", () => {
    expect(CaptureOutput.safeParse(fixture("capture-output.json")).success).toBe(true);
  });

  it("rejects a box with negative size", () => {
    expect(Annotation.safeParse({ kind: "box", x: 0, y: 0, width: -1, height: 5 }).success).toBe(
      false,
    );
  });

  it("rejects fractional pixel coordinates", () => {
    expect(Annotation.safeParse({ kind: "freehand", points: [[0.5, 1]] }).success).toBe(false);
  });
});
