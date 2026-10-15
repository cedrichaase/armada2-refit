# UI — the runtime for a HUD, menus and loading screen of our own

`UI.asi` is the runtime that the game's HUD, front-end menus and loading screen will
be redrawn with, in our own code (`PLAN.md`). This first step is the runtime alone:
a 2D renderer on the game's Direct3D 9 device, text, widgets, input with capture, and
a hook into each of the three places the UI draws. **With the shipped `UI.ini` it
draws nothing**; `[Demo]` turns on a demonstration panel per context.

It is a `no_std` Rust DLL like render3d (`render3d/README.md`, "Building and
installing"), and is **not in `./install` or `a2mod` yet**: nothing uses it until a
panel or screen of ours replaces a stock one. The bench gets it with
`ui/bench-asi.sh` (below).

<!-- CONTEXTS -->

## Building and installing

    ui/build.sh                                   # -> ui/build/UI.asi
    ui/bench-asi.sh <state.json> --demo mission,shell,loading [UI.Scale=1.5]

`build.sh` compiles `shaders/*.hlsl` with render3d's `hlsl-rs.sh` (the generated
`src/shaders/*.rs` is committed and stamped, so a machine without `vkd3d-compiler`
builds) and the font atlas from the vendored TTFs (`src/text/`), then the DLL.
`bench-asi.sh` copies `UI.asi` and `UI.ini` into a bench session's clone and sets the
keys it is given; it is what a scenario's `Setup:` line runs.

## `UI.ini`

| Key | Default | |
|---|---|---|
| `[UI] Enable` | 1 | 0: the plugin patches nothing |
| `[UI] Log` | 1 | `UI.log` in the game directory: which hooks are in, and why one stood down |
| `[UI] Scale` | 1.0 | the UI's size, as a multiple of a 1080-pixel-tall screen |
| `[Demo] Mission`, `Shell`, `Loading` | 0 | the demonstration panel in that context |

## The renderer (`src/gfx/`)

Everything a frame of UI draws goes into one vertex list and is drawn with **one
shader pair** (`shaders/ui.hlsl`), so a frame costs a handful of draw calls whatever is
on screen. Each vertex carries its *kind*: solid colour, textured, signed-distance
text, or an analytic rounded rectangle (its distance to the edge computed per pixel,
antialiased over one pixel, so corners are smooth at any radius without a texture).
The pixel shader weights the kinds with `step()` rather than branching on them, as
vkd3d needs (`render3d/README.md`). A new draw call starts only when the texture or the
clip rectangle changes; solid quads take whatever texture the run has, since they do
not sample it.

**State.** d3d8to9 keeps its own record of what the game bound (`platform/D3D9.md`), so
the frame is drawn between a `D3DSBT_ALL` state block's capture and its apply: once a
frame, not per draw, which is where render3d found `SBT_ALL` expensive. Positions are
in back-buffer pixels and move half a pixel on the way to clip space, so a rectangle
with whole-number edges covers whole pixels exactly.

**No `Reset` to handle.** Textures are in the managed pool and the vertices go through
`DrawIndexedPrimitiveUP`, so nothing of ours is in the default pool. Every texture is
registered once with its pixels (the font atlases from the DLL, images from the game's
TGAs through `a2fmt`) and made on each device the first time it draws there.

**Without a Direct3D 9 device** (DXVK's own d3d8, GOG's d3d8to9) `surface::frame` returns
at once and the game draws as stock.

## Text (`src/text/`)

<!-- TEXT -->

## Widgets (`src/widgets/`)

Immediate mode: a widget is a call made every frame, which lays itself out, draws, and
answers for its input at once (`if ui.button(r, "Launch") { ... }`). What has to
outlive a frame is kept by widget id, a hash of the label and the scopes it is in:
which widget a press started on, so a click is a release over the same widget.

Sizes are **units**, pixels of a 1080-pixel-tall screen times `[UI] Scale`
(`Ui::u`), so a layout is the same at every resolution and aspect; positions come
from `Anchor` (a rectangle against a screen edge or corner) and `Stack` (rows down a
rectangle). Panels and every interactive widget **claim** the pointer: while it is
over one, `Ui::end` asks `input` to keep the mouse from the engine, from the next frame.

The widgets so far are what the first screens need: `panel`, `window` (a panel with a
title), `label`, `note`, `button`, `toggle`, `slider` and `progress`, over the drawing
calls (`rect`, gradients, `rect_round`, `frame`, `line`, `image`, `nine`, `text`).
