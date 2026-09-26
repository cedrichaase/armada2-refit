# Changelog — Armada II remastered

The project as a whole: the modpack's version, which layer versions it bundles, and
changes that belong to no single layer (`a2mod`, the repo layout, cross-layer
conventions). Each layer keeps its own `CHANGELOG.md` in its folder. Versioning rules:
[`CLAUDE.md`](CLAUDE.md), "Changelogs and versions". Newest first.

## 5.0.0 — 2026-09-26

Layers: platform 2.0.1.

### Removed
- The texture work — `a2tex`, `textures/`, `models/`, `archive/`, `promo/` — moved to
  the private repository `~/armada2-remastered-private`, because it cannot exist
  without the game's own textures. This repository's history was rewritten without
  it; changelog hashes were remapped (`publish/README.md`, "The split").

### Added
- `publish/`: the rule that nothing derived from the game's files is committed, and
  `publish/check.sh` to enforce it. `.gitignore` ignores every image, video and game
  format.

### Changed
- `platform/ab-shot.sh` captures to `platform/ab/` (`A2_AB_DIR` overrides), not
  `archive/ab/`.

`a2mod` still switches the textures and models layers in the game directory. Installs
nothing different into the game.

## 4.2.0 — 2026-09-26

Layers: hud 2.1.0, testbench 1.7.0.

### Fixed
- UI tiles meet edge to edge at every aspect: the thin grid lines through the mission
  briefing and the fine lines at HUD panel joins are gone, and so are the faint lines
  MSAA drew just outside UI sprites (see `hud/CHANGELOG.md`).

### Changed
- The test bench's `hud` scenario runs at the Federation briefing over unexplored space,
  where such lines show (see `testbench/CHANGELOG.md`).

Confirmed in game 2026-09-26.

## 4.1.1 — 2026-09-26

Layers: testbench 1.6.1.

### Fixed
- The test bench's pointer lands where it is sent in a mission, and its `hud` scenario
  checks the cursor again. It runs 5 games at once by default (see
  `testbench/CHANGELOG.md`).

Installs nothing into the game. Confirmed working 2026-09-26.

## 4.1.0 — 2026-09-26

Layers: testbench 1.6.0.

### Changed
- The test bench reads the screen with a scene-text model (PP-OCR) instead of
  tesseract (see `testbench/CHANGELOG.md`).

### Removed
- The bench's cursor check from 4.0.1: it never saw the cursor (`testbench/CHANGELOG.md`).

Installs nothing into the game. Confirmed working 2026-09-26.

## 4.0.1 — 2026-09-26

Layers: hud 2.0.1, testbench 1.5.1.

### Fixed
- The cursor is drawn at its stock proportions at any aspect (see `hud/CHANGELOG.md`),
  and the test bench's `hud` scenario now checks it.
- `a2mod` counts the game as running only if it runs from its own game directory, so
  the test bench can switch a clone to stock while the real game is open.

Confirmed in game 2026-09-26.

## 4.0.0 — 2026-09-26

Layers: hud 2.0.0, testbench 1.5.0.

### Changed
- The `font` layer is merged into `hud`, which is now `HUD.asi`: the HUD layout, font
  and cursors corrected at run time for any display mode (`1524bb9`; see `hud/CHANGELOG.md`).
  `a2mod` reports one `hud` layer and reads older manifests' `font` / `hud layout`
  entries as `hud`. MAJOR: a layer name is gone and `hud` now needs the ASI loader.
- The test bench installs `HUD.asi` once for every case (see `testbench/CHANGELOG.md`).

Confirmed in game 2026-09-26.

## 3.6.0 — 2026-09-26

Layers: testbench 1.4.0.

### Changed
- The test bench's HUD scenario accepts the condensed font within ±10% of stock (see
  `testbench/CHANGELOG.md`).

Installs nothing into the game. Confirmed working 2026-09-26.

## 3.5.0 — 2026-09-26

Layers: testbench 1.3.0.

### Changed
- The test bench measures the HUD and font against stock at 800x600, runs silently
  through a per-session null sink, and can drive the stock menus (see
  `testbench/CHANGELOG.md`).

Installs nothing into the game. Confirmed working 2026-09-26.

## 3.4.1 — 2026-09-26

Layers: testbench 1.2.1.

### Changed
- The test bench judges on Sonnet by default (see `testbench/CHANGELOG.md`).

Installs nothing into the game. Confirmed working 2026-09-26.

## 3.4.0 — 2026-09-26

Layers: testbench 1.2.0.

### Added
- `a2test run --jobs N`: cases run in parallel, three games at once by default (see
  `testbench/CHANGELOG.md`).

Installs nothing into the game. Confirmed working 2026-09-26.

## 3.3.0 — 2026-09-26

Layers: testbench 1.1.0.

### Added
- The test bench measures the HUD and font against stock 4:3 rather than remastered
  4:3 (see `testbench/CHANGELOG.md`).

Installs nothing into the game. Confirmed working 2026-09-26.

## 3.2.0 — 2026-09-25

Layers: testbench 1.0.0 (new).

### Added
- `./a2test`: the test bench. Headless runs of the game at any resolution on a clone of
  the install, driven by plain-text scenarios, with a screenshot-and-log report per run
  (see `testbench/CHANGELOG.md`).

Installs nothing into the game. Confirmed working 2026-09-26.

## 3.1.0 — 2026-09-25

Layers: menus 2.1.0.

### Added
- In a mission, Esc now also closes the Options menu and returns to the game
  (`EscapeReturns=`, see `menus/CHANGELOG.md`, `96f5ac1`).

Confirmed in game 2026-09-25.

## 3.0.2 — 2026-09-25

Layers: menus 2.0.2.

### Fixed
- In a mission, Esc opens Options again after it has been closed, and the keyboard
  works again after any in-game menu (see `menus/CHANGELOG.md`, `32656d7`).

Confirmed in game 2026-09-25.

## 3.0.1 — 2026-09-25

Layers: menus 2.0.1.

### Fixed
- The Admiral's Log: its score table no longer stays on screen after it closes, and
  its buttons are scaled with it rather than 1:1 in the corner (see
  `menus/CHANGELOG.md`, `c9f7f9f`).

Confirmed in game 2026-09-25.

## 3.0.0 — 2026-09-25

Layers: menus 2.0.0.

### Changed
- The menus plugin is renamed `MenuScale.asi` → `Menus.asi`, with its ini, log and
  plates folder (see `menus/CHANGELOG.md`).
- `a2mod`: the layer `menu scale` is now `menus`. It still recognises the old
  `MenuScale.*` files, so a stale copy is set aside in stock rather than left live. It
  reads old manifests' layer name as `menus`, and after restoring a pre-rename
  snapshot it says to run `menus/install.sh`.

Confirmed in game 2026-09-25.

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
