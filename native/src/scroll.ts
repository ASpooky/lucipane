// Marks follow the scrolling of the window they were drawn on. Pure: main.ts owns the state.
//
// A stroke gets anchored to a window the first time that window scrolls with the stroke's
// centre inside it. From then on it moves with every scroll of that window, and is drawn
// clipped to the window's area, so marks scrolled out of view disappear and come back when
// scrolled back.

import type { Rect, ScrollEvent } from "@lucipane/contract";
import { outline, type Point, type Stroke, textBounds } from "./board.ts";

export type { Rect };

/** The window a stroke scrolls with, and that window's area to clip it to. */
export type Anchor = { readonly window: number; readonly clip: Rect };
export type AnchorOf = (stroke: Stroke) => Anchor | undefined;

/** The smallest rectangle around a stroke's outline (a text label's box). */
export function bounds(stroke: Stroke): Rect {
  if (stroke.kind === "text") return textBounds(stroke);
  const points = outline(stroke);
  const xs = points.map((q) => q.x);
  const ys = points.map((q) => q.y);
  const x = Math.min(...xs);
  const y = Math.min(...ys);
  return { x, y, width: Math.max(...xs) - x, height: Math.max(...ys) - y };
}

export function intersects(a: Rect, b: Rect): boolean {
  return (
    a.x <= b.x + b.width && b.x <= a.x + a.width && a.y <= b.y + b.height && b.y <= a.y + a.height
  );
}

function contains(r: Rect, q: Point): boolean {
  return q.x >= r.x && q.x <= r.x + r.width && q.y >= r.y && q.y <= r.y + r.height;
}

function centre(r: Rect): Point {
  return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
}

export function translate(stroke: Stroke, dy: number): Stroke {
  const move = (q: Point): Point => ({ x: q.x, y: q.y + dy });
  switch (stroke.kind) {
    case "freehand":
      return { ...stroke, points: stroke.points.map(move) };
    case "box":
    case "arrow":
      return { ...stroke, from: move(stroke.from), to: move(stroke.to) };
    case "text":
      return { ...stroke, at: move(stroke.at) };
  }
}

/** Whether a stroke is on screen: anchored strokes only while they overlap their window. */
export function isVisible(stroke: Stroke, anchorOf: AnchorOf): boolean {
  const anchor = anchorOf(stroke);
  return !anchor || intersects(bounds(stroke), anchor.clip);
}

export type Scrolled = {
  /** The same array when nothing moved. */
  readonly strokes: readonly Stroke[];
  /** Anchors for the strokes that follow this window (new, moved or re-clipped). */
  readonly anchors: ReadonlyMap<Stroke, Anchor>;
  /** Old stroke -> its moved replacement. */
  readonly moved: ReadonlyMap<Stroke, Stroke>;
};

/** Move the strokes that follow the scrolled window by `event.dy`. */
export function scrollStrokes(
  strokes: readonly Stroke[],
  anchorOf: AnchorOf,
  event: ScrollEvent,
): Scrolled {
  const anchor: Anchor = { window: event.window, clip: event.rect };
  const anchors = new Map<Stroke, Anchor>();
  const moved = new Map<Stroke, Stroke>();
  const next = strokes.map((stroke) => {
    const current = anchorOf(stroke);
    const follows = current
      ? current.window === event.window
      : contains(event.rect, centre(bounds(stroke)));
    if (!follows) return stroke;
    const after = event.dy === 0 ? stroke : translate(stroke, event.dy);
    anchors.set(after, anchor);
    if (after !== stroke) moved.set(stroke, after);
    return after;
  });
  return { strokes: moved.size > 0 ? next : strokes, anchors, moved };
}
