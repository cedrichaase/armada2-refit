# Changelog — models

The game's 3D geometry: the widened mission loading screen (`logo-sod.py`,
`loading-panel.sh`), which ships in lockstep with `textures/targets/LOADING`, and
`Planets.asi`, the planets' tessellation, the dilithium moons (`moon-sod.py`) and the
selection bubble (`select-sod.py`). Versioning rules: [`CLAUDE.md`](../CLAUDE.md),
"Changelogs and versions". Newest first. The reasoning is in [`README.md`](README.md).

## 3.3.0 — 2026-10-05

### Added
- `select-sod.py` (`--install`, `--revert`, `--status`, `--split`, `--out`) rounds the
  selection bubble, `SOD/select.sod`: each of its 320 triangles becomes 4 (`--split 2`)
  with the new vertices on the sphere. `install.sh` runs it and `install.sh --remove`
  reverts it; the backup is `.a2neb-backup` (433d846).

Installed, not yet seen in game.

## 3.2.0 — 2026-10-04

### Added
- `hull-bump.py` (`--install`, `--revert`, `--status`, `--manifest`): the 36 Federation
  SODs in `hull-bump.sha256` get the Borg's bump-mapped material spelling, so the engine
  lights those hulls per pixel through its dot3 path. All of them name one flat height map,
  `Textures/RGB/a2flatbump.tga`, which this layer adds and `a2mod` switches with it. It
  also corrects the dot3 shader, `Shaders/dot3_directional.nvv`, to take the normal from
  the vertex normal rather than S x T (backed up, filed under `models` by `a2mod`).
  Not run by `./install`; a hull it patches leaves the `lighting` layer's GPU path.

Confirmed in game 2026-10-04.

## 3.1.0 — 2026-10-04

### Added
- `moon-sod.py` smooths the dilithium moons (`SOD/Mdmoon`, `Mdmoon2`, `Mdmoon3`,
  `Mmooninf`): the rock becomes curved patches through its stock vertices at
  `--split 2`, keeping its shape; the blended glow shell stays stock unless `--glow`.
  `install.sh` runs it, `install.sh --remove` reverts it; backups are `.a2neb-backup` (59e503a).

Confirmed in game 2026-10-04.

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
