/**
 * Events the Rust shell sends to the overlay's WebView when a `lucipane` command is
 * forwarded to the running overlay (see `on_second_instance` in native/src-tauri/src/lib.rs).
 *
 *   "clear"       ClearEvent
 *   "agent-draw"  Stroke (board.ts), author "agent", in CSS pixels
 *
 * and when the wheel scrolls another app (see scroll.rs):
 *
 *   "scroll"      ScrollEvent
 */
import { z } from "zod";

export const ClearEvent = z.object({
  /** Erase only the agent's strokes, keeping the user's. */
  agentOnly: z.boolean(),
  /** Clear the panes of every virtual desktop, not just the current one. */
  allDesktops: z.boolean(),
});
export type ClearEvent = z.infer<typeof ClearEvent>;

/** An area of the overlay in CSS pixels, relative to its top-left corner. */
export const Rect = z.object({
  x: z.number(),
  y: z.number(),
  width: z.number().nonnegative(),
  height: z.number().nonnegative(),
});
export type Rect = z.infer<typeof Rect>;

/** The wheel turned over another app's window, whose content moved by `dy`. */
export const ScrollEvent = z.object({
  /** The window, as an opaque id that stays the same while the window lives. */
  window: z.number().int().nonnegative(),
  /** The window's area, where marks that follow its scrolling are drawn. */
  rect: Rect,
  /** How far the content moved down, in CSS pixels; negative when it moved up. */
  dy: z.number(),
});
export type ScrollEvent = z.infer<typeof ScrollEvent>;
