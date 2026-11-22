/**
 * The board file: the overlay app writes it whenever the finished strokes change, and the
 * `lucipane capture` CLI reads it. Lives at `<data dir>/dev.aspooky.lucipane/board.json`.
 */
import { z } from "zod";
import { Stroke } from "./board.ts";
import { CONTRACT_VERSION } from "./version.ts";

export const BoardSnapshot = z.object({
  contractVersion: z.literal(CONTRACT_VERSION),
  /** Physical (screenshot) pixels per CSS pixel on the overlay's display. */
  scale: z.number().positive(),
  /** Finished strokes in CSS pixels; a stroke still being drawn is not included. */
  strokes: z.array(Stroke).readonly(),
});
export type BoardSnapshot = z.infer<typeof BoardSnapshot>;
