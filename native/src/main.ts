// Spike ①: full-screen transparent canvas with a test line.

const canvas = document.querySelector<HTMLCanvasElement>("#board");
if (!canvas) throw new Error("#board canvas not found");
const ctx = canvas.getContext("2d");
if (!ctx) throw new Error("2d context unavailable");

function resize(c: HTMLCanvasElement, g: CanvasRenderingContext2D): void {
  // Back the canvas with physical pixels so lines stay sharp on high-DPI displays.
  const dpr = window.devicePixelRatio;
  c.width = Math.round(window.innerWidth * dpr);
  c.height = Math.round(window.innerHeight * dpr);
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  drawTestLine(g);
}

function drawTestLine(g: CanvasRenderingContext2D): void {
  g.strokeStyle = "rgba(255, 60, 60, 0.8)";
  g.lineWidth = 6;
  g.lineCap = "round";
  g.beginPath();
  g.moveTo(100, 100);
  g.lineTo(window.innerWidth - 100, window.innerHeight - 100);
  g.stroke();
}

window.addEventListener("resize", () => resize(canvas, ctx));
resize(canvas, ctx);
