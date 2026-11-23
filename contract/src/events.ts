/**
 * Events the Rust shell sends to the overlay's WebView when a `lucipane` command is
 * forwarded to the running overlay (see `on_second_instance` in native/src-tauri/src/lib.rs).
 *
 *   "clear"       ClearEvent
 *   "agent-draw"  Stroke (board.ts), author "agent", in CSS pixels
 */
import { z } from "zod";

export const ClearEvent = z.object({
  /** Erase only the agent's strokes, keeping the user's. */
  agentOnly: z.boolean(),
});
export type ClearEvent = z.infer<typeof ClearEvent>;
