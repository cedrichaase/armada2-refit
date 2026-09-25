# Changelog — postfx

Two `a2mod` layers share this folder: **renderer** (`dxvk.conf`, written by
`renderer-config.sh`) and **bloom** (vkBasalt, built by `vkbasalt/build.sh` and
configured by `postfx.py`). They are versioned together as one folder. Versioning rules:
[`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. The tables behind
the numbers are in [`README.md`](README.md).

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
  that marker (`c3ae017`, `e7863b9`).

### 2026-09-23
- Bloom through vkBasalt (`82b5f9b`). Accepted in game at intensity 0.08 (`f0443d5`).

### 2026-09-22
- `renderer-config.sh`: stage 1, 16x AF (`8f289ca`), and stage 2, the LOD bias
  (`d593388`). Neither could take effect until DXVK was actually in the chain (see
  `platform/CHANGELOG.md`). Stage 3, `seamlessCubes`, followed, and all three were
  confirmed in game (`c89c074`, `518f69c`).
