// Board state transitions. Pure: no DOM, no canvas (rendering lives in render.ts).
// The types come from the contract, the same documents the CLI reads from the board file.

import {
  type Board,
  type BoardSnapshot,
  CONTRACT_VERSION,
  type Point,
  type Stroke,
} from "@lucipane/contract";

export type { Board, Point, Stroke };

export function emptyBoard(): Board {
  return { strokes: [], current: null };
}

/** Start a freehand stroke. An unfinished stroke in progress is discarded. */
export function beginStroke(board: Board, p: Point): Board {
  return { ...board, current: { kind: "freehand", points: [p] } };
}

/** Start a box with both corners at `p`. An unfinished stroke in progress is discarded. */
export function beginBox(board: Board, p: Point): Board {
  return { ...board, current: { kind: "box", from: p, to: p } };
}

/** Freehand: append `p`. Box: move the opposite corner to `p`. */
export function extendStroke(board: Board, p: Point): Board {
  const current = board.current;
  if (!current) return board;
  switch (current.kind) {
    case "freehand":
      return { ...board, current: { ...current, points: [...current.points, p] } };
    case "box":
      return { ...board, current: { ...current, to: p } };
  }
}

/** The polyline that draws a stroke. A box becomes its closed rectangle. */
export function outline(stroke: Stroke): readonly Point[] {
  switch (stroke.kind) {
    case "freehand":
      return stroke.points;
    case "box": {
      const { from: a, to: b } = stroke;
      return [a, { x: b.x, y: a.y }, b, { x: a.x, y: b.y }, a];
    }
  }
}

export function endStroke(board: Board): Board {
  if (!board.current) return board;
  return { strokes: [...board.strokes, board.current], current: null };
}

export function clearBoard(_board: Board): Board {
  return emptyBoard();
}

/**
 * Remove every finished stroke that passes within `radius` of `p`.
 * Whole strokes are erased (not pixels) so the board stays a plain list of strokes.
 * Returns the same board when nothing is hit.
 */
export function eraseAt(board: Board, p: Point, radius: number): Board {
  const kept = board.strokes.filter((s) => !strokeHits(s, p, radius));
  if (kept.length === board.strokes.length) return board;
  return { ...board, strokes: kept };
}

function strokeHits(stroke: Stroke, p: Point, radius: number): boolean {
  const [first, ...rest] = outline(stroke);
  if (!first) return false;
  if (rest.length === 0) return distance(p, first) <= radius;
  let prev = first;
  for (const next of rest) {
    if (distanceToSegment(p, prev, next) <= radius) return true;
    prev = next;
  }
  return false;
}

function distance(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/** Distance from `p` to the segment a–b (clamped to the segment, not the infinite line). */
function distanceToSegment(p: Point, a: Point, b: Point): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const lengthSq = dx * dx + dy * dy;
  if (lengthSq === 0) return distance(p, a);
  const t = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / lengthSq));
  return distance(p, { x: a.x + t * dx, y: a.y + t * dy });
}

/**
 * The document written to the board file for the `lucipane capture` CLI.
 * `scale` is physical pixels per CSS pixel (window.devicePixelRatio).
 */
export function toSnapshot(board: Board, scale: number): BoardSnapshot {
  return { contractVersion: CONTRACT_VERSION, scale, strokes: board.strokes };
}
