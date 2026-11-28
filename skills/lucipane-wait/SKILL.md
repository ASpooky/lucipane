---
name: lucipane-wait
description: Keep listening for drawings on the lucipane overlay and answer each one as it is handed over, alongside the normal chat. Start it only when the user invokes it.
disable-model-invocation: true
---

# lucipane-wait

From now on in this session, the user talks to you in two ways: by chat as usual, and by
drawing on the lucipane overlay. A drawing is handed over when the user leaves draw mode
(`Esc` or `Ctrl+Alt+Shift+D`) after changing their marks. Answer each handover without
waiting for a chat message about it.

This builds on the `lucipane` skill: read it for what the capture output means and how to
draw back.

## Listen

Run this in the background, so the session stays free for chat:

```sh
lucipane wait
```

In Claude Code, use the Bash tool with `run_in_background: true`. You are notified when the
command exits.

`lucipane wait` blocks until the next handover, then prints exactly what `lucipane capture`
prints (image path and annotations) and exits 0. `--out <file.png>` chooses the image path.

## On each handover

1. Read the printed JSON and the image. Treat the user's marks as their message: a circle or
   box asks "what is this?" or "look here", handwriting is text to read. Use the chat so far
   for context.
2. Reply in chat, and point back with `lucipane draw` when it helps (see the `lucipane`
   skill). Clear your previous marks first with `lucipane clear --agent` if they no longer
   apply.
3. Start `lucipane wait` in the background again. Exactly one wait should be running at a
   time: do not start another while one is still running.

If the user also sends a chat message right after a handover, answer both together rather
than twice.

## Stop

Stop listening when the user says so: stop the background wait and do not start another.

## When something is off

- `lucipane wait` exits 1 with `the overlay is not running`: the overlay was closed or never
  started. Tell the user and stop listening; start again when they ask.
- Any other exit 1: the reason is on stderr. Tell the user and stop listening.
