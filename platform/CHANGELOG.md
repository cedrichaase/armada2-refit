# Changelog — platform

What `a2mod` never switches: DXVK in the d3d8 chain, the ASI loader, the widescreen
patch, and the Heroic/Proton setup, with the tools that manage them (`d3d8-chain.py`,
`dxvk-logging.py`, `ab-shot.sh`). Versioning rules:
[`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. Details are in
[`README.md`](README.md).

## 3.4.0 — 2026-10-10

### Added
- `heroic-umu.py --on/--off/--status`: `UMU_HTTP_TIMEOUT=3` and `UMU_HTTP_RETRIES=0` in
  the game's Heroic config, so a slow Steam server holds each of Heroic's three umu runs
  per launch up for at most 3 s instead of up to ~15. Not run by `./install`.

On the bench, a `createprefix` with the Steam server silent took 9.9 s instead of 19.0 s,
and the game started and reached a map with both set. Installed, not yet seen in game.

## 3.3.0 — 2026-10-08

### Added
- `vendor/dxvk-3.1.1/`: DXVK's x32 `d3d9.dll` from the upstream release (zlib/libpng),
  which the release zip puts behind d3d8to9; `./install` still takes Proton's own.

Confirmed in game 2026-10-09.

## 3.2.0 — 2026-10-05

### Added
- `d3d8-chain.py --upgrade`, which `./install` now runs first: a game on the DXVK chain
  (DXVK's d3d8 on DXVK's d3d9) moves to the d3d8to9 chain by swapping the game
  directory's `d3d8.dll` alone, so `Lighting.asi`'s `Shaders` work after a plain
  install. Heroic's config is never touched; any other chain is left as it is and named.
  `--use dxvk` goes back (a2f2ab6).

Exercised on a scratch game directory (DXVK chain, again, GOG's chain); `./a2test run
no-assets` passes; on the bench a clone put back on DXVK's d3d8 came up on d3d8to9 after
`./install`, with `Lighting.log` reading `shaders: on`. Installed, not yet seen in game.

## 3.1.0 — 2026-10-05

### Added
- `vendor/d3d8to9-1.16.0/`: crosire's d3d8to9 release `d3d8.dll` (BSD-2-Clause, 7d35a9f),
  unmodified, with its licence and a `SOURCE.txt` of hashes.
- `d3d8-chain.py --use d3d8to9`: that translator on DXVK's d3d9, with the `d3d9`
  override and `autoInstallDxvk` off as for `--use dxvk`. `--status` names it, and
  `--use` refuses a vendored file that does not match its hash. Under it, plugins can
  reach the Direct3D 9 device (`D3D9.md`). The default stays DXVK's d3d8.
- `d3d9/d3d9dev.h` and `d3d9/hlsl.sh`: the shared piece for plugins that draw through
  that device. The header gets the device and manages shader objects and the state
  around a draw; the script compiles HLSL with vkd3d into an embedded header.

Installs nothing by itself; `--use d3d8to9` confirmed in game 2026-10-05.

## 3.0.0 — 2026-09-28

### Added
- `vendor/STA2WidescreenPatch-1.0/`: `STA2WidescreenPatch.asi` (Ligushka, MIT) and
  the Ultimate ASI Loader 4.68 as `winmm.dll` (ThirteenAG, MIT), unmodified from the
  v1.0 GitHub release, with both licences and a `SOURCE.txt` of hashes. The release
  zip's installers install them from here. The first binaries in the repository;
  `publish/check.sh` accepts them because of their licence file and `SOURCE.txt`.

### Changed
- Patch Project 1.2.5 is no longer part of the platform. `d3d8-chain.py` knows GOG's
  `d3d8.dll` by hash and keeps it as `d3d8.dll.gog-backup` the first time `--use`
  replaces it; `--revert` returns to the GOG release as shipped (its d3d8to9 in the game
  directory, no game-directory `d3d9.dll`, no `d3d9` override, `autoInstallDxvk` on)
  instead of to Patch Project's proxy, and `--use wine` empties the game-directory slot.
  `--use` refuses to replace a `d3d8.dll` it cannot identify.

### Removed
- `d3d8.dll.proxy-backup`: no longer written, read or restored. One left over from
  earlier versions is left where it is.

`d3d8-chain.py` is exercised on a scratch game directory; the vendored files are the
ones on the development install, byte for byte. Installed, not yet seen in game.

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
