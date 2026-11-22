// Spike ②: draw strokes on the overlay canvas.
// In Tauri, input comes from the Rust global mouse hook (middle-button drag) via a Channel.
// In a plain browser (http://localhost:1420), pointer events stand in so the page can be tried
// without the overlay, which is click-through.

import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import {
  type Board,
  beginStroke,
  emptyBoard,
  endStroke,
  extendStroke,
  type Point,
} from "./board.ts";
import { applyPenEvent, type PenEvent } from "./pen.ts";
import { type Pen, render } from "./render.ts";

function setupCanvas(): { canvas: HTMLCanvasElement; ctx: CanvasRenderingContext2D } {
  const canvas = document.querySelector<HTMLCanvasElement>("#board");
  if (!canvas) throw new Error("#board canvas not found");
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("2d context unavailable");
  return { canvas, ctx };
}

const { canvas, ctx } = setupCanvas();

const pen: Pen = { color: "rgba(255, 60, 60, 0.8)", width: 6 };

// The single place board state lives. Only replace it through board.ts functions.
let board: Board = emptyBoard();

function update(next: Board): void {
  board = next;
  render(ctx, board, pen);
}

function resize(): void {
  // Back the canvas with physical pixels so lines stay sharp on high-DPI displays.
  // Resizing clears the canvas, so redraw afterwards.
  const dpr = window.devicePixelRatio;
  canvas.width = Math.round(window.innerWidth * dpr);
  canvas.height = Math.round(window.innerHeight * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  render(ctx, board, pen);
}

async function listenToMouseHook(): Promise<void> {
  const channel = new Channel<PenEvent>();
  channel.onmessage = (ev) => update(applyPenEvent(board, ev));
  await invoke("subscribe_pen", { onEvent: channel });
}

function listenToPointer(): void {
  const toPoint = (e: PointerEvent): Point => ({ x: e.clientX, y: e.clientY });
  canvas.addEventListener("pointerdown", (e) => {
    canvas.setPointerCapture(e.pointerId);
    update(beginStroke(board, toPoint(e)));
  });
  canvas.addEventListener("pointermove", (e) => update(extendStroke(board, toPoint(e))));
  canvas.addEventListener("pointerup", () => update(endStroke(board)));
  canvas.addEventListener("pointercancel", () => update(endStroke(board)));
}

window.addEventListener("resize", resize);
resize();

if (isTauri()) {
  void listenToMouseHook();
} else {
  listenToPointer();
}
