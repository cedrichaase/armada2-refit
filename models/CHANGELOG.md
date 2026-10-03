# Changelog — models

The game's 3D geometry: the widened mission loading screen (`logo-sod.py`,
`loading-panel.sh`), which ships in lockstep with `textures/targets/LOADING`, and
`Planets.asi`, the planets' tessellation. Versioning rules: [`CLAUDE.md`](../CLAUDE.md),
"Changelogs and versions". Newest first. The reasoning is in [`README.md`](README.md).

## 3.0.0 — 2026-10-03

### Added
- `Planets.asi` (`planets.c`, `build.sh`, `Planets.ini`): the engine tessellates planets,
  ground and cloud shell, finely enough for a modern resolution. `Detail=` divides its
  facet tolerance (default 8; 1 = stock). Installed by `install.sh`, which `./install`
  now runs. MAJOR: the layer now needs the ASI loader from `platform/` (88887e3).
- `a2mod` switches `Planets.asi`/`.ini` as part of this layer.

Confirmed in game 2026-10-03.

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
