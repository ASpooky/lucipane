// Draws a Board onto a 2D canvas context. Holds no state.

import { type Board, outline, type Stroke } from "./board.ts";

export type Pen = { readonly color: string; readonly width: number };

export function drawStroke(ctx: CanvasRenderingContext2D, stroke: Stroke, pen: Pen): void {
  const [first, ...rest] = outline(stroke);
  if (!first) return;
  ctx.strokeStyle = pen.color;
  ctx.lineWidth = pen.width;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.beginPath();
  ctx.moveTo(first.x, first.y);
  // A single point still draws a dot thanks to the round cap.
  if (rest.length === 0) ctx.lineTo(first.x, first.y);
  for (const p of rest) ctx.lineTo(p.x, p.y);
  ctx.stroke();
}

/** Clear and redraw everything. Fine for a spike; incremental drawing can come later. */
export function render(ctx: CanvasRenderingContext2D, board: Board, pen: Pen): void {
  ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
  for (const s of board.strokes) drawStroke(ctx, s, pen);
  if (board.current) drawStroke(ctx, board.current, pen);
}
