# Changelog — postfx

Two `a2mod` layers share this folder: **renderer** (`dxvk.conf`, written by
`renderer-config.sh`) and **bloom** (vkBasalt, built by `vkbasalt/build.sh` and
configured by `postfx.py`). They are versioned together as one folder. Versioning rules:
[`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. The tables behind
the numbers are in [`README.md`](README.md).

## 1.0.1 — 2026-09-27

### Changed
- `renderer-config.sh` find the game through `a2env.sh` / `a2env.py` (`A2_GAME`, then
  `~/.config/armada2-remastered.conf`, then Heroic's default) instead of a hard-coded
  path.

Installs nothing different; nothing to see in game.

## 1.0.0 — 2026-09-25

Baseline: the first versioned release. Both layers are confirmed in game.

- **renderer:** `dxvk.conf` at stage 3: 16x anisotropic filtering, a −0.5 mip LOD bias
  (`--bias`) and `seamlessCubes`. Every key is checked against the `d3d9.dll` that
  actually loads.
- **bloom:** vkBasalt is built per-user and 32-bit, with MagicBloom (no adaptation, no
  lens dirt) at threshold 6 and intensity 0.08, accepted by the user. `postfx.py`
  refuses any effect that `fxcheck` cannot compile. Home toggles it in game.

## Before versioning

### 2026-09-25
- Moved to `postfx/`. The generated-file markers keep their old
  `tools/renderer-config.sh` spelling, because an installed `dxvk.conf` is matched by
  that marker (`7daf6d8`, `ad7bda9`).

### 2026-09-23
- Bloom through vkBasalt (`706c4df`). Accepted in game at intensity 0.08 (`ddd6d40`).

### 2026-09-22
- `renderer-config.sh`: stage 1, 16x AF (`6ff0235`), and stage 2, the LOD bias
  (`a9cd708`). Neither could take effect until DXVK was actually in the chain (see
  `platform/CHANGELOG.md`). Stage 3, `seamlessCubes`, followed, and all three were
  confirmed in game (`b5510bb`, `255e995`).
