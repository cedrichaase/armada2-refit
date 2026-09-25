# Changelog — hud

The in-game HUD layout (`ui-widescreen.py`) and the cursors (`cursor-aspect.py`).
Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first.
The derivations are in [`README.md`](README.md).

## 1.0.0 — 2026-09-25

Baseline: the first versioned release.

- `ui-widescreen.py`: re-declares the `gui_<race>.cfg` layout canvas at the display
  aspect and moves the right-anchored and centred panels. `popupPaletteXA`/`XB` are
  handled in the palette's own 1600 space, and `XA` sits flush with the info panel.
  `--revert` undoes it. Confirmed in game.
- `cursor-aspect.py`: squashes the 20 cursor textures 32 → 18 texels across, about
  each `@origin` hotspot. `cursor.spr` stays stock. Confirmed in game.
- Not handled: the bridge display (see `README.md`).

## Before versioning

### 2026-09-25
- Moved to `hud/`. The HUD sections of `SETUP.md` became `README.md` (`c3ae017`,
  `9d6bf1b`).

### 2026-09-21
- The action bar anchor is corrected against the hard-coded 1600 (`9e73db3`) and made
  flush with the info panel (`3c7dd63`).
- Cursors: a first attempt rewrote `cursor.spr` and fixed only the map-plane cursor
  (`46aa0f4`). It was replaced by squashing the art, which covers both draw paths
  (`b49c9da`). Confirmed in game (`24260de`).

### 2026-09-20
- `ui-widescreen.py`: the layout canvas re-declared. Confirmed in game (`d5223bd`).
