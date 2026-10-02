# Changelog — Armada II Refit

The project as a whole: the modpack's version, which layer versions it bundles, and
changes that belong to no single layer (`a2mod`, the repo layout, cross-layer
conventions). Each layer keeps its own `CHANGELOG.md` in its folder. Versioning rules:
[`CLAUDE.md`](CLAUDE.md), "Changelogs and versions". Newest first.

## 8.1.0 — 2026-10-02

Layers: testbench 2.1.0.

### Added
- `a2test session start --stock-shell embed`, and `drive --design` works again
  (testbench 2.1.0).

Installs nothing into the game.

## 8.0.0 — 2026-09-28

Layers: platform 3.0.0, postfx 1.1.0.

### Added
- The release zip, now `armada2-refit-<version>.zip`, carries the renderer and bloom too:
  `game/` holds the four plugins, their `.ini` files and `dxvk.conf` (stage 3), and
  `bloom/` holds bloom for vkBasalt and as a ReShade preset (`postfx.py --export`).
- Installers in the zip, from `publish/installer/`: `install.sh` (Linux, Wine/Proton)
  and `install.ps1` / `install.bat` (Windows). They take no options but `--uninstall`.
  They find the game, back up the stock `binkw32.dll`, install MSAA only beside DXVK's
  `d3d8.dll` (told by content), and leave a foreign `dxvk.conf` alone. They set up
  bloom when vkBasalt or ReShade is installed, fetching the shaders pinned and
  hash-checked.
- The installers also install the prerequisites when missing (`prereqs.txt`):
  STA2WidescreenPatch 1.0 with the Ultimate ASI Loader 4.68, bundled because both are
  MIT. They never overwrite a file, and `--uninstall` removes only what they added and
  nobody changed since. `CREDITS.txt` in the zip names each author, licence and source.
- Third-party binaries whose licence expressly allows it are vendored in
  `platform/vendor/` with their licences and hashes, and `publish/check.sh` accepts a
  binary only there, only with a licence file and only matching `SOURCE.txt`.
- CI runs both installers against a mock game (`test.sh`, and `test.ps1` on a Windows
  runner) before anything is released.

### Removed
- Patch Project 1.2.5 is no longer a requirement, anywhere: nothing here used it, and
  with DXVK in the `d3d8.dll` slot it was never loaded. `platform/d3d8-chain.py
  --revert` now returns to GOG's own `d3d8.dll` rather than to its proxy (platform
  3.0.0).

Installed, not yet seen in game.

## 7.1.0 — 2026-09-28

### Added
- `publish/package.sh`: builds `HUD.asi`, `Menus.asi`, `MSAA.asi` and `binkw32.dll` into
  `armada2-refit-<version>-plugins.zip`, with their `.ini` files, `README.txt`,
  `LICENSE` and `SHA256SUMS`.
- CI (`.github/workflows/ci.yml`): `publish/check.sh` on every new commit, the plugin
  zip as a workflow artifact on every push and pull request, and a `vX.Y.Z` GitHub
  Release when `main` reaches a root version not yet released.

Installs nothing new; no game-side change to confirm.

## 7.0.0 — 2026-09-28

Layers: testbench 2.0.0.

### Changed
- The project is renamed **Armada II Refit**, from Armada II Remastered. `a2env.sh` and
  `a2env.py` default to `~/.config/armada2-refit.conf` and `~/.local/share/armada2-refit`;
  the old names are no longer read.
- `./a2mod refit` replaces `./a2mod remastered`, and the snapshot folder is
  `$GAME/.a2mod/refit/`; a `state.json` that says `remastered` has to be edited to `refit`.

Installs nothing new; no game-side change to confirm.

## 6.5.0 — 2026-09-27

Layers: hud 2.2.0, testbench 1.12.0.

### Fixed
- The HUD follows a display mode changed in the middle of a mission, instead of
  keeping the old mode's layout until the next one (`9873980`; see `hud/CHANGELOG.md`).

### Added
- `testbench/scenarios/hud-mode-switch.md`, its regression test.

Confirmed in game 2026-09-27.

## 6.4.0 — 2026-09-27

Layers: menus 4.3.0, testbench 1.11.0.

### Added
- Game Setup's option labels no longer run over the minimap's frame: `[Labels]` in
  `Menus.ini` keeps them inside the art (see `menus/CHANGELOG.md`), with a bench scenario
  that pins it (`testbench/CHANGELOG.md`). 9838ca6.

Confirmed in game 2026-09-27.

## 6.3.0 — 2026-09-27

Layers: menus 4.2.0, testbench 1.10.0.

### Added
- The menus' edit boxes (the multiplayer name, Save Game's name) are scaled with their
  dialog, font included (see `menus/CHANGELOG.md`), with a bench scenario that pins it
  (`testbench/CHANGELOG.md`).

Confirmed in game 2026-09-27.

## 6.2.2 — 2026-09-27

Layers: cutscenes 2.0.1.

### Fixed
- The cutscene installer no longer refuses because some other copy of the game is
  running, which failed `a2test --install` while the real game was open (see
  `cutscenes/CHANGELOG.md`).

Installs nothing different into the game.

## 6.2.1 — 2026-09-27

Layers: none (publish/ is versioned by the root).

### Fixed
- `publish/check.sh` refuses `*.map`, and `.gitignore` ignores it: `armada2.map` is text,
  so the binary test let it through. The rules for engine findings are in
  `publish/README.md`, "Engine findings", and `CLAUDE.md`, hard rule 9.

Installs nothing different into the game.

## 6.2.0 — 2026-09-27

Layers: textures 2.1.0.

### Added
- Texture packs: `a2tex pack` and `a2tex install --pack`, and `textures/PACKS.md`, the
  short path from extracting stock art to sharing a pack (see `textures/CHANGELOG.md`).

Installs nothing different into the game.

## 6.1.0 — 2026-09-27

Layers: textures 2.0.1, models 2.0.0, cutscenes 2.0.0 (joined), menus 4.1.0, testbench 1.9.0.

### Added
- The texture pipeline (`./a2tex`, `textures/`), the loading-screen model (`models/`),
  the cutscene proxy and movie pipeline (`cutscenes/`) and the menu backdrop pipeline
  (`menus/backdrop.sh`, `install-plates.sh`, the `backdrops/*.conf` recipes), from the
  private repository: code, recipes and docs, in one commit without their history
  (`publish/README.md`, "The merge"). Their assets stay in `A2_DATA`.
- `./install` installs every layer. The asset layers install what is built in
  `A2_DATA`; with nothing built they say so and the game keeps its own art. The test
  bench's `no-assets` scenario checks exactly that, and passes.

### Changed
- `publish/check.sh` no longer refuses the texture, cutscene and backdrop paths; it
  refuses any binary, any game format, and anything inside a data directory a
  checkout from before `A2_DATA` may still hold (also in `.gitignore`).

Installs nothing different into the game.

## 6.0.0 — 2026-09-27

Layers: platform 2.0.3, testbench 1.8.1; gameplay removed.

### Removed
- The `gameplay` layer: `scrollspeed.py` and its notes on map scrolling and the
  cutscene draw distance. The scroll speeds are set in game now that the menus work,
  and the draw-distance notes changed nothing. MAJOR: a layer is gone. It never touched
  what `a2mod` switches; whatever it last wrote to `RTS_CFG.h` and `ARMADA.PRF` stays in
  the game directory beside its `.a2neb-backup`.

### Added
- `A2_DATA` in `a2env.sh` / `a2env.py`: where assets live (default
  `~/.local/share/armada2-refit`, `%LOCALAPPDATA%` on Windows), never a working
  tree. The private repository keeps every stock copy, paid layer and build there.

Installs nothing different into the game.

## 5.1.0 — 2026-09-27

Layers: hud 2.1.1, menus 4.0.0, msaa 1.0.1, postfx 1.0.1, platform 2.0.2, gameplay 1.0.1,
testbench 1.8.0.

### Added
- `a2env.sh` / `a2env.py`: one place that says where the game, its prefix and Proton
  are — environment, then `~/.config/armada2-refit.conf`, then Heroic's default. No
  file names a user's home directory any more.
- `./install`: every layer this repository owns, into `$A2_GAME`.
- `a2test session start --install PATH` / `a2test run --install PATH`, repeatable: public
  and private checkouts stacked into the clone (see `testbench/CHANGELOG.md`).

### Changed
- `A2_GAME` everywhere; `A2_GAME_DIR` and `A2_DIR` are still read. The backdrop plates
  are installed by the private repository, not `menus/install.sh`.

Installs nothing different into the game. Checked on the bench: public and private
worktrees stacked into a clone reproduced the installed game (1677 textures and both
plates byte-identical; plugins differing only in their build timestamp), launched to the
main menu at 16:9, and left the real install untouched.

## 5.0.0 — 2026-09-26

Layers: platform 2.0.1, menus 3.0.0.

### Removed
- The texture work — `a2tex`, `textures/`, `models/`, `archive/`, `promo/` — moved to
  the private repository `~/armada2-refit-private`, because it cannot exist
  without the game's own textures. So did `cutscenes/` (binkproxy with the upscaled
  intro) and the menu backdrop pipeline (`menus/backdrop.sh`, see
  `menus/CHANGELOG.md`). This repository's history was rewritten without them;
  changelog hashes were remapped (`publish/README.md`, "The split").

### Added
- `publish/`: the rule that nothing derived from the game's files is committed, and
  `publish/check.sh` to enforce it. `.gitignore` ignores every image, video and game
  format.

### Changed
- `platform/ab-shot.sh` captures to `platform/ab/` (`A2_AB_DIR` overrides), not
  `archive/ab/`.

`a2mod` still switches the textures, models and cutscenes layers in the game directory.
Installs nothing different into the game.

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
  and cursors corrected at run time for any display mode (`f5f6721`; see `hud/CHANGELOG.md`).
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
- The test bench measures the HUD and font against stock 4:3 rather than refit
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
  (`EscapeReturns=`, see `menus/CHANGELOG.md`, `5f60d5b`).

Confirmed in game 2026-09-25.

## 3.0.2 — 2026-09-25

Layers: menus 2.0.2.

### Fixed
- In a mission, Esc opens Options again after it has been closed, and the keyboard
  works again after any in-game menu (see `menus/CHANGELOG.md`, `c73a36b`).

Confirmed in game 2026-09-25.

## 3.0.1 — 2026-09-25

Layers: menus 2.0.1.

### Fixed
- The Admiral's Log: its score table no longer stays on screen after it closes, and
  its buttons are scaled with it rather than 1:1 in the corner (see
  `menus/CHANGELOG.md`, `91ab056`).

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
- `promo/` holds the before/after footage, kept out of git (`4ea05a1`).
  `archive/intro-upscaler-comparison/` committed (`927f993`).
- Restructured into one folder per `a2mod` layer. `SETUP.md` was split into the layer
  READMEs, and the old root README became `textures/README.md` (`7daf6d8`, `a38d137`).
- The font and draw-distance branches merged into the new layout (`15d55b4`,
  `e648a3a`).

### 2026-09-24
- `./a2mod`: switches the whole game between stock and refit by snapshot, with
  every file hash-checked (`53a5d0f`). The `models` and `cutscenes` layers were added
  to it (`704388e`).

### 2026-09-20
- Project started as a nebula and skybox texture pipeline (`2dd4ee8`).
