---
name: lucipane
description: See what the user drew on their screen with the lucipane overlay (circles, arrows, boxes, handwriting), together with the screen behind it, and point back by drawing boxes, arrows and labels yourself. Use when the user points at something on screen ("this", "here", "the part I marked", "look at my screen"), when showing them where something is on screen helps your answer, or when they ask to clear the drawings.
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
  "contractVersion": 3,
  "image": {
    "path": "C:\\...\\lucipane\\capture-1795370401068.png",
    "width": 1920,
    "height": 1080
  },
  "annotations": [
    { "kind": "box", "author": "user", "x": 640, "y": 300, "width": 420, "height": 180 },
    { "kind": "arrow", "author": "user", "from": [200, 820], "to": [600, 480] },
    { "kind": "freehand", "author": "user", "points": [[200, 820], [260, 760], [330, 700]] }
  ]
}
```

Then read the image at `image.path`. The marks are visible in it, and `annotations` gives their
geometry in the image's pixels (origin top-left):

- `box`: the user framed this area. It is usually the subject of their question.
- `arrow`: points from `from` to `to`; the head is at `to`.
- `freehand`: a circle, underline, hand-drawn arrow or handwriting. Points are in drawing
  order (simplified), so the last point is where the pen lifted, often the head of an arrow.
- `text`: a label at `x`, `y` (its top-left corner).

`author` is `user` for the user's marks (drawn in red) and `agent` for marks an agent placed
(drawn in blue). Answer about the user's marks; the agent's are earlier replies.

Look at what lies under or inside each mark in the image before answering. When several marks
are present, they may be related (an arrow from one box to another), so read them together.

The screenshot shows the whole display and may include private information. Capture only when
the request is about what is on the user's screen.

`--out <file.png>` writes the image to a chosen path instead of a temporary file.

## Point back at the screen

You can draw on the overlay too, in blue, to answer by pointing: "is this the button you
mean?", "the setting is here", numbered boxes for review comments.

```sh
lucipane draw box   --x 640 --y 300 --w 420 --h 180
lucipane draw arrow --from 200,820 --to 600,480      # head at --to
lucipane draw text  --x 650 --y 270 --text "1"
lucipane draw path  --d "M 600 400 C 650 350, 750 350, 800 400"   # SVG path data
```

Coordinates are pixels of the latest `lucipane capture` image, so capture first and place
marks from what you see in it.

`draw path` takes SVG path data (the `d` attribute of `<path>`): `M`, `L`, `H`, `V`, `C`, `S`,
`Q`, `T`, `A` and `Z`, absolute or relative. Use it to circle or underline something, for
curved arrows, or to sketch a small picture. Each subpath (each `M`) becomes its own line.
Curves are flattened to points; a path may have up to 4000 of them after flattening.

- Draw only as part of answering the user, about the screen they asked about. Capture right
  before drawing: the screen may have changed (another app, a game) since the last capture.
- Keep it to a few marks and pair them with your reply ("box 1: …").
- Remove your own marks once they have served their purpose, or before drawing a new set:
  `lucipane clear --agent` (the user's marks stay).

## Clear the marks

```sh
lucipane clear           # everything
lucipane clear --agent   # only your marks
```

Clear the user's marks only when the user asks for it.

## When something is off

- Exit code 1: the capture or clear failed. The reason is on stderr. `the overlay is not
  running` means the user has to start lucipane first.
- `annotations` is empty although the user says they drew something: the overlay may have been
  restarted, which starts with an empty board. Ask the user to draw again.
- Under WSL, call the Windows executable (`lucipane.exe`) and convert the image path before
  reading it: `wslpath -u '<image.path>'`.
