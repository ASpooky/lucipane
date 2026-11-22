// Pen events sent by the Rust global mouse hook (see src-tauri/src/pen.rs, `PenEvent`).
// Coordinates are CSS pixels relative to the overlay window.

import { type Board, beginStroke, endStroke, extendStroke } from "./board.ts";

export type PenEvent =
  | { readonly kind: "down"; readonly x: number; readonly y: number }
  | { readonly kind: "move"; readonly x: number; readonly y: number }
  | { readonly kind: "up" };

export function applyPenEvent(board: Board, ev: PenEvent): Board {
  switch (ev.kind) {
    case "down":
      return beginStroke(board, { x: ev.x, y: ev.y });
    case "move":
      return extendStroke(board, { x: ev.x, y: ev.y });
    case "up":
      return endStroke(board);
  }
}
