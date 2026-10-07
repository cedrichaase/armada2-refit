# Changelog — Armada II Refit

The project as a whole: the modpack's version, which layer versions it bundles, and
changes that belong to no single layer (`a2mod`, the repo layout, cross-layer
conventions). Each layer keeps its own `CHANGELOG.md` in its folder. Versioning rules:
[`CLAUDE.md`](CLAUDE.md), "Changelogs and versions". Newest first.

## 11.6.0 — 2026-10-07

Layers: qol 1.6.0, online 0.6.0.

### Added
- The online handshake for QOLRules: pay-on-queue runs in an *Internet – Online* game when
  every player has it, and stands down otherwise.

Installed, not yet seen in game.

## 11.5.0 — 2026-10-07

Layers: qol 1.5.0.

### Added
- `QOLRules.asi`: pay when queuing, refund on cancel (QOL-7), single player and skirmish
  only. `a2mod` and the bench's log collection know the new files.

Installed, not yet seen in game.

## 11.4.0 — 2026-10-07

Layers: models 4.2.0.

### Added
- Every faction's ships and stations drawn round where they are round: 176 hull models
  (`hull-sod.py`, models 4.2.0).

Confirmed in game 2026-10-07.

## 11.3.0 — 2026-10-07

Layers: models 4.1.0.

### Added
- Federation saucers, bridge modules and aeroshells drawn round rather than as rings of
  flat facets (`hull-sod.py`, models 4.1.0): four hulls so far.

Confirmed in game 2026-10-07.

## 11.2.0 — 2026-10-07

Layers: qol 1.4.0.

### Added
- The view reaches across the whole map: the far plane and the distance past which
  ships, stations, planets, asteroids and nebulae fade out are raised in memory
  (`ViewDistance=`, qol 1.4.0).

Confirmed in game 2026-10-07.

## 11.1.0 — 2026-10-07

Layers: lighting 1.17.0.

### Changed
- A ship close to the camera fades only once the camera's near plane cuts into it, not
  once it fills half the view (`NearFade`, `NearFadeDepth`, `NearFadeMin`, lighting 1.17.0).

### Fixed
- Ships close to the camera keep the new lighting instead of falling back to stock's
  while the engine fades them; a faded ship is drawn blended, as stock draws it
  (lighting 1.17.0).

Confirmed in game 2026-10-07.

## 11.0.0 — 2026-10-07

Layers: lighting 1.16.0, models 4.0.0.

### Added
- The Borg have a lighting profile of their own: dark plating with a faint, tight
  highlight, a sickly green rim and their night lights carrying the look (`BorgAmbient`,
  `BorgSpecular`, `BorgRimLight`, `BorgSun`, ..., lighting 1.16.0).

### Removed
- `models/hull-bump.py` and its flat map `a2flatbump.tga` (models 4.0.0); `a2mod` and
  `textures/tools/inventory.py` no longer list the map.

Confirmed in game 2026-10-07.

## 10.6.0 — 2026-10-07

Layers: lighting 1.15.0.

### Added
- Shadows from the Key: ships and stations shade themselves and each other, and planets
  shadow what is behind them (`Shadows`, `PlanetShadows`, lighting 1.15.0).

Confirmed in game 2026-10-07.

## 10.5.0 — 2026-10-07

Layers: lighting 1.14.0.

### Added
- Phaser fire lights the firing ship with a hot spot where the beam leaves its hull,
  and the target where it strikes, in the beam's colour (`Phasers`, `PhaserImpact`,
  lighting 1.14.0).

Confirmed in game 2026-10-07.

## 10.4.0 — 2026-10-07

Layers: lighting 1.13.0.

### Changed
- City lights on a planet's night side are webs of streets, roads and single lights
  instead of flat cream blotches (`CityLights`, lighting 1.13.0).

Confirmed in game 2026-10-07.

## 10.3.0 — 2026-10-07

Layers: qol 1.3.0, testbench 3.2.0.

### Added
- A move ordered on the map past `WarpDistance=` (default 1100) goes to warp, as one on
  the minimap does (qol 1.3.0); the `warp` scene and `qol-warp` scenario check it
  (testbench 3.2.0).

Confirmed in game 2026-10-07.

## 10.2.0 — 2026-10-07

Layers: lighting 1.12.0, testbench 3.1.0.

### Added
- Torpedo and pulse lights take the colour of the projectile's own sprite instead of
  the ODF's cyan or green (`OrdnanceColours=`, lighting 1.12.0); the bench's
  `factions` scene shows one weapon of each playable faction (testbench 3.1.0).

Confirmed in game 2026-10-07.

## 10.1.0 — 2026-10-07

Layers: lighting 1.11.0.

### Added
- Borg ships, and hulls `models/hull-bump.py` patched, are lit in `Lighting.asi`'s hull
  shaders under d3d8to9 (`BumpShaders=1`, lighting 1.11.0): their bump maps kept, with
  the ambient, light sources, specular, rim and night lights other hulls get.

Confirmed in game 2026-10-07.

## 10.0.0 — 2026-10-06

Layers: testbench 3.0.0.

### Changed
- `a2test session stop` requires a session ID, and `a2test drive` takes `--session ID`
  and refuses to pick among several active sessions (testbench 3.0.0). `CLAUDE.md`: an
  agent keeps the IDs of the sessions it starts and stops only those.

Installs nothing; no game-side change to confirm.

## 9.17.0 — 2026-10-05

Layers: models 3.3.0.

### Added
- The selection bubble around a selected ship is round instead of a visible polygon
  (`models/select-sod.py`, models 3.3.0).

Confirmed in game 2026-10-05.

## 9.16.0 — 2026-10-05

Layers: lighting 1.10.0, postfx 1.2.0.

### Added
- More dynamic range on lit hulls and planets (lighting 1.10.0): the light is no longer
  clamped at 1, highlights roll off towards white (`HighlightKnee`), and night lights and
  specular are brighter (`SelfIllumination=1.8`, `Specular=0.7`, `HullSun=1.1`), so
  bloom has something to take.

### Changed
- Bloom is on from launch with no toggle key (postfx 1.2.0); `a2mod status` no longer
  mentions Home, and the release package's notes and installer say so.

Confirmed in game 2026-10-05.

## 9.15.0 — 2026-10-05

Layers: qol 1.2.0, testbench 2.14.0.

### Added
- Stations in control groups: several of one kind select and group together, recall
  and centre like ships, and share one build menu that sends each order to one of
  them (`StationGroups=`, qol 1.2.0).
- Scene.asi selects and reads the selection, and scenarios can drive it (testbench
  2.14.0).

### Fixed
- `Ctrl+number` replaces a group that held stations, label included, and ships built at
  a station in a group no longer join it (qol 1.2.0).

## 9.14.0 — 2026-10-05

Layers: lighting 1.9.0.

### Added
- Planets on the GPU (lighting 1.9.0): `PlanetShaders=1` draws planets and their cloud
  shells in shaders under d3d8to9, lit per pixel, with a terminator, the atmosphere at
  the limb, water glint and city lights at night. Phase 3 of `platform/D3D9.md`.

Confirmed in game 2026-10-05.

## 9.13.1 — 2026-10-05

### Changed
- CI's release job keeps only the newest three GitHub Releases, deleting older ones and
  keeping their tags.

## 9.13.0 — 2026-10-05

Layers: platform 3.2.0.

### Changed
- `./install` sets up the d3d8to9 chain for a game on the DXVK chain, so the shader
  lighting is on after a plain install (platform 3.2.0).
- The release zip ships `Lighting.asi` and `Lighting.ini`, installed everywhere and taken
  out by `--uninstall`; it lights per pixel only behind d3d8to9, which the zip does not
  install, and per vertex otherwise.

### Fixed
- The release installers put `MSAA.asi` in on the d3d8to9 chain too (crosire's d3d8to9,
  known by hash, in front of DXVK's `d3d9.dll`); they had skipped it whenever the
  `d3d8.dll` was not DXVK's.
- CI checks a new branch's commits from where it leaves `main`, not all of history,
  which failed every new branch on 24cbcd0 (build output, taken out in da4a4a6). The
  installer tests cover `Lighting.asi` and the d3d8to9 chain.

## 9.12.0 — 2026-10-05

Layers: lighting 1.8.0.

### Added
- Specular highlights and a rim light on the shader-lit hulls (lighting 1.8.0).

## 9.11.0 — 2026-10-05

Layers: platform 3.1.0, testbench 2.13.0, lighting 1.7.0.

### Added
- `testbench/d3d9probe/`, a bench tool that checks whether plugins can reach Direct3D 9
  behind the game's d3d8 (testbench 2.13.0). The spike and the plan it supports are in
  `platform/D3D9.md`.
- Phase 0 and 1 of that plan (platform 3.1.0): crosire's d3d8to9 vendored,
  `d3d8-chain.py --use d3d8to9`, and `platform/d3d9/`, the shared header and HLSL build
  for plugins that draw through Direct3D 9.
- The first step of phase 2 (lighting 1.5.0): `Shaders=1` lights the GPU-drawn hulls
  per pixel through that device, reproducing the fixed-function lighting; then the point
  lights at their real positions, so torpedo lights land on the right side (lighting 1.6.0);
  then the hulls' night lights, which the GPU path had dropped (lighting 1.7.0).

## 9.10.2 — 2026-10-05

Layers: postfx 1.1.1.

### Fixed
- Selecting many ships no longer drags the frame rate down: each selected ship's bubble
  is drawn on the CPU, and that path was reading back from GPU memory under DXVK. 30
  selected went from 72.6 ms a frame to the vsync cap of 16.7 ms on the bench
  (postfx 1.1.1, b038b8e).

## 9.10.1 — 2026-10-05

Layers: qol 1.1.1.

### Fixed
- Selecting more than 16 Galaxy or Vor'cha class ships no longer crashes the game
  (qol 1.1.1).

## 9.10.0 — 2026-10-04

Layers: qol 1.1.0, testbench 2.12.2.

### Added
- Control groups: `Shift+number` adds to a group (`Alt+number` takes stock's select and
  centre), and selections and groups hold up to `MaxSelection=` (default 40) instead of
  16 (qol 1.1.0).

### Fixed
- The bench no longer leaks a `pactl subscribe` per stopped session (testbench 2.12.2).

## 9.9.0 — 2026-10-04

Layers: lighting 1.4.0.

### Added
- Nebulae stay drawn, and keep lighting nearby ships, when the camera zooms in close
  beside them (`NebulaCull=`, lighting 1.4.0).

## 9.8.3 — 2026-10-04

Layers: lighting 1.3.3.

### Changed
- Planet glow a quarter brighter and reaching further; nebula light reaching as far as
  in 9.8.1 (lighting 1.3.3).

## 9.8.2 — 2026-10-04

Layers: lighting 1.3.2.

### Fixed
- Nebula, planet and explosion light lit the far side of a ship; a planet's glow now
  comes from the planet and shows. `NebulaRange` back to 5 (lighting 1.3.2).

## 9.8.1 — 2026-10-04

Layers: lighting 1.3.1.

### Changed
- Nebula, planet, skybox and explosion light about twice as strong, and nebulae,
  planets and explosions reach further (lighting 1.3.1).

## 9.8.0 — 2026-10-04

Layers: lighting 1.3.0.

### Added
- Light sources: nebulae glow in their colour, planets light what is near their day
  side, the skybox adds a faint coloured light, explosions light their surroundings, and
  torpedo lights reach GPU-drawn ships again (lighting 1.3.0).

## 9.7.1 — 2026-10-04

Layers: textures 2.1.1, testbench 2.12.1.

### Fixed
- `a2tex install` (and so `./install`) runs in about 36 s instead of about 175 s, with
  identical results (textures 2.1.1).
- A bench session whose start fails is torn down instead of left running without a game
  (testbench 2.12.1).

## 9.7.0 — 2026-10-04

Layers: lighting 1.2.0.

### Changed
- The key light comes in at a lower angle, and the fill and ambient light are darker,
  for more contrast on ships and between a planet's day and night sides (lighting
  1.2.0).

## 9.6.0 — 2026-10-04

Layers: lighting 1.1.0.

### Added
- Planets get a night side: `Lighting.asi` lights them with the key and fill like the
  ships, where stock gave their material a constant half-white term that lit them all
  round (lighting 1.1.0). Seen on the bench.

## 9.5.0 — 2026-10-04

Layers: testbench 2.12.0.

### Added
- `testbench/scene/`: `Scene.asi` builds test scenes inside a running mission (a ship
  beside a planet, beside a nebula, or firing at an indestructible target), with fog,
  HUD, grid, cursor and notices off. `a2test drive scene` moves a free camera and
  changes the scene while it runs. This is for renderer work that end-to-end scenarios
  reach only with difficulty (testbench 2.12.0).

### Changed
- `a2test session stop` terminates the game; `--graceful` quits through its menus
  (testbench 2.12.0).

Installs nothing into the game.

## 9.4.0 — 2026-10-04

Layers: lighting 1.0.0 (new), models 3.2.0, testbench 2.11.0.

### Added
- The `lighting` layer: `Lighting.asi` draws ships and stations through the engine's own
  static vertex buffers (the GPU path stock uses only for asteroids) and replaces each
  map's lights with a warm key and a dim blue fill from `Lighting.ini` (lighting 1.0.0).
  `./install` runs `lighting/install.sh` and `a2mod` switches it; the release package and
  installers do not carry it yet.
- `models/hull-bump.py`: Federation hulls lit per pixel through the engine's dot3 bump
  path, with one flat height map, `a2flatbump.tga`, and a corrected dot3 shader
  (models 3.2.0). Not run by `./install`. `textures/tools/inventory.py` does not count
  the map as stock art.
- `testbench/d3dtrace/`: a bench-only Direct3D 8 tracer, which showed how the engine
  lights a frame (testbench 2.11.0).

Confirmed in game 2026-10-04.

## 9.3.0 — 2026-10-04

Layers: models 3.1.0.

### Added
- The dilithium moons lose their facets but keep their lumpy shape: `models/moon-sod.py`
  smooths the rock in each moon model, leaving the glow shell stock for the frame rate
  (models 3.1.0, 59e503a). `./install` runs it through `models/install.sh`.

Confirmed in game 2026-10-04.

## 9.2.0 — 2026-10-04

Layers: grid 1.0.0 (new), testbench 2.10.0.

### Added
- The `grid` layer: `GridLayout.asi` turns the button bar into a 5×3 grid with one key
  per cell by keyboard position (QWERT / ASDFG / ZXCVB; T cancel, G back), labelled on
  the buttons, replacing the bar's stock keys. At 16:9 and wider it sits between the
  minimap and the info panel, which moves right (grid 1.0.0). QOL-1 in
  `qol/README.md`. `./install` runs `grid/install.sh` and `a2mod` switches it; the
  release package and installers do not carry it yet.
- Scenario `grid-layout` and the fragment `_enter-instant-action` (an Instant Action
  match as the Borg against one easy AI); `GAMEPLAY.md` notes on getting a match with
  units (testbench 2.10.0).

Confirmed in game 2026-10-04.

## 9.1.0 — 2026-10-03

Layers: qol 1.0.0 (new), testbench 2.9.0.

### Added
- The `qol` layer: `QOL.asi` makes right-click-drag panning 2.5x as fast (`PanSpeed=` in
  `QOL.ini`), scaled in memory so `RTS_CFG.h` stays stock and network games still
  accept the player (qol 1.0.0, 8a8248b). `./install` runs `qol/install.sh`, `a2mod`
  switches it, and the release package and both installers carry it.
- The gameplay ideas move from the root `IDEAS.md` to `qol/README.md`, split into
  changes that stay compatible with stock players (`QOL.asi`) and rule changes every
  player needs (a future rules plugin).
- Scenario step `Right-drag from X,Y to X,Y` (testbench 2.9.0).

Confirmed in game 2026-10-03.

## 9.0.0 — 2026-10-03

Layers: models 3.0.0, testbench 2.8.1.

### Added
- Planets are drawn round: `Planets.asi` makes the engine tessellate them for the
  resolution in use instead of for 640x480 (models 3.0.0). `./install` runs
  `models/install.sh`, and `a2mod` switches the plugin with the `models` layer (88887e3).
- `scenarios/no-assets.md` checks the plugin patched both sites (testbench 2.8.1).

Confirmed in game 2026-10-03.

## 8.11.0 — 2026-10-03

Layers: msaa 1.1.0.

### Fixed
- Under MSAA, the top row and left column of the 3D view no longer collect grid lines
  as the camera pans; new `MSAA.ini` key `EdgeFill=` (msaa 1.1.0, 719a8c3).

Confirmed in game 2026-10-03.

## 8.10.0 — 2026-10-03

Layers: testbench 2.8.0.

### Added
- Bench agents get `testbench/GAMEPLAY.md`, notes on playing a match, and report their
  own lessons for it (testbench 2.8.0, af3c97c).

Installs nothing; no game-side change to confirm.

## 8.9.1 — 2026-10-03

Layers: testbench 2.7.1.

### Fixed
- `a2test drive step` works again (testbench 2.7.1).

Installs nothing; no game-side change to confirm.

## 8.9.0 — 2026-10-03

Layers: online 0.5.0.

### Changed
- The online server runs publicly at `c20e.de:2399`, and `Online.asi` uses it by
  default (`Server=c20e.de`) for join codes and the relay (online 0.5.0, f8012c9).

Confirmed in game 2026-10-03.

## 8.8.0 — 2026-10-03

Layers: online 0.4.0, testbench 2.7.0.

### Added
- Online multiplayer, milestone 2: join codes, hole punching and a relay through a
  self-hostable server, `online/server/a2online-server.py` (online 0.4.0). No public
  instance yet: `Server=` in `Online.ini` is empty by default. Bench scenarios for
  joining by code, directly and through the relay (testbench 2.7.0). `Online.asi` is
  still not part of `./install` (2f2255a).

Confirmed in game 2026-10-03.

## 8.7.0 — 2026-10-03

Layers: online 0.3.0, testbench 2.6.0.

### Added
- Online multiplayer, milestone 1: *Internet – Online* runs the game on `Online.asi`'s
  own UDP transport instead of DirectPlay, so hosting and joining work under Proton
  without Microsoft's DLLs (online 0.3.0). No server yet: the host must be reachable as
  on a LAN. Bench scenarios for a whole match, with and without packet loss (testbench
  2.6.0). `Online.asi` is still not part of `./install` (6cbeed9).

Confirmed in game 2026-10-03.

## 8.6.0 — 2026-10-03

Layers: online 0.2.0, testbench 2.5.0.

### Added
- *Internet – Online* on the Multiplayer Connection screen, in place of the IPX button
  that cannot work today; it connects as Manual IP until the transport exists (online
  0.2.0), with a bench scenario (testbench 2.5.0). `Online.asi` is still not part of
  `./install` (fd40da4).

Confirmed in game 2026-10-03.

## 8.5.0 — 2026-10-03

Layers: testbench 2.4.0.

### Added
- Bench scenarios with several games (`Players:`, `Setup:`), and
  `multiplayer-two-players.md`, which hosts, joins and plays a match unattended
  (testbench 2.4.0, 245d1f6).

Installs nothing; no game-side change to confirm.

## 8.4.0 — 2026-10-03

Layers: online 0.1.0.

### Added
- `online/`, the start of online multiplayer: `Online.asi` traces the game's DirectPlay
  8 calls, and `reference-dplay.sh` sets up a bench clone with Microsoft's DirectPlay to
  trace against (online 0.1.0, 14867b0). Not part of `./install` or `a2mod` yet.

Confirmed in game 2026-10-03.

## 8.3.0 — 2026-10-03

Layers: testbench 2.3.0.

### Added
- `./a2test watch`: every live bench session, view-only, tiled in one window
  (testbench 2.3.0).

## 8.2.0 — 2026-10-02

Layers: menus 4.3.1, testbench 2.2.0.

### Fixed
- Options' version label "1.1" and the Manual IP field reach the screen again, and the
  Technology Tree keeps its fixed-pitch font (menus 4.3.1).
- `scenarios/multiplayer-name.md` no longer expects the default name; a clone starts
  with the player's saved one (testbench 2.2.0).

### Added
- `scenarios/menu-repaints.md`, which checks the three (testbench 2.2.0).

Installed, not yet seen in game.

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
