# lucipane

A transparent pane between you and your coding agent.

Draw, sketch, and handwrite on top of your screen, then let AI agents (Claude Code and other MCP clients) see exactly what you mean.

> Status: early development. Nothing to install yet.

## Layout

```
contract/   Shared TS types (zod): native <-> daemon messages, MCP tool definitions
daemon/     Node (TS): board state, core logic, MCP server, WebSocket for native
native/     Tauri app: Rust shell (overlay window, input hooks, capture) + TS canvas UI
docs/       Requirements and notes
```

## Development

Prerequisites: Node 22+, pnpm 10, Rust (stable), and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.
On Windows, build `native/` from Windows itself (not WSL).

```sh
pnpm install
pnpm test          # vitest in contract/ and daemon/
pnpm typecheck
pnpm lint          # biome
pnpm dev:daemon
pnpm dev:native    # tauri dev
```

## Tiling window managers

lucipane shows a full-screen transparent overlay. Tiling window managers must leave it alone,
or they will tile it like a normal window.

**GlazeWM** (Windows) — `~/.glzr/glazewm/config.yaml`:

```yaml
window_rules:
  - commands: ["ignore"]
    match:
      - window_process: { equals: "lucipane" }
```

**AeroSpace** (macOS) — `~/.aerospace.toml`:

```toml
[[on-window-detected]]
if.app-id = 'dev.aspooky.lucipane'
run = 'layout floating'
```

Note: AeroSpace hides windows of inactive workspaces by moving them off-screen, so the overlay
may follow a single workspace. This is not verified yet.
