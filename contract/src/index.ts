/**
 * Contract layer. Documents that cross a process or language boundary:
 *   board.ts     strokes on the overlay (TS state, written to the board file)
 *   snapshot.ts  the board file (overlay app -> CLI)
 *   capture.ts   `lucipane capture` output (CLI -> agent)
 *   events.ts    forwarded commands (Rust shell -> overlay WebView)
 * fixtures/ holds sample documents that both the zod tests here and the Rust tests in
 * native/src-tauri parse, so the two implementations cannot drift apart silently.
 */
export * from "./board.ts";
export * from "./capture.ts";
export * from "./events.ts";
export * from "./snapshot.ts";
export * from "./version.ts";
