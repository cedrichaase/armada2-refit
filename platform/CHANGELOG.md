# Changelog — platform

What `a2mod` never switches: DXVK in the d3d8 chain, the ASI loader, the widescreen
patch, and the Heroic/Proton setup, with the tools that manage them (`d3d8-chain.py`,
`dxvk-logging.py`, `ab-shot.sh`). Versioning rules:
[`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. Details are in
[`README.md`](README.md).

## 2.1.0 — 2026-09-28

### Added
- `vendor/STA2WidescreenPatch-1.0/`: `STA2WidescreenPatch.asi` (Ligushka, MIT) and
  the Ultimate ASI Loader 4.68 as `winmm.dll` (ThirteenAG, MIT), unmodified from the
  v1.0 GitHub release, with both licences and a `SOURCE.txt` of hashes. The release
  zip's installers install them from here. The first binaries in the repository;
  `publish/check.sh` accepts them because of their licence file and `SOURCE.txt`.

The same files as on the development install, byte for byte; installing them is new
only for the release zip, which is not yet seen in game.

## 2.0.3 — 2026-09-27

### Changed
- `README.md` no longer points at `gameplay/`, which is gone.

Installs nothing different; nothing to see in game.

## 2.0.2 — 2026-09-27

### Changed
- `d3d8-chain.py` (and the prefix and Proton) and `dxvk-logging.py` find the game through `a2env.sh` / `a2env.py` (`A2_GAME`, then
  `~/.config/armada2-refit.conf`, then Heroic's default) instead of a hard-coded
  path.

Installs nothing different; nothing to see in game.

## 2.0.1 — 2026-09-26

### Changed
- `ab-shot.sh` writes its captures to `platform/ab/` (`A2_AB_DIR` overrides) instead
  of `archive/ab/`, which left this repository with the texture work.

Installed state unchanged; nothing to see in game.

## 2.0.0 — 2026-09-25

### Removed
- `virtual-desktop.py`. The Wine virtual desktop is superseded by `MenuScale.asi`'s
  `Embed=1` and must stay off for DXVK, so a tool that could turn it back on was a
  hazard. `README.md` ("Hyprland / window management") has the check that replaces
  `--status`. MAJOR because a tool is removed. The installed game is unaffected: this
  prefix's virtual desktop was already off, and nothing else in it changes.

Installed state unchanged; nothing to see in game.

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
- Moved to `platform/`. The rest of `SETUP.md` became `README.md` (`7daf6d8`,
  `a38d137`).

### 2026-09-22
- Found that `syswow64/d3d8.dll` was Wine's builtin, not DXVK, so the game had been
  rendering through wined3d/OpenGL the whole time (`70ca2ec`).
- `d3d8-chain.py` (`70ca2ec`, `859b0ee`). DXVK moved into the game directory, because
  the prefix is not durable (`d79a7f3`).
- `virtual-desktop.py` turned the Wine virtual desktop off. With it on, exclusive
  fullscreen failed under DXVK and the game dropped to 640x480 (`d9f7c58`, `9465bd1`).
- DXVK renders the game. Confirmed in game (`1021bbb`).
- `dxvk-logging.py` and `ab-shot.sh`: separate "the setting was ignored" from "the
  setting is subtle" (`19c2c68`, `2578807`, `fb6c388`).
