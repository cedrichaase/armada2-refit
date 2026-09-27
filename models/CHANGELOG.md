# Changelog — models

`SOD` geometry: the widened mission loading screen (`logo-sod.py`, `loading-panel.sh`).
It ships in lockstep with `textures/targets/LOADING`, and `a2tex install`/`revert` move
the two together. Versioning rules: [`CLAUDE.md`](../CLAUDE.md), "Changelogs and
versions". Newest first. The reasoning is in `textures/README.md`, in the
loading-screen section.

## 2.0.0 — 2026-09-27

### Changed
- `loading-panel.sh` keeps its layers in `$A2_DATA/textures/LOADING/` (`ai/`, `src/`)
  and reads the recipe from `textures/targets/LOADING/`, like every texture target.
  MAJOR: the layout changed.

Installs nothing different. Checked on the bench: `logo.SOD` and the LOADING tiles
installed from `$A2_DATA` are byte-identical to the ones installed today.

## 1.0.1 — 2026-09-27

### Changed
- `logo-sod.py` finds the game through `a2env.py` instead of a hard-coded path.

Installs nothing different.

## 1.0.0 — 2026-09-25

Baseline: the first versioned release.

- `SOD/logo.SOD`: the six loading-screen quads are widened to `panel=1400` units, so the
  screen fills 21:9. Only x coordinates change, and every vertex is checked against the
  stock grid first. Confirmed in game.
- `a2mod` files the SOD backup under the `models` layer.

## Before versioning

### 2026-09-25
- Moved from `tools/` to `models/` (`c3ae017`).

### 2026-09-24
- `logo-sod.py` and `loading-panel.sh` added. Confirmed in game (`f55e0d1`,
  `5ecfb50`).
