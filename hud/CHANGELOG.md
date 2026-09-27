# Changelog — hud

The in-game HUD: its layout canvas, its font and its cursors — `HUD.asi` since 2.0.0,
before that `ui-widescreen.py`, `cursor-aspect.py` and (as the separate `font` layer)
`ui-font-condense.py`. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. The derivations are in [`README.md`](README.md).

## 2.2.0 — 2026-09-27

### Added
- `Relayout=` in `HUD.ini` (default 1, `9873980`).

### Fixed
- The HUD follows a display mode changed in a mission (Graphics Settings). Before, it
  kept the layout of the mode the mission started in: from 21:9 to 1600x1200 every
  panel drew at 0.56 of its width. `HUD.asi` now rebuilds the panels that read rects
  (`Cleanup`, `Init`, `PostLoad`, as between missions) with the canvas for the new mode,
  from the `SimulateAll` call in `Program::DisplayInputProcess` (`0x48380a`). The
  overview, which holds the unit groups, is not rebuilt; `MapRadar`'s calls to
  `Scanner::CleanupGrids`, `Scanner::InitializeGrids` and `Terrain_Geometry::PostLoad`
  are skipped during it, so the fog of war stays; and the briefing's file name and
  objective completion are handed across it, as a saved game's `Load` does.

Confirmed in game 2026-09-27.

## 2.1.1 — 2026-09-27

### Changed
- `install.sh`, `cursor-aspect.py`, `ui-font-condense.py` and `ui-widescreen.py` find the game through `a2env.sh` / `a2env.py` (`A2_GAME`, then
  `~/.config/armada2-remastered.conf`, then Heroic's default) instead of a hard-coded
  path.

Installs nothing different; nothing to see in game.

## 2.1.0 — 2026-09-26

### Added
- `Seams=` in `HUD.ini` (default 1).

### Fixed
- No more 1–2 px gaps between UI tiles where the scale is fractional — the lines through
  the briefing panel and along the joins of pieced HUD frames at 16:10, 16:9 and 21:9.
  `HUD.asi` snaps a sprite's far edge like its near one in `DrawScaled2D` (`0x63aeca`)
  and converts a rect's far edge rather than its width in `Get(DBRectangle)`
  (`0x5358f0`).
- No faint line one pixel outside UI sprites with MSAA on — along the briefing's edge,
  the minimap frame and the command bar, plainest over unexplored space. Snapped edges go
  to `floor(v) + 0.5` (a pixel boundary) instead of stock's `+ 0.25`, which left a
  quarter-covered pixel that MSAA shaded with a wrapped texture coordinate.
- The same lines around the action bar's command buttons: sprites without flag `0x80`
  were never snapped and drew at fractional positions. Every 2D sprite is now snapped
  (the `je` at `0x63aec8` that skipped the block is NOP'd).

Confirmed in game 2026-09-26.

## 2.0.1 — 2026-09-26

### Fixed
- The cursor is no longer drawn 1.79x wide under DXVK. The game draws it itself (the
  synchronous path, `RefreshDisplay`), which 2.0.0 did not reach: `HUD.asi` now wraps
  that draw (`0x6246fa`) with a square scale and a hotspot-preserving position.

Confirmed in game 2026-09-26.

## 2.0.0 — 2026-09-26

### Added
- `HUD.asi` (`hud.c`, `build.sh`, `install.sh`, `HUD.ini`, `f5f6721`): the canvas and palette, the
  font and the hardware cursor corrected in the engine at run time, for the display
  mode actually set, with every game file stock. `HUD.ini` switches each part
  (`Canvas=`, `Font=`, `Cursor=`) and `Log=` writes `HUD.log`.
- `install.sh` reverts the three file-based fixes first, and drops the
  `misc/gui_*.cfg` and `Curs_*` backups that their `--revert`s leave beside files now
  identical to them. Each part of `HUD.asi` stands down if its file-based fix is found.

### Changed
- The `font` layer is merged into `hud`: `font/ui-font-condense.py` is now
  `hud/ui-font-condense.py`, and its notes are the font section of `README.md`.
  `a2mod` files `HUD.asi`/`.ini`, the `.a2font-backup`s, `misc/` and the `Curs_*` art all
  under `hud`, and reads older manifests' `font`/`hud layout` entries as `hud`.
- The file-based scripts are no longer what installs; they remain as the derivation and
  as the `--revert` that `install.sh` runs.

Confirmed in game 2026-09-26.

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
- Moved to `hud/`. The HUD sections of `SETUP.md` became `README.md` (`7daf6d8`,
  `a38d137`).

### 2026-09-21
- The action bar anchor is corrected against the hard-coded 1600 (`1a34f25`) and made
  flush with the info panel (`19a09ca`).
- Cursors: a first attempt rewrote `cursor.spr` and fixed only the map-plane cursor
  (`1c7be05`). It was replaced by squashing the art, which covers both draw paths
  (`bdbe2af`). Confirmed in game (`e74cd41`).

### 2026-09-20
- `ui-widescreen.py`: the layout canvas re-declared. Confirmed in game (`27fff6f`).

## The font layer, before it merged into hud

It was versioned on its own until hud 2.0.0.

### 1.0.0 — 2026-09-25

Baseline: the first versioned release.

- Glyph art and advance widths are condensed by `1.25 * H / W` (0.5233 at 3440x1440)
  with `--method resample`, the default. `--check` verifies that the `.spr` and `.tga`
  files agree, and `--revert` restores stock. Confirmed in game.
- `--method runs` is kept but not shipped. It was rejected in game.

#### Before versioning

#### 2026-09-25
- Ported into `font/` when the `worktree-font-condense` branch merged. The installed
  files rebuild from stock byte-for-byte (`15d55b4`).

#### 2026-09-21
- The condense landed (`72b4aee`) and was confirmed in game: `OBJECTIVES:` measured
  within 0.5% of the prediction (`5d5bd99`).
- `--method runs` became the default (`bef4682`). It was rejected in game and
  `resample` became the default again (`aef7faa`).
