# Changelog — hud

The in-game HUD: its layout canvas, its font and its cursors — `HUD.asi` since 2.0.0,
before that `ui-widescreen.py`, `cursor-aspect.py` and (as the separate `font` layer)
`ui-font-condense.py`. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. The derivations are in [`README.md`](README.md).

## 2.0.0 — 2026-09-26

### Added
- `HUD.asi` (`hud.c`, `build.sh`, `install.sh`, `HUD.ini`, `1524bb9`): the canvas and palette, the
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

Not installed yet, not yet seen in game. The map-plane cursor path is unconfirmed (see
`README.md`).

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
  files rebuild from stock byte-for-byte (`36ce1a7`).

#### 2026-09-21
- The condense landed (`e124156`) and was confirmed in game: `OBJECTIVES:` measured
  within 0.5% of the prediction (`c5487c8`).
- `--method runs` became the default (`b6941ef`). It was rejected in game and
  `resample` became the default again (`c81f534`).
