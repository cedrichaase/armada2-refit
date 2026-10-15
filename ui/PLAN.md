# UI — the plan

Own everything the game shows outside the 3D view: the in-mission HUD, the front-end
menus and the loading screen, drawn by `UI.asi` in our own code, as render3d owns the
3D view. Today three plugins correct the stock UI from outside: `HUD.asi` (layout,
font, cursors at any aspect), `GridLayout.asi` (the button bar as a grid) and
`Menus.asi` (the GDI shell scaled and embedded). Each fix there works against a
fixed-pixel design. A UI of our own can be laid out for any screen, extended freely
and drawn at the back buffer's own resolution.

The steps follow render3d's: a seam, then parts taken over one at a time, each
switchable, each falling back to stock.

## 1. The runtime (this step)

What every later screen is built from, and nothing the player sees by default:

- **A 2D renderer** on the Direct3D 9 device behind d3d8to9 (`src/gfx/`): one shader
  pair for every primitive (solid, gradient, image, nine-slice, signed-distance text,
  antialiased rounded rectangles), batched into a few `DrawIndexedPrimitiveUP` calls a
  frame, clipped by scissor, the device's state saved and put back around it.
- **Text** (`src/text/`): an OFL font vendored in `platform/vendor/`, baked at build
  time into a signed-distance atlas, so one atlas is sharp at every size.
- **Widgets** (`src/widgets/`): immediate mode, sized in units of a 1080-pixel-tall
  screen times `[UI] Scale`. Panels claim the mouse from the engine.
- **Input** (`src/input.rs`): the pointer and keys as the UI sees them, and capture,
  which keeps from the engine what the UI takes.
- **Three drawing contexts** (`src/engine/`): a mission's frame after the engine's
  HUD; a screen loop of our own in the front end; the loading screen.

`[Demo]` in `UI.ini` draws a demonstration panel in each context; with the defaults
the plugin patches its hooks and draws nothing.

## 2. The engine API

Reads and actions, kept in one module per area, each tagged with the panel that needs
it: the selection and what it can do, a unit's state, resources, the minimap's
picture, messages and objectives. Actions go through the game's own commands
(the orders `QOL.asi` and `GridLayout.asi` already send), never around them, so a
player with the UI plays a player without it.

## 3. The HUD, a panel at a time

Each `DisplayInterface` panel (`hud/README.md`) replaced by one of ours, behind a
switch, the stock panel's `Render` skipped while ours draws. Likely order: the button
bar (where `GridLayout.asi` already owns the logic), the info panel, the selection,
resources and messages, then the minimap. `HUD.asi`'s canvas, font and seam fixes stop
mattering for each panel ours replaces; when the last goes, `HUD.asi` and
`GridLayout.asi` are superseded.

## 4. The menus, a screen at a time

A `do_*` screen replaced by a screen of ours run in the shell's loop, returning what
the stock dialog returns and writing the same settings. Start with the screens that
only read and write options (Graphics, Sound, Game Settings), then Load/Save and the
mission lists, then Instant Action's setup. The multiplayer screens come last: their
state is DirectPlay's and the lobby's, and `Online.asi` will own the game list anyway.
When the last goes, `Menus.asi` is superseded.

## 5. The loading screen

Ours in place of `logo.SOD`'s quads and the percentage box: art at any aspect,
progress, the mission's name and briefing.

## What this exposes of Armada2.exe

A UI of our own reads much more of the engine's state than today's plugins patch
(`hud/README.md`, "Engine findings" in `publish/README.md`). It stays within hard rule
9 by construction:

- **Every address and offset lives in `src/engine/`**, one file per area, each with
  the hook or panel that needs it. Removing a panel removes its findings.
- **Fields are read by offset, never through a mirrored layout**: no `#[repr(C)]`
  struct that copies a class's shape.
- **Behaviour is specified from the running game** (the test bench), not by
  translating the engine's code: a screen of ours does what the stock screen is seen
  to do, written in our own structure.
- **Call the engine where its logic will do**: a call names one address, a
  re-implementation describes a function.

## Not decided yet

- Our own cursor (the mission's is the engine's sprite, `hud/README.md`).
- Text entry (IME, the game's own chat box).
- Whether `HUD.asi`'s fixes stay for panels we have not replaced, or the stock panels
  are hidden as a whole once enough are ours.
