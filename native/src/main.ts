// Overlay canvas.
// - Middle-button drag anywhere draws via the Rust global mouse hook (a Channel).
// - Draw mode (toggled by a global hotkey in Rust) makes the overlay take the mouse:
//   left-drag draws, boxes or erases. Single keys: B box, E eraser, C clear, Esc exit.
// - Finished strokes are saved to the board file for the `lucipane capture` CLI, and
//   `lucipane clear` arrives as a "clear" event.
// - In a plain browser (http://localhost:1420) draw mode is always on, for quick testing.

import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  type Board,
  beginBox,
  beginStroke,
  clearBoard,
  emptyBoard,
  endStroke,
  eraseAt,
  extendStroke,
  type Point,
  toSnapshot,
} from "./board.ts";
import { applyPenEvent, type PenEvent } from "./pen.ts";
import { type Pen, render } from "./render.ts";

type Tool = "pen" | "box" | "eraser";

const TOOL_LABELS: Record<Tool, string> = { pen: "Pen", box: "Box", eraser: "Eraser" };

const ERASER_RADIUS = 12;

function setupCanvas(): { canvas: HTMLCanvasElement; ctx: CanvasRenderingContext2D } {
  const canvas = document.querySelector<HTMLCanvasElement>("#board");
  if (!canvas) throw new Error("#board canvas not found");
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("2d context unavailable");
  return { canvas, ctx };
}

const { canvas, ctx } = setupCanvas();
const hudTool = document.querySelector<HTMLElement>("#hud-tool");

const pen: Pen = { color: "rgba(255, 60, 60, 0.8)", width: 6 };

// The single place board state lives. Only replace it through board.ts functions.
let board: Board = emptyBoard();
// UI state.
let drawMode = !isTauri();
let tool: Tool = "pen";
let erasing = false;

function update(next: Board): void {
  if (next === board) return;
  const finishedChanged = next.strokes !== board.strokes;
  board = next;
  render(ctx, board, pen);
  if (finishedChanged) saveBoard();
}

/** Keep the board file current for `lucipane capture`. Only finished strokes count. */
function saveBoard(): void {
  if (!isTauri()) return;
  const snapshot = toSnapshot(board, window.devicePixelRatio);
  invoke("save_board", { snapshot }).catch((e: unknown) => {
    console.error("failed to save the board", e);
  });
}

function setDrawMode(enabled: boolean): void {
  drawMode = enabled;
  erasing = false;
  if (!enabled) update(endStroke(board));
  document.body.classList.toggle("draw-mode", enabled);
}

function setTool(next: Tool): void {
  tool = next;
  document.body.classList.toggle("tool-eraser", next === "eraser");
  if (hudTool) hudTool.textContent = TOOL_LABELS[next];
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

async function listenToRust(): Promise<void> {
  const channel = new Channel<PenEvent>();
  channel.onmessage = (ev) => update(applyPenEvent(board, ev));
  await invoke("subscribe_pen", { onEvent: channel });
  await listen<boolean>("draw-mode", (e) => setDrawMode(e.payload));
  // Sent when `lucipane clear` is run while the overlay is up.
  await listen("clear", () => update(clearBoard(board)));
}

function exitDrawMode(): void {
  if (isTauri()) void invoke("set_draw_mode", { enabled: false });
}

function listenToPointer(): void {
  const toPoint = (e: PointerEvent): Point => ({ x: e.clientX, y: e.clientY });
  const LEFT = 0;
  const LEFT_HELD = 1;

  canvas.addEventListener("pointerdown", (e) => {
    // Only the left button; the middle button is handled by the global hook.
    if (!drawMode || e.button !== LEFT) return;
    canvas.setPointerCapture(e.pointerId);
    if (tool === "eraser") {
      erasing = true;
      update(eraseAt(board, toPoint(e), ERASER_RADIUS));
    } else {
      const begin = tool === "box" ? beginBox : beginStroke;
      update(begin(board, toPoint(e)));
    }
  });
  canvas.addEventListener("pointermove", (e) => {
    if (!drawMode || !(e.buttons & LEFT_HELD)) return;
    if (erasing) update(eraseAt(board, toPoint(e), ERASER_RADIUS));
    else update(extendStroke(board, toPoint(e)));
  });
  const finish = (e: PointerEvent) => {
    if (e.button !== LEFT) return;
    if (erasing) erasing = false;
    else update(endStroke(board));
  };
  canvas.addEventListener("pointerup", finish);
  canvas.addEventListener("pointercancel", finish);
}

function listenToKeys(): void {
  window.addEventListener("keydown", (e) => {
    if (!drawMode || e.repeat || e.ctrlKey || e.altKey || e.metaKey) return;
    switch (e.key.toLowerCase()) {
      case "e":
        setTool(tool === "eraser" ? "pen" : "eraser");
        break;
      case "b":
        setTool(tool === "box" ? "pen" : "box");
        break;
      case "c":
        update(clearBoard(board));
        break;
      case "escape":
        exitDrawMode();
        break;
    }
  });
}

window.addEventListener("resize", resize);
resize();
setDrawMode(drawMode);
listenToPointer();
listenToKeys();
if (isTauri()) void listenToRust();
