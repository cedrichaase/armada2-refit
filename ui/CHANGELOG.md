# Changelog — ui

`UI.asi`: the runtime the game's HUD, menus and loading screen will be redrawn with, in
our own code (`PLAN.md`). Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. Details are in [`README.md`](README.md).

## 0.2.0 — 2026-10-16

### Added
- Widgets: text field (with the clipboard), scroll area, list, dropdown, tabs, check
  box and radio, image button, number spinner, tooltip, modal; keyboard focus (Tab,
  Shift+Tab, Enter, Esc) and an overlay layer (481b796).
- Our own cursor, from the game's cursor art, square at any aspect: `[UI] Cursor`
  (default 0), `CursorSize` (481b796).
- The style (`STYLE.md`): one theme per race and one for the shell, drawn from tokens
  (`widgets/theme.rs`); shader kind 4 (chamfered or rounded corners, stroke, glow,
  elbow, stripes); `[UI] Theme` (default `auto`) (08e2901).
- `[Demo] Shell=1` shows a widget gallery; `[Demo] Style=1` a style gallery (481b796,
  08e2901).
- The HUD, a panel at a time (`HUD.md`): one switch skips a stock panel's drawing and
  mouse while it keeps simulating; engine reads for the button bar, sprites, sounds,
  the selection and resources; our own button bar as a 5x3 grid with its keys, which
  stands down while `GridLayout.asi` is loaded. `[HUD] Enable`, `Bar` (default 0),
  `BarKeys`, `BarLabels`; `bench-asi.sh --no-grid` (0e0cac9).
- The menus (`MENUS.md`): the stock shell dialogs hosted hidden and drawn by the UI
  (the dialog host, a registry of the shell's widgets, the control model, a generic
  mirrored screen and a spec for the main menu); the shell's display mode raised to
  the desktop size; the in-mission menus over the last frame, dimmed. `[Menus] Enable`
  (default 0), `Generic`, `Specs`, `EscapeReturns`, `Trace`, `Host`;
  `bench-asi.sh --menus` (f0fe918).

### Fixed
- A click no longer gives the keyboard focus to a button, toggle, slider, check box or
  tab: it kept every key from the engine and Esc from the screen (345bc74).

Installed, not yet seen in game.

## 0.1.0 — 2026-10-16

### Added
- `UI.asi`, a `no_std` Rust plugin built by `ui/build.sh`, with `UI.ini` (`[UI] Enable`,
  `Log`, `Scale`; `[Demo] Mission`, `Shell`, `Loading`). Not in `./install` or `a2mod`
  yet; `ui/bench-asi.sh` puts it into a bench clone.
- The 2D renderer on the Direct3D 9 device behind d3d8to9: one shader pair
  (`shaders/ui.hlsl`) for solid, gradient, image, nine-slice, signed-distance text and
  rounded rectangles, batched, scissor-clipped, the device's state saved and restored.
- Text: Noto Sans Condensed Medium and SemiBold (OFL-1.1, vendored in
  `platform/vendor/noto-sans-condensed-2.015/`), baked at build time by `ui/fontbake`
  into signed-distance atlases, with GPOS kerning and word wrap.
- Immediate-mode widgets sized in units of a 1080-pixel-tall screen: panel, window,
  label, note, button, toggle, slider, progress; `Anchor` and `Stack` layout.
- Input from the game window's messages, with capture: what the UI takes (clicks and
  the wheel over its panels, keys for a focused field) never reaches the engine.
  The engine's cursor can be hidden a frame at a time, and its shape read.
- Three drawing contexts: a mission's frame after the HUD and chat; a screen loop of
  our own in the front end (`[Demo] Shell=1` puts one in the Credits' place); the
  loading screen, with the load's progress.

Installed, not yet seen in game.
