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
lucipane clear [--agent] [--all]      # erase this desktop's pane (or only what agents drew);
                                      # --all: every desktop's pane
lucipane draw box --x X --y Y --w W --h H         # draw as the agent, in blue,
lucipane draw arrow --from X,Y --to X,Y           # in pixels of the capture image
lucipane draw text --x X --y Y --text TEXT
lucipane draw path --d "M 10 10 C 20 0, 40 0, 50 10"   # SVG path data: curves, circles, sketches
lucipane wait [--out <file.png>]      # block until you leave draw mode after drawing,
                                      # then print the same JSON as capture
```

`capture` and `wait` run in their own process: the drawings are part of the screenshot, and
their geometry comes from a board file the overlay keeps up to date
(`<data dir>/dev.aspooky.lucipane/board.json`). Leaving draw mode with changed drawings counts
up a submission number in that file, which is what `wait` watches. `clear` and `draw` are
forwarded to the running overlay.
No network port is opened.

Each virtual desktop has its own pane: a macOS Space (every full-screen window gets one) or,
on Windows, a GlazeWM workspace (read over GlazeWM's IPC on `localhost:6123`). Drawings stay
with their desktop and come back when you return; `capture`, `wait`, `clear` and `draw` work on
the desktop you are on. Without GlazeWM, Windows has a single pane. On macOS the overlay is a
non-activating panel without a Dock icon, so it also floats over full-screen apps.

### Agent skill

[`skills/lucipane/SKILL.md`](skills/lucipane/SKILL.md) teaches an agent when and how to use
the CLI.
[`skills/lucipane-wait/SKILL.md`](skills/lucipane-wait/SKILL.md) is started by hand
(`/lucipane-wait` in Claude Code): the agent then answers every drawing you hand over by
leaving draw mode, without a chat message about it, while the chat keeps working as usual.

For Claude Code, copy or link both folders into `~/.claude/skills/` and make sure `lucipane`
is on your `PATH`.

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
