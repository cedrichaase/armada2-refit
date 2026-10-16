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

## How the takeover works

Decided before steps 2–4 were started, so that every panel and screen is built the
same way.

### Menus: the stock dialog runs, hidden; we draw it and drive it

Every shell screen is a modal `DialogBoxParamA` whose dialog procedure *is* the
screen's logic: what it reads, what it writes, which `g_shellMode` it names next, what
it returns. Rewriting 38 procedures would mean describing each of them. Instead,
`UI.asi` hosts the stock dialog:

- **The dialog host** (`src/engine/dialogs.rs`) takes `DialogBoxParamA`. It creates the
  stock template, hidden, as a child of the 3D window, and runs a loop of its own in
  place of user32's modal one. Each pass pumps the queue (timers and the procedure's
  own messages still run) and draws a frame through the device, as `run_screen` does.
  `EndDialog` on a hosted dialog records the result and ends the loop, so the caller
  gets what the stock dialog returns. Nested dialogs nest the loop.
- **The control model** (`src/menus/model.rs`) reads the hidden dialog every frame:
  each child control's id, class, style, design rectangle, text, check state, list
  items, selection, enabled and visible flags. Labels therefore come from the game's
  own (localised) strings. Acting on a control sends the dialog what the stock control
  would: the click a button sends, the text an edit holds, a list's selection
  change.
- **A screen** is drawn from that model by `src/menus/`. Every dialog gets the
  **generic screen**, which draws every control by kind in its design place, scaled to
  fit. A screen that has been laid out gets a **screen spec**, Rust code that places
  the same controls, by id, in a layout of our own. A control the model cannot read
  (owner-drawn art, a preview picture) is mirrored: printed off the hidden window into
  a texture. So every menu works from the start, and laying out a screen changes only
  how it looks.
- **Menus.asi is superseded, not run alongside.** While `UI.asi` hosts the dialogs,
  `Menus.asi` must not also hook them; the menu part of `UI.asi` stands down if it
  finds `Menus.asi` loaded, and the install takes `Menus.asi` out when the UI's menus
  are on. `UI.asi` raises the shell's display mode itself.

### HUD: the stock panels keep running; we draw them and route their input

The HUD is a list of `DisplayInterface` components. Their state (which buttons the
palette shows, what the info panel describes) is worked out in their simulate step,
and their pictures and clicks are separate steps.

- **One switch per panel.** For a panel we have replaced, its `Render` and its mouse
  handling are skipped. The skip is made where `DisplayInterface` walks its components,
  by one hook for all panels, never in a vtable. The component's simulate step keeps
  running, so its state stays current.
- **Our panel reads that state** from the component and from the engine through
  `src/engine/` (one file per area, step 2). It **acts by calling the component's own
  functions** (the press function of a palette button, as `GridLayout.asi` does) or the
  game's commands. It never goes around them.
- **The layout is ours**: anchored to the screen, at any aspect, in the style of
  `STYLE.md`. No `gui_*.cfg` rectangle is read, except for what the engine needs to
  hit-test where we have not taken over.
- **HUD.asi and GridLayout.asi are superseded panel by panel.** The UI's button bar has
  the grid keys of its own (`grid/README.md` is the behaviour to match). The UI takes
  the bar only while `GridLayout.asi` is not loaded. When the UI owns every panel, the
  install takes `HUD.asi` and `GridLayout.asi` out.

### Art

`STYLE.md` holds the visual language: our own theme, one per race, drawn by our
renderer, with the game's art (unit portraits, wireframes, icons, the cursor shapes)
used where it carries information. The style is decided once and every panel and
screen follows it.

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
