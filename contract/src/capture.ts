/**
 * What `lucipane capture` prints to stdout as one JSON document.
 * An agent reads the image at `image.path` and relates `annotations` to it.
 */
import { z } from "zod";
import { CONTRACT_VERSION } from "./version.ts";

const Pixel = z.number().int();
const Size = z.number().int().nonnegative();

/**
 * One thing the user drew, in pixels of the captured image (origin top-left).
 * Freehand points are rounded, with consecutive duplicates removed.
 */
export const Annotation = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("freehand"), points: z.array(z.tuple([Pixel, Pixel])) }),
  z.object({ kind: z.literal("box"), x: Pixel, y: Pixel, width: Size, height: Size }),
]);
export type Annotation = z.infer<typeof Annotation>;

export const CaptureOutput = z.object({
  contractVersion: z.literal(CONTRACT_VERSION),
  /** A PNG of the primary display, overlay strokes included. */
  image: z.object({
    path: z.string().min(1),
    width: z.number().int().positive(),
    height: z.number().int().positive(),
  }),
  annotations: z.array(Annotation),
});
export type CaptureOutput = z.infer<typeof CaptureOutput>;
