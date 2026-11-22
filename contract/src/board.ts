/**
 * The board: what the user has drawn on the overlay.
 * Coordinates are CSS pixels relative to the overlay window (top-left origin).
 */
import { z } from "zod";

export const Point = z.object({ x: z.number(), y: z.number() });
export type Point = z.infer<typeof Point>;

/** A hand-drawn line through `points`. */
export const Freehand = z.object({
  kind: z.literal("freehand"),
  points: z.array(Point).readonly(),
});
export type Freehand = z.infer<typeof Freehand>;

/**
 * An axis-aligned rectangle between two opposite corners (in drawing order, not normalized).
 * Kept as a box rather than flattened to points so consumers can tell an area was boxed.
 */
export const Box = z.object({ kind: z.literal("box"), from: Point, to: Point });
export type Box = z.infer<typeof Box>;

export const Stroke = z.discriminatedUnion("kind", [Freehand, Box]);
export type Stroke = z.infer<typeof Stroke>;

export const Board = z.object({
  strokes: z.array(Stroke).readonly(),
  /** The stroke being drawn right now, if any. */
  current: Stroke.nullable(),
});
export type Board = z.infer<typeof Board>;
