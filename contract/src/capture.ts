/**
 * What `lucipane capture` prints to stdout as one JSON document.
 * An agent reads the image at `image.path` and relates `annotations` to it.
 */
import { z } from "zod";
import { Author } from "./board.ts";
import { CONTRACT_VERSION } from "./version.ts";

const Pixel = z.number().int();
const Size = z.number().int().nonnegative();
const PixelPoint = z.tuple([Pixel, Pixel]);

/**
 * One thing drawn on the overlay, in pixels of the captured image (origin top-left).
 * Freehand points are rounded and simplified. `author` tells the user's marks from the
 * agent's own.
 */
export const Annotation = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("freehand"), author: Author, points: z.array(PixelPoint) }),
  z.object({
    kind: z.literal("box"),
    author: Author,
    x: Pixel,
    y: Pixel,
    width: Size,
    height: Size,
  }),
  z.object({ kind: z.literal("arrow"), author: Author, from: PixelPoint, to: PixelPoint }),
  z.object({ kind: z.literal("text"), author: Author, x: Pixel, y: Pixel, text: z.string() }),
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
