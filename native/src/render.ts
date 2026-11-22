// Draws a Board onto a 2D canvas context. Holds no state.

import {
  type Author,
  type Board,
  outline,
  type Point,
  type Stroke,
  TEXT_LINE_HEIGHT,
  TEXT_SIZE,
  textBounds,
} from "./board.ts";

export type Pen = { readonly color: string; readonly width: number };

/** One pen per author, so the user can tell their marks from the agent's. */
export type Palette = Readonly<Record<Author, Pen>>;

export function drawStroke(ctx: CanvasRenderingContext2D, stroke: Stroke, pen: Pen): void {
  ctx.strokeStyle = pen.color;
  ctx.fillStyle = pen.color;
  ctx.lineWidth = pen.width;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  switch (stroke.kind) {
    case "text":
      drawText(ctx, stroke);
      return;
    case "arrow":
      drawPolyline(ctx, outline(stroke));
      drawArrowHead(ctx, stroke.from, stroke.to, pen);
      return;
    default:
      drawPolyline(ctx, outline(stroke));
  }
}

function drawPolyline(ctx: CanvasRenderingContext2D, points: readonly Point[]): void {
  const [first, ...rest] = points;
  if (!first) return;
  ctx.beginPath();
  ctx.moveTo(first.x, first.y);
  // A single point still draws a dot thanks to the round cap.
  if (rest.length === 0) ctx.lineTo(first.x, first.y);
  for (const p of rest) ctx.lineTo(p.x, p.y);
  ctx.stroke();
}

function drawArrowHead(ctx: CanvasRenderingContext2D, from: Point, to: Point, pen: Pen): void {
  const angle = Math.atan2(to.y - from.y, to.x - from.x);
  if (from.x === to.x && from.y === to.y) return;
  const length = Math.max(14, pen.width * 3.5);
  const spread = Math.PI / 7;
  ctx.beginPath();
  for (const side of [-1, 1]) {
    ctx.moveTo(to.x, to.y);
    ctx.lineTo(
      to.x - length * Math.cos(angle + side * spread),
      to.y - length * Math.sin(angle + side * spread),
    );
  }
  ctx.stroke();
}

function drawText(ctx: CanvasRenderingContext2D, text: Extract<Stroke, { kind: "text" }>): void {
  const { x, y, width, height } = textBounds(text);
  const color = ctx.fillStyle;
  // A dark backing keeps the label readable on any background.
  ctx.fillStyle = "rgba(15, 18, 26, 0.8)";
  ctx.fillRect(x - 4, y - 2, width + 8, height + 4);
  ctx.fillStyle = color;
  ctx.font = `600 ${TEXT_SIZE}px system-ui, sans-serif`;
  ctx.textBaseline = "top";
  ctx.fillText(text.text, x, y + (TEXT_SIZE * (TEXT_LINE_HEIGHT - 1)) / 2);
}

/** Clear and redraw everything. Fine for a spike; incremental drawing can come later. */
export function render(ctx: CanvasRenderingContext2D, board: Board, palette: Palette): void {
  ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
  for (const s of board.strokes) drawStroke(ctx, s, palette[s.author]);
  if (board.current) drawStroke(ctx, board.current, palette[board.current.author]);
}
