// One pane per virtual desktop (macOS Space, GlazeWM workspace). Pure: main.ts owns the state.

import { type Board, emptyBoard, endStroke, type Stroke } from "./board.ts";

/** What a workspace keeps while the user is elsewhere. */
export type Pane = {
  readonly board: Board;
  /** The user's strokes at the last handover, so returning is not a new handover. */
  readonly submitted: readonly Stroke[];
};

export function emptyPane(): Pane {
  const board = emptyBoard();
  return { board, submitted: board.strokes };
}

/**
 * Park the pane being left (finishing any stroke in progress) and take out the one for `to`,
 * or a fresh one for a workspace not seen before.
 */
export function switchPane(
  parked: ReadonlyMap<string, Pane>,
  from: string,
  leaving: Pane,
  to: string,
): { parked: Map<string, Pane>; pane: Pane } {
  const next = new Map(parked);
  next.set(from, { ...leaving, board: endStroke(leaving.board) });
  const pane = next.get(to) ?? emptyPane();
  next.delete(to);
  return { parked: next, pane };
}

/** Apply `clear` to the board of every parked pane (`lucipane clear --all`). */
export function clearPanes(
  parked: ReadonlyMap<string, Pane>,
  clear: (board: Board) => Board,
): Map<string, Pane> {
  return new Map([...parked].map(([id, pane]) => [id, { ...pane, board: clear(pane.board) }]));
}
