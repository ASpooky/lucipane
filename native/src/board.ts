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
