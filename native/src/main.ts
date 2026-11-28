// Overlay canvas.
// - Middle-button drag anywhere draws via the Rust global mouse hook (a Channel).
// - Draw mode (toggled by a global hotkey in Rust) makes the overlay take the mouse:
//   left-drag draws. Single keys: A arrow, B box, E eraser, C clear, Esc exit.
// - Finished strokes are saved to the board file for the `lucipane capture` CLI, and
//   `lucipane clear` and `lucipane draw` (agents drawing in blue) arrive as events.
// - Leaving draw mode hands the drawing over to an agent waiting in `lucipane wait`.
// - In a plain browser (http://localhost:1420) draw mode is always on, for quick testing.

import { ClearEvent, Stroke } from "@lucipane/contract";
import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  addStroke,
  type Board,
  beginArrow,
  beginBox,
  beginStroke,
  clearAuthor,
  clearBoard,
  emptyBoard,
  endStroke,
  eraseAt,
  extendStroke,
  type Point,
  toSnapshot,
  userStrokesChanged,
} from "./board.ts";
import { applyPenEvent, type PenEvent } from "./pen.ts";
import { type Palette, render } from "./render.ts";
import { clearPanes, type Pane, switchPane } from "./workspaces.ts";

type Tool = "pen" | "box" | "arrow" | "eraser";

const TOOL_LABELS: Record<Tool, string> = {
  pen: "Pen",
  box: "Box",
  arrow: "Arrow",
  eraser: "Eraser",
};

const BEGIN: Record<Exclude<Tool, "eraser">, typeof beginStroke> = {
  pen: beginStroke,
  box: beginBox,
  arrow: beginArrow,
};

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

// The user draws in red, the agent in blue.
const palette: Palette = {
  user: { color: "rgba(255, 60, 60, 0.85)", width: 6 },
  agent: { color: "rgba(70, 160, 255, 0.9)", width: 5 },
};

// The single place board state lives. Only replace it through board.ts functions.
let board: Board = emptyBoard();
// UI state.
let drawMode = !isTauri();
let tool: Tool = "pen";
let erasing = false;
// Handing the drawing over to an agent running `lucipane wait`: leaving draw mode with the
// user's strokes changed counts the submission up, and the board file carries the count.
let submission = 0;
let submittedStrokes = board.strokes;
// Each virtual desktop (macOS Space, GlazeWM workspace) has its own pane. `board` and
// `submittedStrokes` belong to the current one; the others wait here.
let workspace = "default";
let parkedPanes = new Map<string, Pane>();

function update(next: Board): void {
  if (next === board) return;
  const finishedChanged = next.strokes !== board.strokes;
  board = next;
  render(ctx, board, palette);
  if (finishedChanged) saveBoard();
}

/** Keep the board file current for `lucipane capture`. Only finished strokes count. */
function saveBoard(): void {
  if (!isTauri()) return;
  const snapshot = toSnapshot(board, window.devicePixelRatio, submission);
  invoke("save_board", { snapshot }).catch((e: unknown) => {
    console.error("failed to save the board", e);
  });
}

function setDrawMode(enabled: boolean): void {
  const leaving = drawMode && !enabled;
  drawMode = enabled;
  erasing = false;
  if (!enabled) update(endStroke(board));
  if (leaving) submit();
  document.body.classList.toggle("draw-mode", enabled);
}

/** Hand the drawing over, unless the user's strokes are the same as last time. */
function submit(): void {
  if (!userStrokesChanged(submittedStrokes, board.strokes)) return;
  submission += 1;
  submittedStrokes = board.strokes;
  saveBoard();
}

function switchWorkspace(next: string): void {
  if (next === workspace) return;
  const switched = switchPane(parkedPanes, workspace, { board, submitted: submittedStrokes }, next);
  parkedPanes = switched.parked;
  workspace = next;
  board = switched.pane.board;
  submittedStrokes = switched.pane.submitted;
  erasing = false;
  render(ctx, board, palette);
  saveBoard();
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
  render(ctx, board, palette);
}

async function listenToRust(): Promise<void> {
  const channel = new Channel<PenEvent>();
  channel.onmessage = (ev) => update(applyPenEvent(board, ev));
  await invoke("subscribe_pen", { onEvent: channel });
  workspace = await invoke<string>("current_workspace");
  await listen<string>("workspace", (e) => switchWorkspace(e.payload));
  await listen<boolean>("draw-mode", (e) => setDrawMode(e.payload));
  // Forwarded `lucipane clear [--agent]` and `lucipane draw` commands. Their payloads come
  // from another process, so they are validated against the contract before use.
  await listen<unknown>("clear", (e) => {
    const request = ClearEvent.safeParse(e.payload);
    if (!request.success) return console.error("invalid clear event", request.error);
    const clear = (b: Board) => (request.data.agentOnly ? clearAuthor(b, "agent") : clearBoard(b));
    if (request.data.allDesktops) parkedPanes = clearPanes(parkedPanes, clear);
    update(clear(board));
  });
  await listen<unknown>("agent-draw", (e) => {
    const stroke = Stroke.safeParse(e.payload);
    if (!stroke.success) return console.error("invalid agent-draw event", stroke.error);
    update(addStroke(board, stroke.data));
  });
}

function exitDrawMode(): void {
  if (isTauri()) void invoke("set_draw_mode", { enabled: false });
}

function listenToPointer(): void {
  const toPoint = (e: PointerEvent): Point => ({ x: e.clientX, y: e.clientY });
  const LEFT = 0;
  const LEFT_HELD = 1;

  // On macOS, Ctrl+click opens the context menu, whose tracking loop swallows the rest of the
  // drag. Ctrl is easily still held right after the Ctrl+Alt+Shift+D hotkey.
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  canvas.addEventListener("pointerdown", (e) => {
    // Only the left button; the middle button is handled by the global hook.
    if (!drawMode || e.button !== LEFT) return;
    canvas.setPointerCapture(e.pointerId);
    if (tool === "eraser") {
      erasing = true;
      update(eraseAt(board, toPoint(e), ERASER_RADIUS));
    } else {
      const begin = BEGIN[tool];
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
      case "a":
        setTool(tool === "arrow" ? "pen" : "arrow");
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
