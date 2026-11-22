/**
 * The board: what the user and the agent have drawn on the overlay.
 * Coordinates are CSS pixels relative to the overlay window (top-left origin).
 */
import { z } from "zod";

export const Point = z.object({ x: z.number(), y: z.number() });
export type Point = z.infer<typeof Point>;

/** Who drew a stroke. The overlay draws them in different colors. */
export const Author = z.enum(["user", "agent"]);
export type Author = z.infer<typeof Author>;

/** A hand-drawn line through `points`. */
export const Freehand = z.object({
  kind: z.literal("freehand"),
  author: Author,
  points: z.array(Point).readonly(),
});
export type Freehand = z.infer<typeof Freehand>;

/**
 * An axis-aligned rectangle between two opposite corners (in drawing order, not normalized).
 * Kept as a box rather than flattened to points so consumers can tell an area was boxed.
 */
export const Box = z.object({ kind: z.literal("box"), author: Author, from: Point, to: Point });
export type Box = z.infer<typeof Box>;

/** A straight arrow pointing from `from` to `to` (the head is at `to`). */
export const Arrow = z.object({
  kind: z.literal("arrow"),
  author: Author,
  from: Point,
  to: Point,
});
export type Arrow = z.infer<typeof Arrow>;

/** A short text label with its top-left corner at `at`. */
export const Text = z.object({
  kind: z.literal("text"),
  author: Author,
  at: Point,
  text: z.string().min(1).max(500),
});
export type Text = z.infer<typeof Text>;

export const Stroke = z.discriminatedUnion("kind", [Freehand, Box, Arrow, Text]);
export type Stroke = z.infer<typeof Stroke>;

export const Board = z.object({
  strokes: z.array(Stroke).readonly(),
  /** The stroke being drawn right now, if any. */
  current: Stroke.nullable(),
});
export type Board = z.infer<typeof Board>;
