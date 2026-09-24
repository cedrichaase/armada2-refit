# Changelog — platform

What `a2mod` never switches: DXVK in the d3d8 chain, the ASI loader, the widescreen
patch, and the Heroic/Proton setup, with the tools that manage them (`d3d8-chain.py`,
`dxvk-logging.py`, `virtual-desktop.py`, `ab-shot.sh`). Versioning rules:
[`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. Details are in
[`README.md`](README.md).

## 1.0.0 — 2026-09-25

Baseline: the first versioned release.

- The render chain is Patch Project proxy → DXVK `d3d8.dll` → DXVK `d3d9.dll` → Vulkan.
  Both DXVK DLLs sit in the **game directory**, with `d3d9=n,b` in the overrides,
  `autoInstallDxvk` false and the Wine virtual desktop off. Confirmed in game at
  3440x1440.
- `d3d8-chain.py --status` identifies every link by hash. `--use` and `--revert` own
  the `d3d9=n,b` override.

## Before versioning

### 2026-09-25
- Moved to `platform/`. The rest of `SETUP.md` became `README.md` (`c3ae017`,
  `9d6bf1b`).

### 2026-09-22
- Found that `syswow64/d3d8.dll` was Wine's builtin, not DXVK, so the game had been
  rendering through wined3d/OpenGL the whole time (`afb8cd0`).
- `d3d8-chain.py` (`afb8cd0`, `4114699`). DXVK moved into the game directory, because
  the prefix is not durable (`b287056`).
- `virtual-desktop.py` turned the Wine virtual desktop off. With it on, exclusive
  fullscreen failed under DXVK and the game dropped to 640x480 (`f8101d6`, `b848258`).
- DXVK renders the game. Confirmed in game (`da1d15f`).
- `dxvk-logging.py` and `ab-shot.sh`: separate "the setting was ignored" from "the
  setting is subtle" (`5e428ac`, `539da5a`, `e232fb4`).
