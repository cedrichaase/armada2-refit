# Changelog — Armada II remastered

The project as a whole: the modpack's version, which layer versions it bundles, and
changes that belong to no single layer (`a2mod`, the repo layout, cross-layer
conventions). Each layer keeps its own `CHANGELOG.md` in its folder. Versioning rules:
[`CLAUDE.md`](CLAUDE.md), "Changelogs and versions". Newest first.

## 3.0.0 — 2026-09-25

Layers: menus 2.0.0.

### Changed
- The menus plugin is renamed `MenuScale.asi` → `Menus.asi`, with its ini, log and
  plates folder (see `menus/CHANGELOG.md`).
- `a2mod`: the layer `menu scale` is now `menus`. It still recognises the old
  `MenuScale.*` files, so a stale copy is set aside in stock rather than left live. It
  reads old manifests' layer name as `menus`, and after restoring a pre-rename
  snapshot it says to run `menus/install.sh`.

Installed, not yet seen in game. Confirmed in game 2026-09-25.

## 2.0.0 — 2026-09-25

Layers: platform 2.0.0.

### Removed
- `platform/virtual-desktop.py`. The Wine virtual desktop is superseded by `Embed=1`
  (see `platform/CHANGELOG.md`).

Installed state unchanged; nothing to see in game.

## 1.0.0 — 2026-09-25

Baseline: the first versioned release. Every layer starts at 1.0.0. The project
changelog and the ten layer changelogs are introduced here, each backfilled from git
history.

| Layer | Version | Changelog |
|---|---|---|
| textures | 1.0.0 | [`textures/CHANGELOG.md`](textures/CHANGELOG.md) |
| models | 1.0.0 | [`models/CHANGELOG.md`](models/CHANGELOG.md) |
| hud | 1.0.0 | [`hud/CHANGELOG.md`](hud/CHANGELOG.md) |
| font | 1.0.0 | [`font/CHANGELOG.md`](font/CHANGELOG.md) |
| menus | 1.0.0 | [`menus/CHANGELOG.md`](menus/CHANGELOG.md) |
| msaa | 1.0.0 | [`msaa/CHANGELOG.md`](msaa/CHANGELOG.md) |
| cutscenes | 1.0.0 | [`cutscenes/CHANGELOG.md`](cutscenes/CHANGELOG.md) |
| postfx | 1.0.0 | [`postfx/CHANGELOG.md`](postfx/CHANGELOG.md) |
| platform | 1.0.0 | [`platform/CHANGELOG.md`](platform/CHANGELOG.md) |
| gameplay | 1.0.0 | [`gameplay/CHANGELOG.md`](gameplay/CHANGELOG.md) |

Every layer that `a2mod` switches is installed and confirmed in game at this release.

## Before versioning

Project-level history from git. Layer work is in each layer's changelog.

### 2026-09-25
- `promo/` holds the before/after footage, kept out of git (`a4adb73`).
  `archive/intro-upscaler-comparison/` committed (`927f993`).
- Restructured into one folder per `a2mod` layer. `SETUP.md` was split into the layer
  READMEs, and the old root README became `textures/README.md` (`c3ae017`, `9d6bf1b`).
- The font and draw-distance branches merged into the new layout (`36ce1a7`,
  `5d9454a`).

### 2026-09-24
- `./a2mod`: switches the whole game between stock and remastered by snapshot, with
  every file hash-checked (`4393a1b`). The `models` and `cutscenes` layers were added
  to it (`1c178f9`).

### 2026-09-20
- Project started as a nebula and skybox texture pipeline (`c0d3f9b`).
