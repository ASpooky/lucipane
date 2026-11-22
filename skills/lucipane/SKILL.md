---
name: lucipane
description: See what the user drew on their screen with the lucipane overlay (circles, arrows, boxes, handwriting), together with the screen behind it. Use when the user points at something on screen ("this", "here", "the part I marked", "look at my screen") or asks you to clear their drawings.
---

# lucipane

The user has a transparent overlay on their primary display. They draw on it (red strokes and
boxes) to point at things, then ask you about them. The `lucipane` command lets you see the
screen with those marks and their exact positions.

## See what the user marked

```sh
lucipane capture
```

It prints one JSON document:

```json
{
  "contractVersion": 1,
  "image": {
    "path": "C:\\...\\lucipane\\capture-1795370401068.png",
    "width": 1920,
    "height": 1080
  },
  "annotations": [
    { "kind": "box", "x": 640, "y": 300, "width": 420, "height": 180 },
    { "kind": "freehand", "points": [[200, 820], [260, 760], [330, 700]] }
  ]
}
```

Then read the image at `image.path`. The marks are visible in it in red, and `annotations`
gives their geometry in the image's pixels (origin top-left):

- `box`: the user framed this area. It is usually the subject of their question.
- `freehand`: a circle, underline, arrow or handwriting. Points are in drawing order, so the
  last point is where the pen lifted, often the head of an arrow.

Look at what lies under or inside each mark in the image before answering. When several marks
are present, they may be related (an arrow from one box to another), so read them together.

The screenshot shows the whole display and may include private information. Capture only when
the request is about what is on the user's screen.

`--out <file.png>` writes the image to a chosen path instead of a temporary file.

## Clear the marks

```sh
lucipane clear
```

Erases every mark on the overlay. Run it only when the user asks for it.

## When something is off

- Exit code 1: the capture or clear failed. The reason is on stderr. `the overlay is not
  running` means the user has to start lucipane first.
- `annotations` is empty although the user says they drew something: the overlay may have been
  restarted, which starts with an empty board. Ask the user to draw again.
- Under WSL, call the Windows executable (`lucipane.exe`) and convert the image path before
  reading it: `wslpath -u '<image.path>'`.
