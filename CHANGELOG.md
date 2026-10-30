# Changelog — Armada II Refit

The project as a whole: the modpack's version, which layer versions it bundles, and
changes that belong to no single layer (`a2mod`, the repo layout, cross-layer
conventions). Each layer keeps its own `CHANGELOG.md` in its folder. Versioning rules:
[`CLAUDE.md`](CLAUDE.md), "Changelogs and versions". Newest first.

## 13.43.0 — 2026-10-30

Layers: render3d 0.29.3, ui 0.25.2, display 0.11.1.

### Changed
- The shader compiler scripts live in `tools/shaderc/` (new, not a layer): `hlsl-rs.sh`,
  `d3dbc-fix.py`, `d3dbc-slots.py`, `dxil-spaces.py`, `dxil-alpha.py` from `render3d/`,
  and `fetch-dxc.sh`, `fetch-vkd3d.sh`, `verify-generated.sh` from `gpu/shaders/`; the
  build scripts, CI cache key and docs follow. Generated shader files change in their
  header line only; the stamps are unchanged. HASH. Installed, not yet seen in game

## 13.42.4 — 2026-10-30

Layers: render3d 0.29.2.

### Fixed
- render3d: with `[Modules] world=0` or `sky=0` the menu scene keeps its planets,
  nebulae and sky (the module is loaded for the menu scene only, as hulls=0 since
  13.42.2); missions unchanged. `e23d234`

Confirmed in game 2026-10-30.

## 13.42.3 — 2026-10-29

Layers: ui 0.25.1.

### Fixed
- The minimap showed no nebulae (issue 10): the engine's map picture is drawn
  additively now. `0f38f33`

Confirmed in game 2026-10-30.

## 13.42.2 — 2026-10-30

Layers: render3d 0.29.1.

### Fixed
- render3d: with `[Modules] hulls=0` the menu scene shows its ships again (drawn by the
  hulls module for the menu scene only; missions unchanged, every hull the engine's
  through passthrough). `b1ff913`

Confirmed in game 2026-10-30.

## 13.42.1 — 2026-10-29

Layers: testbench 3.74.1.

### Fixed
- Intermittent "the game is no longer running" at launch on the bench: the audio guard
  took winepulse's corked format-probe stream on the user's output for a leak. `8c0d697`

Confirmed in game 2026-10-30.

## 13.42.0 — 2026-10-29

Layers: ui 0.25.0, testbench 3.74.0.

### Added
- ui: the last of what the stock overlay and minimap drew that ours left out, and the
  frame rate: the pointed unit's damaged systems (`[HUD] SystemIcons`), the shot
  director's scenes on the minimap in the tactical view (`ShotScenes`), the action
  cursor over the minimap (`MinimapCursor`) and `FRAMERATE_DISPLAY`'s text
  (`FrameRate`), each on as shipped. `80ce17f`
- testbench: `ui-leftovers-2` (testbench 3.74.0).

Confirmed in game 2026-10-30.

## 13.41.0 — 2026-10-29

Layers: render3d 0.29.0, ui 0.24.0, display 0.11.0, testbench 3.73.0.

### Added
- Record and replay of a `gpu` frame (`gpu/README.md`, "Record and replay"): the
  interface records one frame of render3d's and the UI's calls, in the order they were
  made, with the contents of what the frame finds made, to `$A2_DATA/replays/<name>/`
  (`a2test drive record NAME`, trigger `Gpu.record`); `gpu/replay.sh` draws it again
  with no game in `a2replay.exe` (`gpu/replay/`, `no_std` Rust linking `gpu`) under Wine
  on lavapipe with the validation layer, through Display8 on Vulkan, Direct3D 9 (DXVK) or
  Direct3D 12 (vkd3d-proton), and writes `replay-<backend>.ppm` for `a2test baseline
  diff`. The showcase frame replays on Vulkan identical to the game's (RMSE 0).
  `gpu/replay/inspect.py` lists a recording. Off by default; idle, two flags per call. `239f29c`

Confirmed in game 2026-10-30.

## 13.40.0 — 2026-10-29

Layers: display 0.10.0, render3d 0.28.0, ui 0.23.2, testbench 3.72.0.

### Added
- display: passthrough on Vulkan (`Passthrough=1`, shipped): the engine's own draws in
  stub mode are drawn by Display8 through a fixed-function emulation, into render3d's
  scene pass or the engine's targets, so a render3d module switched off or a model it
  hands back shows as stock instead of vanishing, Borg hulls included (their bump
  vertex shader ported, ROADMAP 4d). ROADMAP phase 4, first slice. `24be351`
- gpu: `display8_vk_pass` and `VkPass8` in the display contract; render3d publishes its
  pass, every `gpu` says when a `Cmd` begins and ends.
- testbench: `display-passthrough`, `display-passthrough-forward`.

Confirmed in game 2026-10-30.

## 13.39.1 — 2026-10-29

Layers: render3d 0.27.1, ui 0.23.1.

### Changed
- render3d and the UI notice a new device or a Reset through Display8's device epoch
  (the shared `gpu` crate resolves the export once); the per-frame device comparison
  stays as the fallback without Display8. `4e1846a`

Confirmed in game 2026-10-30.

## 13.39.0 — 2026-10-29

Layers: gui-installer 2.2.0.

### Added
- gui-installer: the bug report states Display8's mode, backend and the reason, read from
  `Display8.log` (gui-installer 2.2.0, `70d273b`). The README notes where the logs are written and
  the Program Files VirtualStore redirect; `display/vkprobe/README.md` no longer says
  `Vk::start` leaves the portability flag unset (docs only).

Confirmed in game 2026-10-30.

## 13.38.0 — 2026-10-29

Layers: testbench 3.71.0.

### Added
- testbench: `display-inventory-modes`, the mode switch and the right-drag pan in
  Display8's inventory (testbench 3.71.0). `d922e1e`

Confirmed in game 2026-10-30.

## 13.37.0 — 2026-10-29

Layers: testbench 3.70.0, display 0.9.1.

### Changed
- Bench games on `Backend=vulkan` run on the user's GPU with the validation layer on;
  `--vulkan-device lavapipe` and `--no-validate` opt out. Display8 ranks Vulkan devices
  discrete > integrated > virtual > other > CPU and logs its choice. `67c2e4c`

Confirmed in game 2026-10-30.

## 13.36.2 — 2026-10-29

Layers: testbench 3.69.1.

### Fixed
- Bench scenarios `ui-whole-game` and `render3d-mode-switch` brought up to date (the clone's
  interface mode, a loading-screen wait, the objectives button, forward chain and a longer
  wait for the mode switch); `ui-whole-game` passes at 1024x768 again. `6de8947`

Confirmed in game 2026-10-30.

## 13.36.1 — 2026-10-28

Layers: gui-installer 2.1.1.

### Fixed
- The release package's `install.sh` puts `Display8.ini` in on macOS as shipped, on
  `Backend=auto` (Vulkan on MoltenVK first, Direct3D 9 last), instead of rewriting it to
  `Backend=d3d9`. On a Sikarugir wrapper, Direct3D 9 is the wrapper's D9VK, where the
  main menu crashed as Render3D baked its nebulae (#19). An existing `Display8.ini`
  keeps the `Backend=` it has. `publish/installer/test.sh` expects the shipped file on
  macOS too. `4c0f9e4`
- gui-installer: the `display8` line on macOS no longer says "on Direct3D 9"
  (gui-installer 2.1.1, `4c0f9e4`).

`Backend=vulkan` on macOS (Apple M1, Sikarugir, Wine 11) confirmed in game 2026-10-28,
set by hand; the installer change: installed, not yet seen in game.

## 13.36.0 — 2026-10-25

Layers: ui 0.23.0, cutscenes 2.2.0, testbench 3.69.0.

### Fixed
- ui: the release package crashed before the first movie (Display8's stub offers no
  640x480 and the package has no cutscene proxy to raise the intro's mode): `UI.asi`
  raises it, and draws the reels' and the intro's frames fitted without the proxy. `6d7180d`

### Added
- testbench: `display-movie-no-proxy`, the movies on the stub without the proxy, set up
  by `cutscenes/binkproxy/bench-unproxy.sh` (cutscenes 2.2.0, testbench 3.69.0, `6d7180d`).

Confirmed in game 2026-10-30.

## 13.35.0 — 2026-10-25

Layers: display 0.9.0, gui-installer 2.1.0.

### Added
- display: `[Display8] VSync=0` turns the frame limit off (the vertical blank, the
  game's only one); `VSync=1`, as shipped, keeps it (display 0.9.0, `4f1d1de`).
- Display8 in the release package (`display/d3d8.dll`, `display/Display8.ini`; display
  0.9.0): `install.sh` and `install.ps1` put it in front of crosire's d3d8to9, which
  goes behind it as `d3d8_real.dll`, in stub mode on `Backend=auto` (`Backend=d3d9` on
  macOS), merge an existing `Display8.ini` by `ui/ini-merge.py`'s rules without Python,
  and put d3d8to9 back on `--uninstall`; `Render3D.asi` and `UI.asi` go in behind it.
  manifest.json: the layer `display` (`when` `display8`) and the step `display`. `2af094a`
- gui-installer: Display8 known by content, the chain read behind it, its layer and
  status line; the bug report names it (gui-installer 2.1.0, `2af094a`).

`VSync=` confirmed in game 2026-10-25. Display8 from the release package: installed, not
yet seen in game.

## 13.34.0 — 2026-10-23

Layers: render3d 0.27.0, testbench 3.68.0.

### Changed
- render3d: an explosion inside a nebula lights the gas strongly about the breach,
  within two of the ship's radii, and the rest of the cloud with a dim wash, instead
  of lighting the whole cloud in view alike. New `[Nebulae]` keys `LitExplosionCore`
  and `LitExplosionFar`; `LitExplosion` defaults to 1.5, and an install moves a
  player's old 0.75 to it (render3d 0.27.0).

### Added
- testbench: `render3d-nebula-explosion` (testbench 3.68.0).

Confirmed in game 2026-10-23.

## 13.33.0 — 2026-10-23

Layers: ui 0.22.0, testbench 3.67.0.

### Added
- `[UI] KeepClip=1`, on as shipped: in a mission under Wine, UI.asi has Wine clip the
  pointer to the full-screen game again while it moves, for the pointer that escaped
  into the desktop's bar after a screenshot (ui 0.22.0).
- testbench: a bar step, an Omarchy-like screenshot, a corner push, an Xwayland
  protocol trace and `ui-pointer-bar` (testbench 3.67.0).

Confirmed in game 2026-10-23.

## 13.32.0 — 2026-10-23

Layers: render3d 0.26.1, testbench 3.66.0.

### Fixed
- render3d: on Vulkan and Direct3D 12, the auto-assimilator's emitter flare drew as a
  black square on the Assimilator (a NaN from the fx shader's kinds weighted by zero);
  the shader selects the quad's kind (render3d 0.26.1).

### Added
- testbench: `render3d-cone-emitter`, for that flare on Vulkan (testbench 3.66.0).

Confirmed in game 2026-10-23.

## 13.31.0 — 2026-10-23

Layers: ui 0.21.0, testbench 3.65.0.

### Added
- `[UI] PointerLog=1`, off as shipped: UI.log records changes of the game window's
  focus and jumps of the pointer, for the cursor that jumps after a change of focus
  (ui 0.21.0).
- testbench: the step `Take the focus from the game ...` and `ui-focus-pointer`
  (testbench 3.65.0).

Confirmed in game 2026-10-30.

## 13.30.0 — 2026-10-23

Layers: render3d 0.26.0, testbench 3.64.0.

### Changed
- render3d's phaser beams light the hulls, stations, rocks and planets they pass on
  Vulkan and Direct3D 12, evenly along the beam, falling off with distance from it,
  out to 200 units (was 4 full, 40 gone); the emitter and the hit stay brighter. New
  keys `BeamCore`, `BeamGasReach`; `BeamLight` and `BeamReach` take new defaults, and
  an install moves a player's old defaults to them (render3d 0.26.0).

### Added
- testbench: the `beam-pass` scene and `render3d-beam-light` (testbench 3.64.0).

Confirmed in game 2026-10-23.

## 13.29.0 — 2026-10-23

Layers: render3d 0.25.0, testbench 3.63.0.

### Added
- render3d: the Borg's damaged sensors (the green sparks seen in the Mutara) hold
  still at half size and give a small steady light. New `[Hulls]` keys:
  `SensorSteady`, `SensorSize`, `SensorLight`, `SensorLightReach`. New
  `[Nebulae] LitSpark=` key (render3d 0.25.0).
- testbench: `render3d-hull-sparks` (testbench 3.63.0).

### Fixed
- render3d: welds and shields-down discharges light only a tight corona of nebula gas,
  no longer a wide round blue ball. The shipped `Render3D.ini` lists `LitCorona=`. The
  scene's hand-added lights survive the ordnance module's second read of the engine's
  lights (render3d 0.25.0).

Confirmed in game 2026-10-23.

## 13.28.0 — 2026-10-23

Layers: render3d 0.24.0, testbench 3.62.0.

### Added
- render3d's light inside the nebulae on Vulkan and Direct3D 12: explosions,
  torpedoes, pulses, phaser beams and vents light the gas they are in, from the
  clustered lights. `[Nebulae] Lit=0` turns it off; Direct3D 9 is unchanged
  (render3d 0.24.0).
- testbench: the `nebula-battle` scene and `render3d-nebula-lit` (testbench 3.62.0).

### Changed
- render3d's clustered lights' cull tightened, the same clusters (render3d 0.24.0).

Confirmed in game 2026-10-23.

## 13.27.0 — 2026-10-23

Layers: testbench 3.61.0, ui 0.20.1, render3d 0.23.2, qol 1.6.4, cutscenes 2.1.1.

### Fixed
- `ui-hud-bar` and `ui-hud-map` press Home before selecting at measured places, and
  check in `UI.log` that the click selected the unit: the Warzone start view now opens
  zoomed out in some runs, where their fixed clicks selected nothing (testbench 3.61.0).
- Several `./install`s of one tree at once (`a2test -j`) no longer clash in its builds:
  `ui/build.sh` writes `mod.rs` and `UI.asi` through temp files of its own (ui 0.20.1),
  `render3d/hlsl-rs.sh` puts each shader file in place by one rename (render3d 0.23.2),
  and the QOL and Bink proxy builds take a lock (qol 1.6.4, cutscenes 2.1.1).

Confirmed in game 2026-10-30.

## 13.26.0 — 2026-10-23

Layers: ui 0.20.0, testbench 3.60.0.

### Added
- The placement markers drawn by the UI in the theme, in every Display8 mode and on
  every backend: the footprint under a station being placed (clear: green, bracketed;
  blocked: red, hatched) and the cells other objects take (amber tiles; red and crossed
  under the footprint). `[HUD] Placement=0` gives the engine's sprites (ui 0.20.0).
- `ui-placement` (testbench 3.60.0).

### Fixed
- The footprint under a station being placed went missing on maps with many taken
  cells, wherever the UI draws the engine's last draws (in stub mode nothing else
  draws it) (ui 0.20.0).

Confirmed in game 2026-10-23.

## 13.24.1 — 2026-10-23

Layers: render3d 0.23.1.

### Changed
- The shipped `Render3D.ini` lists `[Lights]` with the clustered lights' keys at their
  defaults, so `./install` merges them into the game's file (render3d 0.23.1).

Confirmed in game 2026-10-30.

## 13.24.0 — 2026-10-23

Layers: render3d 0.23.0 (with gpu's compute), testbench 3.58.0.

### Added
- render3d's clustered lights on Vulkan and Direct3D 12: every light lights the hulls,
  planets, moons and asteroids near it per pixel; phaser beams as line lights; the
  tractor and assimilation wash confined to the cone. `[Lights] Clustered=0` turns it
  off; Direct3D 9 is unchanged (render3d 0.23.0).
- gpu: compute (storage buffers and images, compute pipelines, dispatches, barriers) on
  Vulkan and Direct3D 12, `Gpu::compute` off on Direct3D 9 (render3d 0.23.0).
- testbench: the `battle` scene and `render3d-clustered` (testbench 3.58.0).

Confirmed in game 2026-10-30.

## 13.23.0 — 2026-10-23

Layers: display 0.8.0, ui 0.19.2, testbench 3.57.1.

### Changed
- Display8 ships in stub mode on `Backend=auto` (display 0.8.0): render3d and the UI
  draw the whole frame on Vulkan, else Direct3D 12, else Direct3D 9. An install brings
  a `Display8.ini` that still holds the old defaults (`forward`, `d3d9`) to the new ones
  and keeps a value set by hand (`Defaults=2`, `ui/ini-merge.py`, ui 0.19.2).
  `display-inventory` and `display-engine-draws` pin forward mode (testbench 3.57.1).

Confirmed in game 2026-10-30.

## 13.22.0 — 2026-10-23

Layers: display 0.7.0, testbench 3.57.0.

### Added
- Display8's `Backend=auto` (display 0.7.0): the stub tries Vulkan, Direct3D 12, then
  Direct3D 9 at the engine's first `CreateDevice`, and any `Backend=` falls back to Direct3D 9 when its own cannot
  start, with each attempt in `Display8.log`. `display8_backend` answers once the device
  is made. The shipped defaults are unchanged (`Mode=forward`, `Backend=d3d9`).
- `display-fallback` (testbench 3.57.0): Vulkan and Direct3D 12 made to fail on
  purpose (`FailBackend=vulkan,d3d12`), Direct3D 9 drawing the game.

Confirmed in game 2026-10-30.

## 13.21.1 — 2026-10-23

Layers: display 0.6.2.

### Fixed
- Display8's stub logs the backend that could not start, and `Display8.ini` describes
  the stub's three backends (display 0.6.2). The docs of display, gpu, render3d, ui and
  platform describe the chain as `./install` leaves it, Display8 in front, and the stub
  on Direct3D 9, Vulkan and Direct3D 12. The shipped defaults are unchanged.

Confirmed in game 2026-10-23.

## 13.21.0 — 2026-10-23

Layers: platform 3.6.0, menus 4.3.5, renderer 2.0.1.

### Added
- `A2_HEROIC` in `a2env.sh`/`a2env.py`: Heroic's config directory, the native
  `~/.config/heroic` or else the Flatpak's
  `~/.var/app/com.heroicgameslauncher.hgl/config/heroic`; `A2_PROTON`'s default hangs
  off it, and `a2env.heroic_roots()` lists every one present.

### Fixed
- Every Heroic config and tool lookup goes through `A2_HEROIC`, so the Flatpak Heroic
  works: `d3d8-chain.py --status` no longer crashes without `~/.config/heroic` and
  recognises the Flatpak Heroic's DXVK by hash (platform 3.6.0); `menus/install.sh`'s
  override check (menus 4.3.5); `renderer-config.sh`'s DXVK lookup (renderer 2.0.1);
  umu in `online/reference-dplay.sh` and Proton in `cutscenes/binkproxy/run-test.sh`.

Confirmed in game 2026-10-30.

## 13.19.0 — 2026-10-23

Layers: testbench 3.56.0.

### Fixed
- `qol-refused-advisor` selects the starbase after Home, wherever the start view put
  it, and checks the selection, the keys, the payments and the refusal it stages (testbench 3.56.0).

Confirmed in game 2026-10-30.

## 13.18.0 — 2026-10-23

Layers: display 0.6.1, render3d 0.22.1, ui 0.19.1, testbench 3.55.0.

### Fixed
- Stub mode: the movies played black with their sound, on every backend; Display8 now
  puts what the engine writes into the locked back buffer into the frame at the unlock,
  under the UI's movie frame (display 0.6.1).
- The long waits on Vulkan before the main menu and at a mission's start: `gpu`'s
  read-back read the card's memory through the PCIe window, and render3d's nebula bake,
  which reads its whole volume back, took 0.4 to 1.5 s a class; every class was also
  baked twice (render3d 0.22.1, ui 0.19.1).

### Added
- `display-movie`: the movies on screen (testbench 3.55.0).

Measured on the bench (1280x720, RX 5700, cold pipeline cache, `a2_fed01`): nebula
bakes on Vulkan before 4.3 s at the main menu and 11.5 s at the mission's first frames,
after 0.35 s and 0.68 s (forward mode: 0.32 s and 0.80 s). Confirmed in game 2026-10-23.

## 13.16.0 — 2026-10-23

Layers: display 0.6.0, render3d 0.22.0, ui 0.19.0, testbench 3.53.0.

### Added
- Phase 6 of the display plan: render3d and the UI draw the whole frame on Display8's
  Direct3D 12 device (`Backend=d3d12`) through `gpu`'s Direct3D 12 backend, from the
  DXIL built beside every shader, with alpha-tested twins where a pipeline has an alpha
  test (display 0.6.0, render3d 0.22.0, ui 0.19.0). Off by default (`Backend=d3d9`).
- `display8_dx12_flush` and `Dx12Device8::features`, `sampler_anisotropy`,
  `sampler_lod_bias` in the contract (display 0.6.0).
- The bench loads the Khronos layer under vkd3d-proton for `Backend=d3d12` with
  `Validate=1` (testbench 3.53.0).

### Fixed
- In the DXIL, pixel shaders that left out or reordered their vertex shader's position
  take it first, as Direct3D 12 links stages by register (render3d 0.22.0).

Measured on the bench against Direct3D 9 on the GPU: showcase RMSE 0.97, nebula 0.39,
firing 0.41 (/255); `display-stub` (the whole game) passes on Direct3D 12. Installed,
not yet seen in game.

## 13.15.0 — 2026-10-23

Layers: testbench 3.52.0, models 5.0.1.

### Added
- The bench's stock cache: stock-start sessions copy a kept `a2mod stock` of the install
  instead of running it on each clone (testbench 3.52.0).

### Changed
- `hull-sod.py` keeps its rounded models in `$A2_DATA/cache/hull-sod/`, so a repeat
  install takes 3 s instead of 25 s (models 5.0.1).

Confirmed in game 2026-10-30.

## 13.12.0 — 2026-10-23

Layers: display 0.5.0, render3d 0.21.0, ui 0.18.0, testbench 3.51.0.

### Added
- Phase 5 of the display plan: Display8's stub mode on Vulkan (`Backend=vulkan`), and
  render3d and the UI drawing the whole frame on it through `gpu`'s Vulkan backend,
  from the SPIR-V built beside every shader's d3dbc (display 0.5.0, render3d 0.21.0,
  ui 0.18.0). Off by default (`Backend=d3d9`).
- The bench runs a Vulkan game on lavapipe with the validation layer, never on the GPU
  (testbench 3.51.0, display 0.5.0).

Measured on the bench against Direct3D 9 on the GPU: showcase RMSE 0.87, nebula 0.36,
firing 0.40 (/255) on the GPU; `display-stub` (the whole game) passes on Vulkan; no
validation message. Confirmed in game 2026-10-23.

## 13.11.0 — 2026-10-23

Layers: ui 0.17.2, testbench 3.50.1.

### Fixed
- A placement's build footprint was not drawn with Display8 in stub mode; the UI now
  draws it, as the order ring (ui 0.17.2).
- `ui-hud-map` and `ui-hud-bar` click the Assembler and the Nexus where the start view
  puts them (testbench 3.50.1).

Confirmed in game 2026-10-23.

## 13.10.0 — 2026-10-23

Layers: display 0.4.0, render3d 0.20.0, ui 0.17.1.

### Added
- Phase 6 of the display plan, Display8's side: stub mode on Direct3D 12
  (`Backend=d3d12`) with the contract's Direct3D 12 exports in `gpu/src/display.rs`
  (display 0.4.0).
- Every shader also built as signed DXIL by a pinned dxc (v1.9.2609, fetched into
  `$A2_DATA/tools/` by `gpu/shaders/fetch-dxc.sh`) (render3d 0.20.0, ui 0.17.1,
  display 0.4.0).

Confirmed in game 2026-10-30.

## 13.8.0 — 2026-10-23

Layers: ui 0.17.0, cutscenes 2.1.0, testbench 3.50.0.

### Added
- Phase 1 of the display plan, the UI's part: the engine's last draws in a mission
  (drag box, height posts, order ring, fade, the unit view's video) drawn by the UI
  (ui 0.17.0); full-screen movies handed by the cutscene proxy to the UI to draw
  (ui 0.17.0, cutscenes 2.1.0); the in-mission menus over the UI's own copy of the
  frame (ui 0.17.0); scenario `display-engine-draws` (testbench 3.50.0).

### Fixed
- A display mode chosen in the middle of a mission no longer crashes the game (a stack
  overflow between the UI's window subclass and DXVK's window hook) (ui 0.17.0).

Confirmed in game 2026-10-23.

## 13.7.0 — 2026-10-23

Layers: display 0.3.0, testbench 3.49.0.

### Added
- Display8's stub mode (`Mode=stub`, off by default): the engine draws nothing and
  render3d and the UI draw the whole frame on a Direct3D 9 device of Display8's own
  (display 0.3.0). Scenarios `port-*` and `display-stub` (testbench 3.49.0).

Confirmed in game 2026-10-23.

## 13.6.0 — 2026-10-23

Layers: display 0.2.0, platform 3.5.0.

### Added
- Display8 in the install: `./install` puts it in front of the d3d8 chain, `a2mod`
  switches it (layer `display`: stock puts the chain's own `d3d8.dll` back), drop
  switches by category, and the plugins can ask it for the device's epoch (display
  0.2.0). `d3d8-chain.py` switches the chain behind it (platform 3.5.0).

Confirmed in game 2026-10-23.

## 13.5.0 — 2026-10-23

Layers: render3d 0.19.0, ui 0.16.0.

### Changed
- `gpu/`: one graphics interface for render3d and the UI, shaped for Vulkan and
  Direct3D 12, on Direct3D 9 for now; the UI and every module of render3d draw through
  it. Every shader is modern HLSL, one source for the Direct3D 9 bytecode and SPIR-V
  (`gpu/shaders/compat.hlsli`). The same pixels as before on the bench: 39 frames of
  `port-showcase`, `port-nebula` and `port-firing` at RMSE 0 (render3d 0.19.0, ui
  0.16.0).

Confirmed in game 2026-10-23.

## 13.4.0 — 2026-10-23

Layers: display 0.1.0 (new), testbench 3.48.0.

### Added
- `display/`: Display8, a `d3d8.dll` of ours in front of the real chain, forward mode,
  on the bench only: every Direct3D 8 call counted by context, method and caller, with
  the answers, locks, read-backs and gamma the engine asks for, a drop switch by caller
  and a detail capture of the engine's draws (display 0.1.0). Not in `./install` or
  `a2mod` yet.
- Scenario steps `Mark the display context` and `Expect the engine to draw nothing in`,
  scenario `display-inventory` (testbench 3.48.0).

Confirmed in game 2026-10-23.

## 13.3.0 — 2026-10-23

Layers: testbench 3.47.0.

### Added
- Bench scenarios wait for the log lines they check instead of a fixed time
  (`Wait up to N seconds for the logs the next steps expect`), and a log wait counts only
  lines written since the last action, and `Quit the game` no longer waits out each
  menu press (testbench 3.47.0).

Bench-only; installs nothing into the game.

## 13.2.1 — 2026-10-23

Layers: testbench 3.46.1.

### Fixed
- The bench dashboard's screens stay black while a game launches, without the striped
  placeholder (testbench 3.46.1).

Confirmed in game 2026-10-30.

## 13.2.0 — 2026-10-22

Layers: testbench 3.46.0.

### Added
- `a2test watch --install-desktop`: the bench dashboard in the app launcher, always run
  from the main checkout (testbench 3.46.0).

Confirmed in game 2026-10-30.

## 13.1.0 — 2026-10-22

Layers: render3d 0.18.0, ui 0.15.0, testbench 3.45.0, films 1.13.0.

### Added
- A frame that repeats on the test bench: `[Bench] Clock=bench` in `Render3D.ini` and
  `UI.ini` (default `wall`, as before) runs every animation on the sim time, and the
  scene builder's `Seed=`, `Step=` and `Pause=` put two launches on the same tick;
  `a2test --clock bench`, scenario `bench-determinism` (render3d 0.17.0, ui 0.14.0,
  testbench 3.42.0).
- Pixel baselines: `a2test baseline record|compare|diff|list|show` and the baseline
  steps, for PNG, PPM and PFM (testbench 3.43.0).
- Frame dumps: `Render3D.dump` / `UI.dump` make the plugins write the HDR scene, the bloom
  levels and the presented frame; `a2test drive dump` (render3d 0.18.0, ui 0.15.0,
  testbench 3.44.0).
- The bench dashboard in place of `a2test watch`'s page, who started each job recorded
  at start, `a2test drive note` for an agent's intent, the judge's tokens per day; noVNC
  vendored in `testbench/vendor/` (testbench 3.45.0, films 1.13.0).
- `render3d/perf/ab.sh`: frame-time A/B of any two checkouts or revisions (render3d 0.18.0).

Confirmed in game 2026-10-22.

## 13.0.0 — 2026-10-22

Layers: renderer 2.0.0 (was postfx), gui-installer 2.0.0, testbench 3.41.2, films 1.12.1.

### Removed
- The vkBasalt bloom and its ReShade preset, everywhere: `postfx/postfx.py`,
  `postfx/vkbasalt/`, the `bloom` layer of `a2mod`, the release package's `bloom/`
  folder and `publish/installer/shaders.txt`, the MagicBloom and ReShade header credits,
  and the GUI installer's vkBasalt panel and how-to. Bloom, tone mapping and every
  other post-process are `Render3D.asi`'s (renderer 2.0.0, gui-installer 2.0.0).
- `a2mod` no longer flips vkBasalt's `enableOnLaunch`; a `bloom` key in an older
  `state.json` is ignored.
- The test bench and the film tooling no longer carry vkBasalt into a session
  (testbench 3.41.2, films 1.12.1).

### Changed
- `postfx/` is `renderer/`, the name of the one `a2mod` layer it still holds.
- What an earlier version set up comes out, since both would bloom the frame twice:
  `./install` removes the per-user vkBasalt layer; `install.sh` the `armada2-refit-bloom`
  folder it wrote; `install.ps1` `A2Bloom.ini` and its `ReShadePreset.ini` copy; the GUI
  installer the vkBasalt variables it wrote into Heroic. The package's `bloom` step does
  this now, so the step list and the package schema are unchanged.

Confirmed in game 2026-10-22.

## 12.42.1 — 2026-10-22

Layers: none (publish/ is versioned by the root).

### Added
- CI scans every PE in the release zip and the Windows installer on VirusTotal
  (`publish/virustotal.py`, job `virustotal`; on `main` and on dispatch, report only)
  and attaches `virustotal.json` and `virustotal.md` to the release with a VirusTotal line
  in its notes (`virustotal-release`). Secret `VT_API_KEY`; variable `VT_FAIL_ABOVE`
  makes the scan fail above that many detections (issue #4).
- `.github/workflows/virustotal-fp.yml` and `publish/virustotal-fp.py`: for a release
  tag, an issue "False positives: vX.Y.Z" with a checklist per antivirus vendor
  (`publish/av-vendors.json`, kept by hand), the flagged files zipped as an artifact
  (password `infected`), and optionally a comment and harmless vote on VirusTotal.

Installs nothing.

## 12.42.0 — 2026-10-22

Layers: gui-installer 1.8.0.

### Added
- Installer errors that explain themselves (issue #4): `install.ps1` and `install.sh`
  check every file of the package before stopping and name each one an antivirus took
  or locked, as an antivirus; a file the game folder refuses is named too. With
  `A2_PROGRESS=1` each failure is first printed as `::error <code> <path>` (`av-locked`,
  `av-missing`, `damaged`, `game-running`, `no-game`, `write-denied`).
- gui-installer: those failures in words, with what to do, a **Copy report** button and
  the README's new "Antivirus false positives"; an unpack an antivirus still holds no
  longer blocks every retry.
- README.md: "Antivirus false positives", and `README.txt` in the package points at it.

### Changed
- CI keeps the newest ten releases on GitHub, not three (tags were always kept).
- The damaged-package message names the files by their Windows path in `install.ps1`
  and no longer lists `sha256sum -c`'s output in `install.sh`.

Confirmed in game 2026-10-22 (the error dialog and Copy report, on Linux).

## 12.41.3 — 2026-10-22

Layers: testbench 3.41.1.

### Fixed
- testbench: `umu_run.py` is found beside `A2_PROTON`, so the bench launches with
  Heroic's Flatpak.

Installs nothing.

## 12.41.2 — 2026-10-21

Layers: render3d 0.16.4, ui 0.13.4, hud 2.2.3, menus 4.3.4, qol 1.6.3, grid 1.0.2, online 0.6.3, cutscenes 2.0.3.

### Changed
- Every plugin we build (the eight ASIs and `binkw32.dll`) carries a version resource:
  company, product, description, file and product version, copyright and the
  repository URL, from `platform/versioninfo/` (issue #4: antivirus false positives;
  915cad8).
- `publish/package.sh` refuses to package a plugin of ours without one, and
  `publish/installer/test.ps1` reads each as Windows does. `version` moved to
  `platform/versioninfo/version.sh`, which the build scripts source too.

Confirmed in game 2026-10-22.

## 12.41.1 — 2026-10-21

Layers: qol 1.6.2.

### Fixed
- qol: an order `QOLRules.asi` refuses for want of resources speaks stock's advisor
  line again ("Insufficient crew", ...), to the refused player only (issue #1, c23cb19).

Confirmed in game 2026-10-22.

## 12.41.0 — 2026-10-21

Layers: render3d 0.16.3, testbench 3.41.0.

### Fixed
- render3d: a black hole under the fog of war is no longer a dark disc in the fog: a
  fog-of-war ghost's additive meshes are added at half strength, as stock draws them
  (issue #3, 7e7a184).

### Added
- testbench: scene and scenario `fog-blackhole` (issue #3).

Confirmed in game 2026-10-22.

## 12.40.0 — 2026-10-21

Layers: render3d 0.16.2, testbench 3.40.0.

### Fixed
- render3d: a freighter carrying latinum trails its golden spray again, and every
  freighter's cargo pods light in the colour of its cargo up to the fill, as in stock
  (GitHub issue #2, 7c49408); old SODs' alpha-tested meshes are cut-outs, as the
  engine draws them.

### Added
- testbench: the scene command `cargo`, the `freighters` scene and the scenario
  `render3d-freighter-trail`.

Confirmed in game 2026-10-22.

## 12.39.1 — 2026-10-21

Layers: render3d 0.16.1.

### Changed
- CI: a changelog entry with a line starting "Pre-release" is released as a
  pre-release even when confirmed in game (`publish/README.md`).

### Fixed
- render3d: on Windows, on a card that takes shaders of 4096 instruction slots, the
  sky behind the main menu and every map was black and the planets stock: the sky's
  and the planets' bakes are now chains of passes that fit, and `Render3D.log` names
  any shader Direct3D refuses (0bd0a97).

Confirmed in game 2026-10-21.
Pre-release until it is seen on Windows.

## 12.39.0 — 2026-10-20

Layers: gui-installer 1.7.0.

### Added
- gui-installer: **Diagnostics for a bug report** in the window and `--report` on the
  command line, a report a player can post: the Direct3D chain and the plugins by hash
  against the release, changed settings, the plugins' logs, and findings such as
  UI.asi and Render3D.asi without crosire's d3d8to9; user names and addresses left out.

Confirmed in game 2026-10-30.

## 12.38.1 — 2026-10-18

Layers: ui 0.13.3.

### Fixed
- ui: the button bar is shown at the start of a mission when Game Settings' "Command
  Bar" says Shown; the setting was checked against the wrong flag, and the toggle could
  hide a bar that was already up.

Confirmed in game 2026-10-19.

## 12.38.0 — 2026-10-17

Layers: films 1.12.0, online 0.6.2, render3d 0.16.0, testbench 3.39.0.

### Added
- films: `Music:` beds from a path under the game's `sounds/`; a shot's `Labels:`; the
  intro cinematics' reference moments.

- testbench: `Scene.asi`'s `Captions=0`, a mission's intro cinematics without the script's
  subtitles, stardate lower thirds and title (`ShowTitle`/`ShowSubtitle` add no overlay
  message); `films/intros-scout` is filmed with it.

- render3d: a torpedo or pulse lights its own ship where it leaves it, for half a second in
  the shot's colour (the muzzle light; `[Ordnance]` `MuzzleLights`, `MuzzleBrightness`,
  `MuzzleTime`, `MuzzleReach`); until now only phasers lit their emitter.

### Changed
- render3d: the auto-assimilator's beam (the Assimilator ship's special) is brighter and
  carries further, so it reads against dark space (`Brightness` 2.0, `Fall` 0.4); the
  other beams are unchanged.

### Fixed
- films: `check` takes stem mixes on reference shots; a reference's `Quality:` with a note.
- films: a 4K animatic no longer runs the machine out of memory (it is joined in runs).
- online: the bench's local server outlives a 4K take's start, so the joiner gets a join
  code.

Installed, not yet seen in game (render3d); films, online's bench script and testbench
install nothing.

## 12.37.0 — 2026-10-17

Layers: testbench 3.38.0, films 1.11.0.

### Added
- Scenes `assimilation-fusion` and `assimilator`, for the Fusion Cube's assimilation
  beam and the Assimilator's auto-assimilator (testbench 3.38.0); scratch boards
  `films/beams-scout`, `films/fleets-scout`, `films/intros-scout` and `films/online-scout`
  for the promo's new shots (films 1.11.0).

Installs nothing; the beams seen firing on the bench 2026-10-17 (stock), the boards not
yet filmed.

## 12.36.0 — 2026-10-17

Layers: testbench 3.37.0.

### Added
- `Scene.asi`'s `Race=` key and `SCENE_RACE=<race>` at install: the player's race, and
  so the HUD's look, pinned for a bench scene in stock and refit alike (testbench
  3.37.0).

Installs nothing; seen working on the bench 2026-10-17 (`showcase`, `Hud=1`, `Race=federation`:
the Federation HUD on 3 of 3 launches in stock and 3 of 3 in refit; `SCENE_RACE=klingon`
over it: the Klingon theme).

## 12.35.0 — 2026-10-17

Layers: testbench 3.36.0, films 1.10.0.

### Added
- Scene command `tech` and key `Tech=`, scene `assimilation`, so the bench can stage
  the Borg technology assimilation beam (testbench 3.36.0); board `films/assimilation`
  (films 1.10.0).

### Fixed
- Scenario `specials`: the assimilation beam forms (testbench 3.36.0).

Confirmed in game 2026-10-30.

## 12.34.2 — 2026-10-17

Layers: ui 0.13.2, testbench 3.35.1.

### Fixed
- `UI.asi`'s cursor is hidden while the engine's would be: Scene.asi's `Cursor=0` /
  `cursor off` hides the pointer in refit takes again, and the loading screen shows no
  cursor, as in stock.

Confirmed in game 2026-10-30.
## 12.34.1 — 2026-10-18

Layers: render3d 0.15.1, ui 0.13.1.

### Fixed
- Alt-tab out of the game and back on Windows no longer ends in "Fatal error in
  Direct3D": render3d lets go of every default-pool object before the device's `Reset`.

Confirmed in game 2026-10-18.

## 12.34.0 — 2026-10-18

Layers: ui 0.13.0.

### Added
- The game runs at the desktop's resolution from the first launch (`[UI] Native`),
  instead of the stock 1024x768 and a mode change at every switch to the menus.

Confirmed in game 2026-10-18.

## 12.33.0 — 2026-10-18

Layers: ui 0.12.1, render3d 0.15.0.

### Fixed
- UI.asi and Render3D.asi on Windows: their shaders are rewritten into what Windows'
  own Direct3D 9 accepts (`render3d/d3dbc-fix.py`, `hlsl-rs.sh --fix`, and no sampling
  inside a branch in the HLSL), so the menus and the HUD draw instead of a black screen,
  and render3d draws the 3D view instead of standing down.
- Both let go of everything they hold on the engine's device before it releases it
  (`DestroyRenderSurfaces`), so a mission starts on Windows instead of ending in "Fatal
  error in Direct3D".

Confirmed in game 2026-10-18.

## 12.32.0 — 2026-10-17

Layers: testbench 3.35.0.

### Added
- Test bench: `Scene.asi`'s `Zoom=` key and `zoom` command, the RTS camera's zoom as a
  level from 0 to 1, for shots that need a player's-eye view without key presses.

Confirmed in game 2026-10-30.

## 12.31.0 — 2026-10-17

Layers: ui 0.12.0, testbench 3.34.0.

### Added
- A regression test for the HUD's layout: `./a2test run ui-hud-layout` lays the UI's
  HUD out at 4:3, 16:9 and 21:9 with 1, 6 and 30 ships selected, from UI.asi's own
  panel rectangles (`[HUD] LayoutLog`), and writes the shots as a contact sheet.

Confirmed in game 2026-10-30.

## 12.30.0 — 2026-10-17

Layers: testbench 3.33.0, films 1.9.0.

### Added
- Takes record the game's sound by category: `Mute:` silences music, voice, effects or
  interface in a take (`a2test session start --mute`), and `Stems:` records music, voice
  and interface as separate tracks beside it (`--stems`, `Stems.asi` in the bench clone
  only), mixed per shot in the animatic with `Audio: effects + voice -6` (e682c70).

Confirmed in game 2026-10-30.

## 12.29.0 — 2026-10-17

Layers: testbench 3.32.0, films 1.8.0.

### Added
- Event marks for films: `Scene.asi` logs timestamped events per take, `a2film` keeps
  them as `<take>.marks.json`, lists them (`a2film marks`), suggests `In`/`Length` from
  them (`Cut:`) and checks a shot's `Expect:` after filming (1aacd31).

Confirmed in game 2026-10-30.

## 12.28.0 — 2026-10-17

Layers: testbench 3.31.0, films 1.7.0.

### Added
- A machine-wide queue for game slots: bench sessions, `a2test run` cases and `a2film`
  takes wait for a free slot (`A2_SLOTS`, default 2 to 3) instead of all starting at
  once, and say who holds the others; `a2test slots` and `a2film slots` list them
  (testbench 3.31.0, films 1.7.0).
- `a2film` films a take once more when its game died with it (films 1.7.0).

Tooling only; nothing to see in game.

## 12.27.0 — 2026-10-17

Layers: films 1.6.0.

### Added
- `a2film`: boards split over files (`Include:`), `a2film adopt` for another board's
  takes of the same shot, the shot field `Tier:` for money shots probed larger in the
  same run (at the final resolution: filmed as the final take), and the reference catalogue `films/reference/` with `From: ref:<id>`; the
  rough cut uses each shot's best take (films 1.6.0, 13be105).

Tooling only; nothing to see in game.

## 12.26.0 — 2026-10-17

Layers: films 1.5.0.

### Added
- `a2film review`: contact sheets, flags, motion curves and variant differences for the
  takes of a board, with an HTML overview (films 1.5.0).

Tooling only; nothing to see in game.

## 12.25.0 — 2026-10-17

Layers: films 1.4.0.

### Added
- `a2film animatic`: numbered cuts (`animatic-<tier>-rc<N>.mp4`, the latest still
  `animatic-<tier>.mp4`) with a `cuts.json` log, partial cuts (`--only`, `--section`,
  `--from`/`--to`) and a `<cut>.shots.json` shot map beside every cut (films 1.4.0).

Tooling only; nothing to see in game.

## 12.24.1 — 2026-10-17

Layers: ui 0.11.1, testbench 3.30.1.

### Changed
- The HUD's bottom row: the bar beside the minimap at 16:9 and wider, the info panel
  moved right for it; above the minimap at 4:3. A group's tiles are smaller, so a big
  selection stays low (ui 0.11.1; testbench 3.30.1 follows it in two scenarios).

Confirmed in game 2026-10-30.

## 12.24.0 — 2026-10-17

Layers: gui-installer 1.6.0.

### Added
- The release package carries `Render3D.asi` + `Render3D.ini` with its recipes
  (`game/Surfaces/`, `game/Nebulae/`) and `UI.asi` + `UI.ini`; `install.sh` and
  `install.ps1` install them in new steps `render3d` and `ui` when d3d8to9 is
  `d3d8.dll`, and take out `HUD.asi`, `Menus.asi` and `GridLayout.asi`. Where the chain
  is left as it is, those three go in instead and the 3D view stays the engine's.
- `asteroid-sod.py` in the package's `models/`; the scripts smooth the asteroids too.

### Changed
- `manifest.json`: steps `render3d`, `ui` in place of `hud`, `menus`, `grid`; layers
  `render3d` and `ui` (`when: d3d8to9`) in place of `hud`, `menus` and `grid`.
- `publish/package.sh` builds `Render3D.asi` and `UI.asi` (rustup's cargo with
  `i686-pc-windows-msvc`), which CI's package job now installs.
- `--uninstall` / `-Uninstall` also take out `Render3D.*`, `UI.*` and the recipes.

### Fixed
- CI builds `Render3D.asi` again: its `render3d` and `package` jobs link with the Rust
  toolchain's own lld as `lld-link`, since Ubuntu's lld-link 18 refuses the `EXPORTAS`
  export rustc now writes for `DllMain` (red on `main` since 12.21.0 at least).

Confirmed in game 2026-10-30.

## 12.23.0 — 2026-10-17

Layers: render3d 0.14.0.

### Changed
- The dilithium moons' light lies deep in their cracks: dark walls, the light at the
  bottom, hidden by the near wall at an angle; `[World] MoonCrackDepth`,
  `MoonCrackFalloff` (render3d 0.14.0).

Confirmed in game 2026-10-17.

## 12.22.0 — 2026-10-17

Layers: render3d 0.13.0, testbench 3.30.0.

### Added
- `[World] PlanetGhost`, how dim a fogged planet is drawn (render3d 0.13.0); scene
  `fog-planet` and scenario `render3d-planet-ghost` (testbench 3.30.0).

### Fixed
- A planet under the fog of war keeps render3d's sphere and surface instead of the
  engine's low-polygon sphere and stock texture (render3d 0.13.0).

Confirmed in game 2026-10-17.

## 12.21.0 — 2026-10-17

Layers: ui 0.11.0, render3d 0.12.0, testbench 3.29.0.

### Added
- The Credits are the UI's: the game's credits page by page over the menu scene, the
  logos at their own aspect, Esc back to the main menu (ui 0.11.0).
- Game Settings' "Command Bar" shows or hides the button bar, remembered between
  sessions (ui 0.11.0).
- The warp cursor over the minimap when a selected ship can warp (ui 0.11.0).
- Hover sounds on every main menu and campaign selection entry (ui 0.11.0); the menu
  scene's Klingons decloak with the cloak's sound (render3d 0.12.0).

### Changed
- Options, Instant Action, Multiplayer and the other front-end screens stand over the
  menu scene; the Admiral's Log's seven other pages are redrawn; the lobby and Game
  Setup are no wider than 16:9 (ui 0.11.0).

### Removed
- The top-right help and palette buttons (ui 0.11.0).

Confirmed in game 2026-10-17.

## 12.20.1 — 2026-10-17

Layers: ui 0.10.1.

### Fixed
- The briefing's objectives keep a gap after their LCARS switches instead of running
  under them (ui 0.10.1).

Confirmed in game 2026-10-17.

## 12.20.0 — 2026-10-17

Layers: render3d 0.11.0.

### Changed
- The dilithium moons glow from inside, through their cracks only: the cracks are lit in
  the moon's colour, and the round nimbus behind the moon is no longer drawn (render3d
  0.11.0).

Confirmed in game 2026-10-17.

## 12.19.0 — 2026-10-17

Layers: ui 0.10.0, testbench 3.28.0.

### Added
- The HUD's command and top-right buttons show flat icons of our own in the theme's
  colours, keeping the game's metaphors; the mission scripts' trade status and team
  message are drawn by the UI instead of stretched (ui 0.10.0, `[HUD] Icons`,
  `[HUD] TeamTexts`).

### Changed
- The info panel no longer shows ten empty queue slots for an idle builder; the
  minimap's filters are words and state dots (ui 0.10.0).

Confirmed in game 2026-10-17.

## 12.18.1 — 2026-10-17

Layers: render3d 0.10.1, ui 0.9.1.

### Fixed
- The warp flashes of the menu scene's departure no longer show as boxes (render3d
  0.10.1).
- The campaign selection keeps its campaigns' names in order after an aborted mission
  (ui 0.9.1).

Confirmed in game 2026-10-17.

## 12.18.0 — 2026-10-17

Layers: ui 0.9.0, render3d 0.10.0, testbench 3.27.0.

### Added
- Starting a mission from the campaign selection keeps the menu scene running under
  the loading screen: the campaign's ships go to warp with the camera following them
  into a warp tunnel, which holds until the mission starts (ui 0.9.0, render3d 0.10.0).

### Changed
- The loading screen is redrawn inside long loading steps too, after each model and
  texture loads (ui 0.9.0).

Confirmed in game 2026-10-17.

## 12.17.0 — 2026-10-16

Layers: ui 0.8.0, render3d 0.9.0, testbench 3.26.0.

### Changed
- The campaign selection gives each campaign a scene of its own on render3d's menu
  map, the camera moving to it as the campaign is pointed at: a starbase for the
  tutorials, the fleet over the planet, Klingon ships decloaking before a Metreon
  nebula, a Borg fusion cube casting its shadow through a Mutara nebula with the queen's
  Diamond close by. Its entries
  are a faction emblem and a name (ui 0.8.0, render3d 0.9.0).

### Fixed
- Ships in the menu scene cast shadows, and cut-out meshes (half a Borg fusion cube)
  cast theirs in missions too (render3d 0.9.0).
- After an aborted mission the campaign selection is over the menu scene again, not
  the stock look (ui 0.8.0).

Confirmed in game 2026-10-16.

## 12.16.0 — 2026-10-16

Layers: ui 0.7.0, render3d 0.8.0, testbench 3.25.0.

### Added
- The main menu and the campaign selection stand over a 3D scene drawn in game by
  render3d: a Federation fleet in orbit over a class M planet, with the mission
  renderer's lighting, shadows, running lights and bloom (`[Menus] Scene`; ui 0.7.0,
  render3d 0.8.0). Both screens restyled over it.

Confirmed in game 2026-10-16.

## 12.15.0 — 2026-10-16

Layers: ui 0.6.0, render3d 0.7.1, testbench 3.24.0.

### Fixed
- Planets can be clicked and colonised again with render3d (render3d 0.7.1); a
  cursor over a mission's opening briefing (ui 0.6.0).

### Changed
- The UI's HUD: the engine's tooltips themed, the selection ellipsoids off with
  thinner brackets, the resources reordered, panels kept inside the LCARS frame, a
  square unit view frame, no Save to text in the Admiral's Log (ui 0.6.0).

Confirmed in game 2026-10-17.

## 12.14.0 — 2026-10-16

Layers: ui 0.5.0, render3d 0.7.0.

### Added
- The info panel shows the selected unit live, close up, in place of its wireframe,
  and is a third bigger (`[HUD] InfoCamera`, `InfoScale`; ui 0.5.0, render3d 0.7.0).

Confirmed in game 2026-10-17.

## 12.13.0 — 2026-10-16

Layers: ui 0.4.0, hud 2.2.2, testbench 3.23.0.

### Changed
- **The UI draws the whole HUD, every menu, the loading screen and the cursor by
  default** (ui 0.4.0). `./install` no longer runs `hud/install.sh`,
  `grid/install.sh` or `menus/install.sh`: `ui/install.sh` takes `HUD.asi`,
  `GridLayout.asi` and `Menus.asi` out (`ui/supersede.sh`); the menu plates stay
  installed, for the UI's main menu. Those layers stay in the repository; `a2mod`
  still recognises their files in an older install.

### Added
- The multiplayer HUD panels and the Internet lobby; scenario `ui-whole-game` at three
  aspects (ui 0.4.0, testbench 3.23.0).

Confirmed in game 2026-10-17.

## 12.12.0 — 2026-10-16

Layers: ui 0.3.0, testbench 3.22.0.

### Added
- `UI.asi` in `./install` and `a2mod` (the `ui` layer), with every HUD panel, most menu
  screens and a loading screen of its own, each off by default in `UI.ini` (ui 0.3.0).
- Scenarios for each, and `Scene.asi`'s `succeed`/`fail` (testbench 3.22.0).

Confirmed in game 2026-10-17.

## 12.11.0 — 2026-10-16

Layers: ui 0.2.0, testbench 3.21.0.

### Added
- The UI's widgets, its own cursor, its style (a theme per race), the first HUD panel
  of its own (the button bar, superseding `GridLayout.asi` when on) and the menus
  hosted and drawn by the UI (superseding `Menus.asi` when on); all off by default
  (ui 0.2.0).
- Scenarios for each (testbench 3.21.0).

Confirmed in game 2026-10-17.

## 12.10.0 — 2026-10-16

Layers: ui 0.1.0, testbench 3.20.0.

### Added
- A new layer, `ui/`: `UI.asi`, the runtime the HUD, menus and loading screen will be
  redrawn with (`ui/PLAN.md`): a 2D renderer on the Direct3D 9 device, SDF text from a
  vendored OFL font, immediate-mode widgets, input with capture, and hooks into a
  mission's frame, a front-end screen of our own and the loading screen. Draws nothing
  unless a `[Demo]` key is on; not in `./install` or `a2mod` yet (ui 0.1.0).
- Scenario `ui-runtime` (testbench 3.20.0).

Confirmed in game 2026-10-17.

## 12.9.0 — 2026-10-16

Layers: films 1.3.0.

### Added
- `films/origin` shots `tr15` (the tractor beam's cone) and `sh14` (shield hits)
  (films 1.3.0).

Tooling only; nothing to see in game.

## 12.8.1 — 2026-10-16

Layers: render3d 0.6.1.

### Fixed
- `render3d/build.sh` no longer shares a fixed temp file between installs running at
  once (render3d 0.6.1).

Tooling only; nothing to see in game.

## 12.8.0 — 2026-10-16

Layers: render3d 0.6.0, testbench 3.19.1.

### Changed
- Tractor and assimilation beams as a translucent cone; shield hits with mist, Klingon
  rays and light on the struck hull (render3d 0.6.0; 85c80d1, 424abef, 7cb274a, 8336f62).

Confirmed in game 2026-10-17.

## 12.7.0 — 2026-10-16

Layers: render3d 0.5.0.

### Changed
- Cloaking: wave-swayed transition, a drifting shimmer while cloaked, and the player's
  own cloaked ships at 10% hull (render3d 0.5.0; 7941b19).

Confirmed in game 2026-10-17.

## 12.6.0 — 2026-10-16

Layers: films 1.2.0.

### Added
- The `films/origin` board: an Akira class, the Metreon nebula and the assimilation
  beam, for the origin video (films 1.2.0; 522f9b7).

Tooling only; nothing to see in game.

## 12.5.0 — 2026-10-16

Layers: films 1.1.0.

### Changed
- `a2film` re-films a take once the code it shows has changed since its commit, not
  only when its spec changes, and says why a take is stale; `--ignore-code` keeps such
  takes (films 1.1.0; 3937489).

Tooling only; nothing to see in game.

## 12.4.0 — 2026-10-16

Layers: render3d 0.4.0.

### Changed
- render3d ray-marches the nebulae's gas at full size; `[Nebulae]` `Steps=`,
  `Denoise=` and `Footprint=` added, `Resolution=` and `EdgeRefine=` removed
  (render3d 0.4.0; d367dca).
- render3d adds the marched gas up over frames, which takes out its grain;
  `[Nebulae]` `Temporal=`, `TemporalClip=` and `Jitter=` added (render3d 0.4.0;
  9a54a5f, 65b182b, a9da7c5, 0e2f6ed).

### Fixed
- render3d's marched gas no longer slides as the RTS camera pans (render3d 0.4.0; 316df23).

Confirmed in game 2026-10-17.

## 12.3.0 — 2026-10-16

Layers: render3d 0.3.0.

### Changed
- The procedural skies are fitted to their stock skies' shape (clump size, edges,
  tones) by measurement, and graded to stock again, Aqua and `mbgrg` included; new
  recipe key `Octaves=` and tool `render3d/perf/sky-shape.py` (render3d 0.3.0; a40a752).

Confirmed in game 2026-10-17.

## 12.2.0 — 2026-10-16

Layers: render3d 0.2.1, testbench 3.19.0.

### Changed
- The procedural skies lose their grain, dust and draw-time fine detail, and their
  grades are refitted to stock without them (render3d 0.2.1; 712a09a).

### Added
- Bench scene `sky`: the sky alone (testbench 3.19.0; 712a09a).

Confirmed in game 2026-10-17.

## 12.1.0 — 2026-10-16

Layers: films 1.0.0, testbench 3.18.0.

### Added
- `films/` and `./a2film`: films written as storyboards (shots, camera, layout,
  transitions, titles, music), probed on the test bench at 1280x720 four games at once,
  filmed at 4K, and cut into a rough cut and an edit manifest for an editor (films
  1.0.0). The `film` skill holds the procedure. Installs nothing.
- `a2test session start --assets none`, the game as a release installs it (testbench
  3.18.0).

Tooling only; nothing to see in game.

## 12.0.0 — 2026-10-15

Layers: render3d 0.2.0, models 5.0.0, testbench 3.17.0.

### Removed
- The per-part plugins render3d replaces, and their layers: `msaa`, `lighting`, `sky`,
  `nebulae`, `surfaces`, `explosions`, `shields`, `repair`, `venting`, `specials` and
  `fog`, and models' `Planets.asi` (models 5.0.0). `a2mod` no longer switches them as
  layers; a stale copy is filed under render3d. Their recipes moved into `render3d/`
  (render3d 0.2.0).
- The release package no longer carries `MSAA.asi`, `Planets.asi`, `Lighting.asi`,
  `Sky.asi` or the sky recipes, nor the `lighting`, `sky` and `msaa` steps; its
  installers take out those plugins and the old `Sky\mbg*.ini` left by an earlier
  release. render3d is not in the package yet, so a release draws the 3D view stock.

### Changed
- `./install` installs render3d and the models' SOD rewrites with no plugin to skip;
  `render3d/install.sh` takes out any old plugin an earlier install left. Scenarios that
  tested the plugins check render3d's modules (testbench 3.17.0).

Confirmed in game 2026-10-17.

## 11.41.0 — 2026-10-15

Layers: render3d 0.2.0, testbench 3.17.0.

### Added
- render3d draws the 3D view as its own sequence of steps, its modules taking over
  every part from the engine and the plugins: hulls, effects (explosions, shield hits,
  repair welding, debris, construction, worker bees, sparks), ordnance (shots,
  special weapons, mines, pods, mining beams, the warp flash), the sky, the fog of war, grid, selection, height posts and
  the build ghost, particles and vents, the Evolver and holograms, planets, moons,
  asteroids and nebulae, and the special-object effects (render3d 0.2.0).
- render3d's `Timing=1` and `render3d/perf/compare.sh`: render3d measured against the
  plugins it replaces; it now takes less CPU than they did (`render3d/PERF.md`).

### Changed
- `./install` runs `render3d/install.sh` last, which takes out the plugins its modules
  replace (removed from the repository in 12.0.0). Scenario `no-assets` checks the modules instead; scenario `render3d-match`
  plays a match with a battle on render3d, and `render3d-mode-switch` changes the
  display mode in one (testbench 3.17.0).

### Fixed
- render3d draws the ships and stations whose ODF names another SOD (`baseName`):
  Martok's Negh'Var, Caddebostan's ship, the Ferengi scavenger, the 8472 hero mothership,
  Martok's base and the prison had stock's light and cloak (render3d 0.2.0).

Confirmed in game 2026-10-17.

## 11.40.0 — 2026-10-15

Layers: render3d 0.1.0, testbench 3.16.0.

### Added
- `render3d/`: `Render3D.asi`, the 3D view through a renderer of our own, in Rust. The
  engine's steps (and every plugin hooked into them) draw into a half-float target with
  MSAA, which is bloomed by energy and tone-mapped into the back buffer (render3d 0.1.0).
  `a2mod` switches it as the `render3d` layer.

### Changed
- `./install` runs `render3d/install.sh` in place of `msaa/install.sh`, and removes
  `MSAA.asi`: render3d's scene target has samples of its own.
- The bench gathers `Render3D.log`; scenario `render3d` (testbench 3.16.0).

Confirmed in game 2026-10-17.

## 11.39.1 — 2026-10-13

Layers: surfaces 1.1.1.

### Changed
- The class planets' recipes graded to stock's colours, measured on the bench in the
  same light, with their detail unchanged: pastel J, turquoise M, even orange L, violet
  patches on K, paler H; cloud on K, more on H and L, as stock's shell has (surfaces 1.1.1).

Confirmed in game 2026-10-30.

## 11.39.0 — 2026-10-13

Layers: venting 1.0.0, testbench 3.15.0.

### Added
- `venting/`: `Venting.asi` draws the plasma and gas a damaged ship vents in shaders, in
  place of the stock sprites: billowing puffs, plasma cooling from white-hot to red, gas
  pale to blue, a glow and jet core at each vent, one unbroken trail behind a moving
  ship, and a subtle flickering light on the hull about each vent (venting 1.0.0).
  `./install` and `a2mod` carry it as the `venting` layer.
- The bench's `Vent=` key and `vent` command show a system's damage node on a healed
  craft; `SCENE=venting` and scenario `venting` (testbench 3.15.0).

Confirmed in game 2026-10-30.

## 11.38.0 — 2026-10-13

Layers: sky 1.3.0.

### Changed
- The procedural skies colour-graded to their stock sets (`GradeGain=`/`GradeCurve=`,
  fitted by `sky/grade.py`), with fine detail finer than the baked cube added at draw
  time (`Fine=`), a supersampled bake, and a dither that no longer draws a grid
  (sky 1.3.0).

Confirmed in game 2026-10-30.

## 11.37.0 — 2026-10-13

Layers: lighting 1.24.0, testbench 3.14.0.

### Added
- Cloaked ships look like thick glass: what is behind them is bent along the hull, with
  colour fringes at the edges and a slow ripple, and while cloaking the hull goes patch by
  patch behind a glowing front (`Cloak=1` and the `Cloak*` keys; lighting 1.24.0).
- `SCENE=cloak`, the scene command `cloak`, and the scenario `cloak` (testbench 3.14.0).

Confirmed in game 2026-10-30.

## 11.36.0 — 2026-10-13

Layers: specials 1.0.0, shields 1.2.0, testbench 3.13.0.

### Added
- The `specials` layer: `Specials.asi` draws the warp flash by shader (a white-hot core
  and blue glow with a lens streak, bursting as the craft goes and leaving a ring), and
  the special weapons' shots and waves from recipes in `Specials.ini`: beams, bolts and
  bands of light running out, each lighting the hulls about it (specials 1.0.0).
  `./install` runs `specials/install.sh`; `a2mod` switches `Specials.asi` and
  `Specials.ini` as the `specials` layer. Not in the release package yet.
- The special weapons' shield shells, the corbomite reflector's among them, drawn by
  `Shields.asi` (shields 1.2.0).
- `Scene.asi` fires special weapons (`special`, `energy`), `SCENE=specials`, scenario
  `specials`; the bench gathers `Specials.log` (testbench 3.13.0).

Confirmed in game 2026-10-30.

## 11.35.0 — 2026-10-13

Layers: surfaces 1.1.0, lighting 1.23.0, models 4.3.0, testbench 3.12.0.

### Added
- The asteroids are smoothed (`models/asteroid-sod.py`; models 4.3.0) and drawn from
  recipes, `Master1` and `Master2`: cratered rock baked into cubes, looked up triplanar,
  lit per pixel with its own relief (surfaces 1.1.0, lighting 1.23.0).
- Recipe keys `Face=`, `Mapping=` and `Gloss=` (surfaces 1.1.0).
- `SCENE=asteroids` on the bench (testbench 3.12.0).

### Changed
- A model with a recipe is looked up about its mesh's centre, scaled to its extents,
  and its relief no longer scales with its size (lighting 1.23.0).

Confirmed in game 2026-10-30.

## 11.34.1 — 2026-10-13

Layers: nebulae 1.5.1.

### Changed
- The Cerulean nebula's swirl a little more noticeable: `Swirl=` 18 → 24 (nebulae 1.5.1).

Confirmed in game 2026-10-30.

## 11.34.0 — 2026-10-13

Layers: lighting 1.22.0, nebulae 1.5.0.

### Changed
- Nebulae light ships in the colour of their gas (the Cerulean a light blue, not pure
  blue), and less brightly: `NebulaRecipeColour=1`, `NebulaBrightness` 2.0 → 1.4,
  through `Nebulae.asi`'s new export `NebulaLightColour`.

Confirmed in game 2026-10-30.

## 11.33.0 — 2026-10-13

Layers: nebulae 1.4.0.

### Added
- The Cerulean nebula's specks wander in slow, roughly circular loops, each out of step
  with its neighbours: `Swirl=` (nebulae 1.4.0).

Confirmed in game 2026-10-30.

## 11.32.1 — 2026-10-13

Layers: nebulae 1.3.1.

### Changed
- The Metreon nebula's lightning flashes half as often (nebulae 1.3.1).

Confirmed in game 2026-10-30.

## 11.32.0 — 2026-10-13

Layers: nebulae 1.3.0.

### Added
- Nebulae with softer, wispier outlines that shift slowly: `Fringe=` (a thin haze about
  the body, on the Radioactive and Metreon nebulae) and `Fuzz=` (the outline feathered by
  a drifting fine noise); the flow changes the gas's shape instead of wobbling it
  (nebulae 1.3.0).

Confirmed in game 2026-10-30.

## 11.31.1 — 2026-10-13

Layers: shields 1.1.1.

### Changed
- The shields-down flash at half strength and the arcs a little sparser (shields 1.1.1).

Confirmed in game 2026-10-30.

## 11.31.0 — 2026-10-13

Layers: lighting 1.21.0.

### Added
- Planets have an atmosphere past the limb, a glow lit by the Key (`PlanetHalo=`,
  `PlanetHaloHeight=`, `PlanetHaloForward=`; lighting 1.21.0).

### Fixed
- The dark outline round planets, most visible against the volumetric nebulae: the
  cloud shell no longer writes depth (lighting 1.21.0).

Confirmed in game 2026-10-30.

## 11.30.2 — 2026-10-13

Layers: nebulae 1.2.1.

### Fixed
- Ships' shadows in the gas read at both of the gas's two points per slice pixel, so a
  shaft seen along its length is one streak, not a copy of the hull's silhouette per
  slice (nebulae 1.2.1).

Confirmed in game 2026-10-30.

## 11.30.1 — 2026-10-13

Layers: sky 1.2.3.

### Changed
- The green sky `mbgrg` (Klingon mission 1 and four other maps) is dimmer and its gas
  soft-edged instead of hard clumps (sky 1.2.3).

Confirmed in game 2026-10-30.

## 11.30.0 — 2026-10-13

Layers: fog 1.1.0.

### Added
- Stock's darker rim along the fog's and the shroud's edge, back over the smooth edge:
  `FogRim=`, `ShroudRim=`, `RimWidth=` in `Fog.ini` (fog 1.1.0).

Confirmed in game 2026-10-30.

## 11.29.0 — 2026-10-12

Layers: fog 1.0.0, testbench 3.11.0.

### Added
- The `fog` layer: `Fog.asi` draws the fog of war and the shroud in the 3D view with
  smooth edges, through a pixel shader over the engine's own draw of each, in place of
  the cell-by-cell staircase (fog 1.0.0). `./install` runs `fog/install.sh`; `a2mod`
  switches `Fog.asi` and `Fog.ini` as the `fog` layer. Not in the release package yet.
- The `fog` scene and the `fog-edges` scenario; `no-assets` checks `Fog.log`
  (testbench 3.11.0).

Confirmed in game 2026-10-30.

## 11.28.0 — 2026-10-12

Layers: nebulae 1.2.0.

### Added
- Hulls in or before the gas keep exact edges: `EdgeRefine=1` draws the gas at full size
  along depth edges and at half size elsewhere (nebulae 1.2.0).

Confirmed in game 2026-10-30.

## 11.27.0 — 2026-10-12

Layers: shields 1.1.0, repair 1.0.0, testbench 3.10.0.

### Added
- `Shields.asi` draws the shields-down ring too: jagged blue-white arcs crackling over
  the ship's shield ellipsoid where stock turns a band of lightning texture, and the
  brightest discharge lights the hull (shields 1.1.0).
- The `repair` layer: `Repair.asi` draws the welding on a ship under repair in shaders
  (a flickering arc with rays, a cooling hot spot, sparks streaking off the hull) in
  place of the stock spark sprites, and each arc lights the hull about it (repair 1.0.0).
  `./install` runs `repair/install.sh`; `a2mod` switches `Repair.asi` and `Repair.ini`
  as the `repair` layer. Not in the release package yet.
- The `shields-down-repair` scenario; `no-assets` checks `Repair.log`; the bench clears
  and gathers `Repair.log` (testbench 3.10.0).

Confirmed in game 2026-10-30.

## 11.26.1 — 2026-10-12

Layers: explosions 1.0.1.

### Changed
- The shock ring matches stock's shockwave disc in size, brightness and colour: a broad,
  overdriven orange band with a dark centre in place of a thin rim (explosions 1.0.1).

Confirmed in game 2026-10-30.

## 11.26.0 — 2026-10-12

Layers: shields 1.0.0, testbench 3.9.0.

### Added
- The `shields` layer: `Shields.asi` draws the ordinary shield hits in a shader over
  the struck ship's shield ellipsoid (a hot spot, a ring, a soft glow and the rim lit
  about each hit, by faction, weaker as the shields run out) in place of the stock
  shell (shields 1.0.0). `./install` runs `shields/install.sh`; `a2mod` switches
  `Shields.asi` and `Shields.ini` as the `shields` layer. Not in the release package
  yet.
- The `shields` scene and the `shields-hits` scenario; `no-assets` checks
  `Shields.log` (testbench 3.9.0).

Confirmed in game 2026-10-30.

## 11.25.0 — 2026-10-12

Layers: explosions 1.0.0, testbench 3.8.0.

### Added
- The `explosions` layer: `Explosions.asi` draws each ship's and station's fireball as a
  ray-marched volume of gas and each shockwave as a ring of light, in place of stock's
  sprites and disc; render only (explosions 1.0.0). `./install` runs
  `explosions/install.sh`; `a2mod` switches `Explosions.asi` and `Explosions.ini` as the
  `explosions` layer. Not in the release package yet.
- `Scene.asi` `kill`, the `explosion` scene and the `explosions` scenario
  (testbench 3.8.0).

Confirmed in game 2026-10-30.

## 11.24.0 — 2026-10-12

Layers: lighting 1.20.0, testbench 3.7.0.

### Added
- Torpedo and pulse impacts light the hull or shield they strike (`Impacts=` and the
  `Impact*` keys in `Lighting.ini`; lighting 1.20.0).
- The `impacts` bench scene (testbench 3.7.0).

Confirmed in game 2026-10-30.

## 11.23.0 — 2026-10-12

Layers: nebulae 1.1.0, lighting 1.19.0.

### Added
- The nebula gas is lit by the Key, scattering it forward, and ships cast the Key's
  shadow through it (nebulae 1.1.0, `KeyShare=`, `Scatter=`, `Shadows=`,
  `ShadowSoftness=`), from `Lighting.asi`'s Key and shadow map, now exported as
  `LightingKey` and `LightingShadowMap` (lighting 1.19.0).

Confirmed in game 2026-10-30.

## 11.22.9 — 2026-10-12

Layers: lighting 1.18.2.

### Fixed
- Hull shadows (`Shadows=1`) are made again: since lighting 1.17.2 every opaque hull
  counted as blended (the engine's ONE / ZERO blend) and the shadow map was never drawn.
  Hulls under a mirrored matrix (the Steamrunner, the Intrepid) are drawn front faces
  out in the shaders, which was what had darkened them (lighting 1.18.2).

Confirmed in game 2026-10-30.

## 11.22.8 — 2026-10-12

Layers: lighting 1.18.1, platform 3.4.1, sky 1.2.2, nebulae 1.0.3, surfaces 1.0.1.

### Fixed
- `Lighting.asi`: view slots for new cameras, the near fade's state block and the bump
  declaration tied to their device, paired vtable patches all or nothing, the glow and
  phaser colour caches no longer fill up, and the shadow lists turn over with no Key
  light (lighting 1.18.1).
- `d9_device()` tells a new device by the d3d9 device, not the d3d8 device's address
  (platform 3.4.1); `Sky.asi`, `Nebulae.asi` and
  `Surfaces.asi` rebuilt with it (sky 1.2.2, nebulae 1.0.3, surfaces 1.0.1).

Confirmed in game 2026-10-30.

## 11.22.7 — 2026-10-12

Layers: nebulae 1.0.2.

### Fixed
- Nebulae: the blue-noise dither thresholds read linear, not as sRGB (nebulae 1.0.2).
  Changes the look slightly.

Confirmed in game 2026-10-30.

## 11.22.6 — 2026-10-12

Layers: nebulae 1.0.1, sky 1.2.1.

### Fixed
- Nebulae: classes forgotten when the game frees its own (wrapping
  `GameObjectClass_Clean`); failed builds latched, falling back to the billboards; no
  composite without gas; alpha test off in the half-size passes; viewport restored
  after a bake; timing queries; a log buffer (nebulae 1.0.1).
- Sky: viewport restored after the bake; timing queries (sky 1.2.1).

Confirmed in game 2026-10-30.

## 11.22.5 — 2026-10-12

Layers: online 0.6.1.

### Fixed
- Online multiplayer hardening: malformed or hostile datagrams can no longer crash or
  exhaust a game, a kicked joiner is cut off, the network thread stops for good on
  `Close`, and the server rate-limits join-code lookups (online 0.6.1).

Confirmed in game 2026-10-30.

## 11.22.4 — 2026-10-12

Layers: qol 1.6.1, grid 1.0.1.

### Fixed
- `QOLRules.asi`: losing a producer refunds its item in progress once, officers included
  once; the network rules latch is cleared at every game start; a delete order still
  finds an item that started while it was on the way (qol 1.6.1).
- `QOL.ini` refuses numbers too long to parse (qol 1.6.1).
- `GridLayout.asi`: keys press nothing while the bar is hidden (grid 1.0.1).

Confirmed in game 2026-10-30.

## 11.22.3 — 2026-10-12

Layers: menus 4.3.3.

### Fixed
- Menus: `IntegerScale=1` in a window under 800x600, the backdrop's sides after an
  unmatched frame, embedded nested dialogs' fit, 8K stretch taps, `N.soften=` bounds,
  surfaces released on destroy; `menus/install.sh` always rebuilds and its closing check
  no longer stops `./install` (menus 4.3.3).

Confirmed in game 2026-10-30.

## 11.22.2 — 2026-10-12

Layers: hud 2.2.1, msaa 1.1.1.

### Fixed
- `HUD.asi` lays the HUD out with `Font=0` or the font standing down, and hands no
  short objective list across a re-layout (hud 2.2.1).
- `MSAA.asi` finds 4x for `Samples=6` and other counts that are not a power of two
  (msaa 1.1.1).

Confirmed in game 2026-10-30.

## 11.22.1 — 2026-10-12

Layers: cutscenes 2.0.2.

### Fixed
- `binkw32.dll`: a replacement movie bigger than its destination outside the fit path
  is scaled to fit instead of written past the buffer; a `binkw32_orig.dll` missing an
  export no longer leaves null pointers behind; an over-long movie path and a `.wav`'s
  inconsistent block align are handled (cutscenes 2.0.2).

Confirmed in game 2026-10-30.

## 11.22.0 — 2026-10-12

surfaces 1.0.0, lighting 1.18.0, testbench 3.6.0

### Added
- `surfaces/`: `Surfaces.asi`, planet surfaces (classes D, H, J, K, L, M) and the
  dilithium moon made from recipes as height fields baked into cubes, with relief,
  craters, fissures, oceans, ice and drifting clouds with shadows; drawn by
  `Lighting.asi`. `./install` runs `surfaces/install.sh`; `a2mod` switches it as the
  `surfaces` layer.

Confirmed in game 2026-10-30.

## 11.21.0 — 2026-10-12

Layers: sky 1.2.0.

### Added
- Detail in the procedural skies: grain, dust, haze and embedded stars as recipe keys,
  used by every sky recipe; `mbgblue` calibrated like `mbg02` (sky 1.2.0). `d1d15e2`

Confirmed in game 2026-10-30.

## 11.20.0 — 2026-10-12

Layers: gui-installer 1.5.0.

### Added
- The graphical installer for Windows and macOS (gui-installer 1.5.0): a portable
  `armada2-refit-installer-windows.exe` for the plain GOG install, and
  `armada2-refit-installer-macos.zip`, a universal2 app for the game in CrossOver or
  another Wine, writing the DLL overrides into the bottle's registry. Every release
  carries both beside the AppImage.
- The release zip's `install.sh` runs on macOS (`/bin/bash` 3.2, BSD tools) into a Wine
  prefix: crosire's d3d8to9 on Wine's own Direct3D 9, never DXVK's (one an earlier
  install put in comes out); MSAA, `dxvk.conf` and bloom skipped; the game found in a
  CrossOver bottle. `A2_PYTHON` names the Python for the SOD rewrites on every system.
- `manifest.json` names `windows` and `macos`; the Windows and macOS installers install
  only packages that do.

### Changed
- `install.ps1` does what `install.sh` does: QOLRules, GridLayout (the player's
  `GridLayout.ini` kept), Planets and the SOD rewrites (`A2_PYTHON`, `py -3` or a real
  `python`); crosire's d3d8to9 in place of GOG's `d3d8.dll`, on Windows' own Direct3D 9
  (a DXVK `d3d8.dll` is left as it is), recorded so `-Uninstall` restores GOG's;
  `A2_PROGRESS` and `A2_SKIP`; every line in order on stdout, and a plain last-line
  error with exit 1.
- `install.sh` needs no `sha256sum`, `tac` or GNU `sed`; `publish/installer/test.sh`
  runs on macOS too and, on Linux, again without those tools and again as Darwin.
- CI builds and tests the `.exe` on windows-2025 and the `.app` on macos-15, and runs
  `install.sh`'s test on macOS.
- The zip's `README.txt` names the graphical installers and the macOS way.

Confirmed in game 2026-10-30.

## 11.19.0 — 2026-10-10

Layers: nebulae 1.0.0.

### Added
- The `nebulae` layer: `Nebulae.asi` draws each nebula class with a recipe, latinum
  included, as a volume of gas in place of its billboards, with recipes for every class
  (nebulae 1.0.0). `./install` runs `nebulae/install.sh`; `a2mod` switches
  `Nebulae.asi`, `Nebulae.ini` and `Nebulae/*.ini` as the `nebulae` layer. Not in the
  release package yet.

Confirmed in game 2026-10-10.

## 11.18.0 — 2026-10-10

Layers: platform 3.4.0.

### Added
- `platform/heroic-umu.py --on` bounds umu's Steam-runtime check to 3 s, with no
  retries, in the game's Heroic config (platform 3.4.0). Run by hand; not in `./install`
  or the release installers.

Confirmed in game 2026-10-30.

## 11.17.1 — 2026-10-09

Layers: gui-installer 1.4.1.

### Fixed
- The installer AppImage no longer prints fontconfig warnings at start: it uses the
  host's fontconfig and freetype (gui-installer 1.4.1).

Installs nothing into the game. Confirmed in game 2026-10-09.

## 11.17.0 — 2026-10-09

Layers: gui-installer 1.4.0.

### Changed
- The graphical installer has a procedural nebula behind its stars that goes to warp
  while installing, an amber accent, a bolder title, a Launch game button and a
  progress bar with an engine at its tip, always writes Heroic's launch settings, says how
  to get Heroic when there is none, and drops the remastering subtitle and the textures
  note (gui-installer 1.4.0).

Installs nothing into the game. Confirmed in game 2026-10-09.

## 11.16.0 — 2026-10-09

Layers: gui-installer 1.3.0.

### Added
- The release zip carries the sky layer: `game/Sky.asi`, `game/Sky.ini` and the recipes in
  `sky/`, in `manifest.json` as the `sky` layer and step. `install.sh` and `install.ps1`
  put the recipes in the game's `Sky` folder, say when the chain gives the stock sky,
  and take out only their own recipes on `--uninstall`; `test.sh` and `test.ps1` check
  it (gui-installer 1.3.0 shows its step).

Installer tests pass on Linux (`publish/installer/test.sh`, `gui-installer/test.sh`);
`test.ps1` runs in CI. Not yet installed from a release.

## 11.15.0 — 2026-10-09

Layers: sky 1.1.0.

### Added
- Procedural recipes for every sky a stock map uses, and the recipe key `Band=` (sky 1.1.0).

Confirmed in game 2026-10-09.

## 11.14.0 — 2026-10-09

Layers: sky 1.0.0 (new).

### Added
- The `sky` layer: `Sky.asi` draws a map's sky from a recipe, in a shader, with no cube
  faces and so no seams; recipes for `mbgaqu` and `mbgkling` (sky 1.0.0). `./install`
  installs it and `a2mod` switches it.
- `no-assets` checks that `Sky.asi` patched its site.

Confirmed in game 2026-10-09.

## 11.13.0 — 2026-10-09

Layers: gui-installer 1.2.0.

### Added
- The graphical installer is released as an AppImage built in CI on ubuntu-24.04 and
  attached to each release beside the script (gui-installer 1.2.0).

Installs nothing into the game; the AppImage's window and its mock-game test checked on
the build machine, not yet seen on another distribution.

## 11.12.0 — 2026-10-09

Layers: gui-installer 1.1.0.

### Changed
- The graphical installer's window is rebuilt: one screen, neutral libadwaita over a
  starfield drawn in code, no LCARS, no layer table (gui-installer 1.1.0).

Installs nothing into the game; the window checked on a headless sway, not yet seen in
game.

## 11.11.0 — 2026-10-07

Layers: testbench 3.5.0.

### Added
- `Scene.asi`'s `Music=` key and `SCENE_MUSIC=0` at install: a bench session without the
  game's music, for footage (testbench 3.5.0).

Installs nothing; seen working on the bench 2026-10-07.

## 11.10.0 — 2026-10-07

Layers: testbench 3.4.0.

### Added
- Bench scenes for footage: `colonize`/`Population=`, `goto`/`stop`, and switches for
  tooltips, the hover gauges (`hover`/`Hover=`) and shield-hit flashes
  (`shieldfx`/`ShieldFx=`) in `Scene.asi` (testbench 3.4.0).

### Changed
- `Scene.asi`'s `Heal=` keeps shields, crew and systems full, and ships with their
  engines switched off no longer stream a damage plume; the `showcase` cubes hold fire
  (testbench 3.4.0).

Installs nothing; seen working on the bench 2026-10-07.

## 11.9.0 — 2026-10-07

Layers: testbench 3.3.0.

### Added
- Camera moves in bench scenes: `glide` and `spin` (`Scene.asi`), and the `showcase`
  scene, a fleet action for footage (testbench 3.3.0).

Installs nothing; seen working on the bench 2026-10-07.

## 11.8.0 — 2026-10-08

Layers: platform 3.3.0.

### Added
- The release zip's `install.sh` sets up the game as the repository's `./install` does,
  textures, menu backdrops and movies apart: the Direct3D chain (crosire's d3d8to9 on
  DXVK 3.1.1's d3d9, in `chain/`; a replaced GOG or DXVK DLL is kept and put back by
  `--uninstall`), `QOLRules.asi`, `GridLayout.asi`, `Planets.asi`, and the moon,
  selection-bubble and hull SODs rewritten from the player's own files (`models/`,
  needs python3). New `::step` ids `chain`, `grid`, `models` and manifest layers to match.
  The Windows installer is unchanged.

Confirmed in game 2026-10-09.

## 11.7.1 — 2026-10-08

Layers: menus 4.3.2.

### Fixed
- The main menu could open off-screen, leaving a black screen after the intro (seen on
  NixOS with GE-Proton10-32): an embedded full-screen menu is pinned to the game window.

Confirmed in game 2026-10-30.

## 11.7.0 — 2026-10-08

Layers: gui-installer 1.0.0 (new; reads package schemas 0 and 1), postfx 1.2.1.

### Added
- `gui-installer/armada2-refit-installer.py`, the graphical installer for Linux: finds the
  game, installs any GitHub release (cached locally, newest full release by default),
  writes Heroic's launch variables. In every release zip and as a release asset of its own.
- Package schema 1: `manifest.json` in the release zip (layers, versions, `::step` ids).
- `install.sh`: `A2_PROGRESS=1` prints `::step <id>` per stage; `A2_SKIP=cutscenes`
  leaves the cutscene player out and puts back a stock `binkw32.dll` an earlier install
  replaced.
- The release zip carries `Online.asi` + `Online.ini` (online 0.6.0), installed and
  uninstalled by `install.sh` and `install.ps1`.
- Without a vkBasalt layer, `install.sh`, `README.txt` and the GUI say how to get one,
  per distribution (Arch, Fedora, Debian, Ubuntu).

### Fixed
- `postfx/vkbasalt/build.sh` builds on Debian and Ubuntu (postfx 1.2.1).

Confirmed in game 2026-10-30.

## 11.6.2 — 2026-10-07

Layers: lighting 1.17.2.

### Fixed
- Hulls shadowed whole by a blended or alpha-tested draw above them under `Shadows=1`
  (lighting 1.17.2).

Confirmed in game 2026-10-07.

## 11.6.1 — 2026-10-07

Layers: lighting 1.17.1.

### Fixed
- Hulls going dark from frame to frame under `Shadows=1`: the shadow map is made per
  camera, not from whichever view rendered last (lighting 1.17.1).

Confirmed in game 2026-10-07.

## 11.6.0 — 2026-10-07

Layers: qol 1.6.0, online 0.6.0.

### Added
- The online handshake for QOLRules: pay-on-queue runs in an *Internet – Online* game when
  every player has it, and stands down otherwise.

Confirmed in game 2026-10-09.

## 11.5.0 — 2026-10-07

Layers: qol 1.5.0.

### Added
- `QOLRules.asi`: pay when queuing, refund on cancel (QOL-7), single player and skirmish
  only. `a2mod` and the bench's log collection know the new files.

Confirmed in game 2026-10-09: pay on queue and the refund on cancel; the refund on
losing the building and save/load not yet seen.

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

Confirmed in game 2026-10-30.

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

Confirmed in game 2026-10-30.

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
