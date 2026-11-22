// Board state and its pure transitions. No DOM, no canvas: rendering lives in render.ts.

export type Point = { readonly x: number; readonly y: number };
export type Stroke = { readonly points: readonly Point[] };
export type Board = {
  readonly strokes: readonly Stroke[];
  readonly current: Stroke | null;
};

export function emptyBoard(): Board {
  return { strokes: [], current: null };
}

/** Start a new stroke. An unfinished stroke in progress is discarded. */
export function beginStroke(board: Board, p: Point): Board {
  return { ...board, current: { points: [p] } };
}

export function extendStroke(board: Board, p: Point): Board {
  if (!board.current) return board;
  return { ...board, current: { points: [...board.current.points, p] } };
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
  const [first, ...rest] = stroke.points;
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
