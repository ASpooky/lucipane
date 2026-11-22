# lucipane

A transparent pane between you and your coding agent.

Draw, sketch, and handwrite on top of your screen, then let AI agents (Claude Code and any
agent with a shell) see exactly what you mean.

> Status: early development. Nothing to install yet.

## Using it

Run `lucipane` to start the overlay. Then:

| Action                  | How                                             |
| ----------------------- | ----------------------------------------------- |
| Draw anywhere, any time | Hold the middle mouse button and drag           |
| Enter / leave draw mode | `Ctrl+Alt+Shift+D` (`Esc` also leaves)          |
| In draw mode            | Left-drag draws. `B` box, `E` eraser, `C` clear |

Agents use the same executable as a CLI:

```sh
lucipane capture [--out <file.png>]   # screenshot (with your drawings) + annotations as JSON
lucipane clear                        # erase everything on the overlay
```

`capture` runs in its own process: the drawings are part of the screenshot, and their
geometry comes from a board file the overlay keeps up to date
(`<data dir>/dev.aspooky.lucipane/board.json`). `clear` is forwarded to the running overlay.
No network port is opened.

### Agent skill

[`skills/lucipane/SKILL.md`](skills/lucipane/SKILL.md) teaches an agent when and how to use
the CLI. For Claude Code, copy or link the folder into `~/.claude/skills/` and make sure
`lucipane` is on your `PATH`.

## Layout

```
contract/   Documents shared across processes (zod schemas + JSON fixtures)
native/     Tauri app: Rust (overlay window, input hooks, capture, CLI) + TS canvas UI
skills/     Agent skill for the CLI
docs/       Requirements and notes
```

The fixtures in `contract/fixtures/` are parsed by both the zod tests and the Rust tests, so
the TS and Rust sides cannot drift apart silently.

## Development

Prerequisites: Node 22+, pnpm 10, Rust (stable), and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.
On Windows, build `native/` from Windows itself (not WSL).

```sh
pnpm install
pnpm test          # vitest in contract/ and native/
pnpm typecheck
pnpm lint          # biome
pnpm dev:native    # tauri dev
cd native/src-tauri && cargo test

# The dev build doubles as the CLI:
native/src-tauri/target/debug/lucipane capture
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
