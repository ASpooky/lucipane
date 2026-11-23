/**
 * The board file: the overlay app writes it whenever the finished strokes change, and the
 * `lucipane capture` and `lucipane wait` CLI commands read it.
 * Lives at `<data dir>/dev.aspooky.lucipane/board.json`.
 */
import { z } from "zod";
import { Stroke } from "./board.ts";
import { CONTRACT_VERSION } from "./version.ts";

export const BoardSnapshot = z.object({
  contractVersion: z.literal(CONTRACT_VERSION),
  /** Physical (screenshot) pixels per CSS pixel on the overlay's display. */
  scale: z.number().positive(),
  /**
   * How many times the user has handed their drawing over: the overlay counts up each time
   * draw mode ends with the user's strokes changed. `lucipane wait` returns when this differs
   * from the value it started with. Starts at 0 with every overlay launch.
   */
  submission: z.number().int().nonnegative(),
  /** Finished strokes in CSS pixels; a stroke still being drawn is not included. */
  strokes: z.array(Stroke).readonly(),
});
export type BoardSnapshot = z.infer<typeof BoardSnapshot>;
