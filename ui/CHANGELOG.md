# Changelog — ui

`UI.asi`: the runtime the game's HUD, menus and loading screen will be redrawn with, in
our own code (`PLAN.md`). Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. Details are in [`README.md`](README.md).

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
