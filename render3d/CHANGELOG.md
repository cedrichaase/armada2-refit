# Changelog — render3d

`Render3D.asi`: the 3D view drawn through render3d, in Rust. Versioning rules:
[`CLAUDE.md`](../CLAUDE.md), "Changelogs and versions". Newest first. The reasoning is
in [`README.md`](README.md), the plan in [`PLAN.md`](PLAN.md).

## 1.3.0 — 2026-11-06

### Added
- `[Overlays] MovePointColour`, `MovePointBrightness`, `MovePointOpacity`. `396bd4f`
- `r3d_core::files`: one `a2fmt::vfs` file system over the game directory, with the mount list `LAYOUT` (`textures` = `Textures\RGB`, `sod`, `sprites`, `odf`, and the recipe override folders `Sky`, `Surfaces`, `Nebulae`, `Menu`), held to `games/armada2.toml` `[files]` by `tools/layers.py --check`. A mounted archive that serves nothing logs `files: <archive> at <folder>: <why>` (none in Armada II).
- Content sets are data (AR-6, root 15.10.0): `render3d/content/sets/<set>/<module>.ini`, one ini table per module (`effects`, `hulls`, `ordnance`, `overlays`, `particles`, `sky`, `specials`, `world`), built into the core with every recipe of `render3d/content/` (`core/build.rs`, `r3d_core::content`) and parsed by each `content.rs` at first use (`a2ini::Table`). A set is a folder: a new one needs no code; a set with no table for a module is its empty table (`NONE`). The stock set reproduces the old constants exactly (`hosttest/src/sets.rs`).
- effects' table names the Evolver's cocoon SOD (`CocoonSod`, was a literal in `chunks.rs`) and `[Shields] RayFactions=`'s default (`RayFactions = Klingon`, was in `shields.rs`); hulls' names the `[Hulls]` key prefix of the bump-mapped hulls' profile (`BumpProfile = Borg`: the `Borg*` keys as before; with no prefix those hulls take the hull profile). `b524e42`

### Changed
- Overlays: move paths are drawn for the selected ships only (the overview's `GetSelectList`, so a selection above 16 is read too; entries `ov_vt_select_num`, `ov_vt_select_list` in `profiles/gog-1.1/render3d.overlays.toml`).
- Overlays: the path is QOL.asi's queue when it exports one (`QOL_MoveQueueVersion`, `QOL_MoveQueue`: appended legs, a patrol run closed back to its first point), found by its export (any loaded module, not by file name) once a second until found; no sketch is drawn then. Without it, the engine's path (the "ends from a sketch" filter) and the sketch as before, for selected ships.
- Overlays: the line is finer (`MovePathDot` 2.5 -> 1.9, `MovePathSpacing` 9 -> 8) and very light grey (`MovePathColour` 0.85 0.85 0.85); the points keep their size and colour, now their own keys.
- Install: an installed `Render3D.ini` still holding the old line defaults (`MovePathColour=0.55 1.0 0.55`, `MovePathDot=2.5`, `MovePathSpacing=9`) gets the new ones (`tools/ini-merge.py` RETIRED); a value the player set stays.
- Render3D.log: `move paths: N selected ship(s) on a path, M point(s) ahead (qol|engine|-), sketch K`, and `QOL's queue found (version 1)` / `not found`. `396bd4f`
- Overlays, move paths: with QOL's `QOL_MoveQueueGroup`, one selected ship of a queue draws its own path to its own targets; several selected ships of one queue draw the group's path once (the queue's points) with each ship's own leg to where it heads and a small dot at each of its own targets. Render3D.log says `QOL's queue found (version 1, groups)`. `aeb6b41`
- The engine resolver (`crates/a2engine`, `csdk/a2engine.h`; root 15.8.0) names the executable in its log lines and notes by the matched build's `exe` (`profiles/*/build.toml`), or on an unknown build by the process's own module file name, not by a literal: `engine: Armada2.exe timestamp ...` on Armada II as before. The generated symbol tables carry each build's `exe`. Render3D.log's identity line is now the core table's, so it names the matched build's executable; it no longer formats its own.
- `assets` (SODs, textures and their chains, the sprite table, ODFs), `recipes` (only the read of the game folder's copy) and the world module's texture means open files through `files`, with no `Textures\RGB` literal left; stock finds the same files in the same order, and a recipe in the game folder still wins over the built-in one.
- `lint-layers.sh` rule 4 (content names outside `content.rs`) is strict and in CI with rules 1-3; an engine format literal stays with `// lint-allow(content): <reason>` (four, in overlays, sky and world).
- Engine fields and values come from the matched build at run time (`field::X()`, `value::X()`, crates/a2engine `Engine::field`/`value`), no longer compile-time consts of `gog-1.1`: a build whose structs differ gets its own offsets, and a hook whose field the build lacks stands down (PLAN-ARCHITECTURE.md, AR-3).
- core: the struct offsets, vtable offsets and sizes the core reads (`core/src/engine/**`, the render state and particle list of `frame/steps.rs`, the crew and Borg nodes of `scene/instance.rs`) are `field` and `value` entries of `profiles/gog-1.1/render3d.toml` (90, `core.reads`), read through `field::X()`; `DAMAGE_NODES` becomes `damage_nodes()`, `DEV_CLEAR_DEPTH`/`DEV_FLUSH` `dev_clear_depth()`/`dev_flush()`, `render_state_flag` `render_state`.
- particles: its struct offsets and the latinum resource id are `pt_*` entries of `render3d.particles.toml` (12, `particles.reads`); the offsets it shared with the core (the current instance, the entity handle, an instance's class and database, the system bytes, a list's layout, an element's name) are read through the core's helpers (`for_each_in_list_at`, `system_bytes`, `current_instance`), and the vents' engine lights through `light::new_point_light` and `set_reach`.
- sky: its offsets are `sky_*` entries of `render3d.sky.toml` (10, `sky.reads`): the starfield's octree and stars, the auto_ptr list's element, the edit-mode byte, the background instance's transform, the hidden class flag and the sky detail byte; the render state, the camera rectangle, an instance's scale and database, a class's `Render` and a sprite's name and colour (new: `sprite_colour`, 0x24) come from the core.
- specials: a timed effect's offsets, its weapon class's ordnance and cPrjID, the effect's `Render` slot and an instance's radius are `sp_*` entries of `render3d.specials.toml` (12, `specials.reads`); the effects list walks through the core (`for_each_list_node`, `list_node_value`).
- hulls: its offsets are `hl_*` entries of `render3d.hulls.toml` (43, `hulls.reads`): the instance's sphere, placement, origin, face limit and slots, the cloak, warp, waves, hologram, rig, scrap, Evolver and cocoon, the freighter's cargo and the resource count; what the core has (an object's instance, team, class and culled byte, the on-screen byte, an instance's database, clock and assimilated byte) is read through it (new: `obj::team`, `obj::culled`, `write_on_screen`). The two rig placeholder readers share `rig_item`.
- world: its offsets are `wd_*` entries of `render3d.world.toml` (43; `world.reads`, the cull's node sphere `world.cull`): the nebulae's lights, class and position, the ghosts' spirits, the planets' textures, colony, race, cities and atmosphere, the asteroid fields' rocks and the moons' dilithium; `planets.rs` reads the race's city name through `hooks::race_name`, and the lists, the frustum test, the on-screen byte, an instance's scale, clock and database come from the core (new: `for_each_in_vector`).
- overlays: its offsets are `ov_*` entries of `render3d.overlays.toml` (43, `overlays.reads`): the user profile's options, the terrain's rectangle, cells and layer textures, the scanner grids, the selection buckets (and their count), the ghosts' material and tint, Storm3D's ghost state, the spirits, the placeholder and the tactical camera's marker; team colours, an instance's scale and database and the ghost list come from the core (new: `team_rgb`).
- ordnance: its offsets are `or_*` entries of `render3d.ordnance.toml` (30, `ordnance.reads`): a shot's class, time left, handles, ends and colours, the class's light, life, sprite, sizes and cPrjID, the `Submit` slot, a point light's colour, a phaser's state, the bytes `RenderAll` tests, a warp effect's distance and a shockwave's class and radius; the entity's instance, flags, class and on-screen byte, a sprite's texture, rectangle and colour, the light registry and the list walk come from the core (new: `texture_name`, `rect`, `for_each_light_record`, `set_light_record_colour`, `render_slot`, `list_node_next`), and `lights.rs` makes its lights through `light::new_point_light` and `set_reach`.
- effects: its offsets are `ef_*` entries of `render3d.effects.toml` (69, `effects.reads`): the engine's map and list node layouts, an instance's placement, radius and origin, the explosions' slots, class, times and radii, `NodeParticleEffect's particles, the shield hits, the shield list's nodes, a shot's damage, a craft's shield transform and level, the bees, `BuildingEffect's `Info` and welding edges, and Storm3D's team colour; an object's flags, instance, class, handle and on-screen byte, an instance's database and a class's `Render` come from the core. `b524e42`

Confirmed in game 2026-11-06.

## 1.2.0 — 2026-11-05

### Added
- Overlays: move paths (issue #17). Each of the player's ships on a move or patrol path shows it as a row of fine light-green dots from the ship through the points to come, a bigger dot at each point, until flown: the part flown is gone (`paths.rs`, `vs_dots`/`ps_dots`). The path sketched with Tab is drawn the same way while Tab is held and stays drawn after; stock's one-pixel sketch line (`mDrawPath`'s call in `mNormalModeRender`) is re-pointed and draws nothing while ours are drawn. `[Overlays] MovePaths` (a live row; 0: stock's line), `MovePathColour`, `MovePathBrightness`, `MovePathOpacity`, `MovePathDot`, `MovePathPoint`, `MovePathSpacing`. The hook `render3d.overlays.paths` and its entries in `profiles/gog-1.1/render3d.overlays.toml`; Render3D.log says `move paths: N ship(s) on a path, M point(s) ahead, sketch K`.

Installed, not yet seen in game.

## 1.1.0 — 2026-11-05

### Added
- The menu scene: `Render3D_MenuLocked(names)`, the views whose campaigns are locked (UI.asi's campaign selection, issue #13). A locked view's group holds its ships cloaked with `LockedCloak=1`, drawn whole in the hulls' cloak glass as a fully cloaked hull is in a mission (`ShipAt::cloaked`; no decloak, no sound), puts each ship at its `LockedAt=` and turns it to its `LockedYaw=` (a ship's, or the group's added to each ship's `Yaw`); the built-in map holds the Klingons cloaked in the Metreon gas and sinks the Borg cube and the Diamond into the Mutara gas, the cube at `LockedYaw=70` so it reads as a cube (judged on the bench at 1280x720; two places further back read flatter and were not taken). Render3D.log: `menu: locked views`.

Confirmed in game 2026-11-05.

## 1.0.7 — 2026-11-05

### Fixed
- a2rt: the game directory, `%LOCALAPPDATA%` and every file path now go through the wide Win32 calls (`GetModuleFileNameW`, `GetEnvironmentVariableW`, `CreateFileW`, `FindFirstFileW`, ...; `wstr` / `from_wide`; the imports join `kernel32.def`). With a non-ASCII install path (`C:\Users\Jörg\...`) the ANSI call's code-page bytes were not UTF-8, the game directory came out empty, `RefitPlayer.ini` was never consulted (every Refit-screen setting saved and ignored) and was written to a relative path, and the log folder was lost. Installed, not yet seen in game. Bench-verified.

## 1.0.6 — 2026-11-05

### Fixed
- Menu scene: a decloaking group's `CloakSound` plays once per arrival, as its first ship's decloak begins, not once per ship (the Klingon campaign view played four uncloak sounds; issue #12); each one played logs `menu: decloak sound N <wav> (group <name>)`, and `render3d-menu-decloak-sound` checks one per arrival. Confirmed in game 2026-11-05.

## 1.0.5 — 2026-11-04

### Fixed
- Links the a2fmt that no longer panics on a TGA header whose size overflows, or hangs on self-including sprite tables (root 15.6.27). Installed, not yet seen in game.

## 1.0.4 — 2026-11-04

### Fixed
- `a2rt::move_file` (the settings overlay's rename) retries a failed move 8 x 50 ms; `Sleep` joins `kernel32.def`.
- `install.sh` (and `--remove`) no longer deletes `Sky\<name>.ini` by the names of the shipped recipes: a sky recipe of the player's own, the override the README documents, survives. The old Sky.asi copies are taken out by hash, by `retire/retire.sh` (platform 4.0.1). Installed, not yet seen in game.

## 1.0.3 — 2026-11-04

### Fixed
- effects: the engine's weld and spark sprites are dropped only while one of render3d's own views runs (`begin_view` to `end_view`), not whenever the module is on: with `Inset=0` the engine-drawn inset lost them. Debris taken over at the explosion (`Craft::Explode`) is simulation-time and still ours only; the inset with `Inset=0` shows none of it.
- Comments in `qol/` and `ui/src/engine/` list no instruction sequences (hard rule 9).

Installed, not yet seen in game.

## 1.0.2 — 2026-11-03

### Fixed
- Built on `gpu` 2.0.1 (PR #23 by bmh129 and its review): on Vulkan a frame that spends Display8's descriptor pool, or the storage sets' pool, goes on in pools of gpu's own (a chain per frame slot, up to 64 pools) instead of dropping its draws and dispatches; a frame that found the pool spent goes straight to gpu's own pools, and the chain grows only on a spent or fragmented pool. The spent-pool log line names the Vulkan error.

Installed, not yet seen in game.

## 1.0.1 — 2026-11-03

### Changed
- `Render3D.ini`: the `[Modules]` comment says every module row is read again while the
  game runs (it listed only some). Comment only; no behaviour changes.

Installed, not yet seen in game.

## 1.0.0 — 2026-11-03

MAJOR: Render3D.asi no longer runs without Display8. With no Display8 of its contract,
one that has stood down to the system's Direct3D 8 (`Backend=none`) or a device it
cannot open, `gpu` is stood down and render3d leaves the 3D view to the engine; it
logs `frame: standing down: no Vulkan or Direct3D 12 device behind the engine's` once.

### Removed
- The Direct3D 9 path: `gpu::d3d9` (gpu 2.0.0), the engine glue that reached the
  game's d3d9 device (`Device::from_d3d8`, the raw COM tally, the unbinding in the
  `DestroyRenderSurfaces` hook, the menu scene's `SetViewport`), `gfx::reset` and
  `gfx::forget`. The `Reset` and `DestroyRenderSurfaces` hooks stay and drop what the
  modules made.
- The d3dbc form of every shader: `<crate>/src/shaders/<name>.rs` (vkd3d's bytecode) is
  gone; the shaders are `<name>_spv.rs` and `<name>_dxil.rs`, byte for byte the SPIR-V
  and DXIL they were (only the stamps moved). `gpu::code!` takes two names, three with
  `_AT_DXIL`. `tools/shaderc`: `d3dbc-fix.py`, `d3dbc-slots.py`, `fetch-vkd3d.sh`, the
  vkd3d step of `hlsl-rs.sh` and `verify-generated.sh`, `--fix`, and the `:long` and
  `:dxc` marks of an entries line (CI's `shaders` job needs dxc and spirv-val only).

### Changed
- `gpu::Gpu::attach_engine` and `engine_id` take no device; `gpu::format_code` replaces
  `gpu::d3d9::format_code` (same numbers, so logs and keys read as before).

Installed, not yet seen in game. Pixel gate: `port-showcase`, `port-nebula` and
`port-firing` against `roundD-base-*` at RMSE 0 (run 20261103-145601), `display-passthrough` at RMSE 0 against `p10-vk` and
`display-passthrough-d3d12` at 0.479 (run 20261103-151214); `display-bail-out` and
`display-fallback` show the stand-down lines; `render3d-settings-live`, `ui-menus-refit` and
`no-assets` pass (run 20261103-151336).

## 0.47.0 — 2026-11-03

### Added
- Publishes its scene pass on Direct3D 12 too (gpu 1.3.0, contract 1.4: `display8v1_dx12_pass`, PASS_ENTER at each `Cmd`'s start, PASS_PUBLISH with the pass at its end), as on Vulkan, for Display8's passthrough there. Against a Display8 of contract 1.3 or older it publishes nothing, as before. The picture is unchanged. Installed, not yet seen in game.

## 0.46.0 — 2026-11-03

### Added
- `View.Distance` (`[View] Distance`, `Render3D.ini`; ENUM, applies with the next mission, LOCAL, `moved_from = "QOL.QOL.ViewDistance"`) and the profile hook `view.distance`: `core/src/engine/viewdist.rs` raises the far plane and the object culling distance after ART_CFG.h's parse, live through `Render3D_SettingsChanged`. With `Enable=0` the patch is not installed (QOL's was). W-VIEW. Installed, not yet seen in game.

## 0.45.0 — 2026-11-03

### Changed
- The modules' built-in tables of game names (`content.rs` of effects, hulls, ordnance, overlays,
  particles, sky, specials and world) are per content set, chosen by `content::get()` from
  `Engine::content()` (phase 8f, `profiles/gog-1.1/content.toml`); the `stock` set is the
  tables as they were, so the look is unchanged. A build with no set gets no built-in names or
  recipes and the engine's own look. `overlays` now compiles its `content.rs`. Installed, not
  yet seen in game.

## 0.44.1 — 2026-11-03

### Changed
- `Render3D_Settings` and the modules' settings tables are regenerated at A2SET_ABI 1.1 (48-byte rows; `moved_from` NULL on every row). Nothing else changes. Installed, not yet seen in game.

## 0.44.0 — 2026-11-03

### Added
- Render3D.asi on its engine profile (phase 8e): `profiles/gog-1.1/render3d.toml` (core and
  frame) and `render3d.<module>.toml` per module, with the generated `syms.rs` in
  `core/src/` and each module crate. Every address of `Armada2.exe` is a profile entry read
  through `at(sym::X)`; `Hooks::ready` checks a hook's entries before its owner reads or
  writes, `Site::of` builds a hook's site from the profile. Hooks: `core.reads`,
  `frame.seam`, `frame.steps`, `frame.destroy`, and per module `<m>.reads` plus
  `effects.{repair,shields,debris}`, `ordnance.{warp,shots}`, `overlays.posts`,
  `particles.{particle,billboard}`, `world.{lights,cull,ghosts}` (README, "Hooks").
- Render3D.log names the executable and the build (`engine: ...`, `engine: <part> symbols: N/M`).
- A hook that stands down feeds `Render3D_SettingsNote`; an unknown `Armada2.exe`, or a
  core or seam hook not ready, stands render3d down whole; a module not ready is left to
  the engine (forcing `render3d.hulls.*` equals `[Modules] hulls=0`).

### Changed
- The core's 87 engine reads outside the glue moved into `core/src/engine/` (`view.rs`);
  `lint-layers.sh` rule 1 is strict for the core and the modules (CI).
- The engine's addresses that were `pub const` in `r3d_core::engine` are lowercase fns.
- Effects' debris hooks are all-or-none (the cancelled-construction site was optional).
- Installed, not yet seen in game

## 0.43.0 — 2026-11-03

### Changed
- The crates (README.md, "Crates"): render3d is a workspace of a thin cdylib
  (`render3d`: DllMain, allocator, panic handler, exports), `r3d-core` (`core/`: frame,
  gfx, scene, assets, the shared engine glue, the menu scene, `module`, `registry`,
  `hooks`, `services`, `recipes`, `particles`) and one `r3d-<m>` crate per module
  (`mods/<m>/`), each depending on the core and on no other module. The staging names
  `core_module`, `core_hooks`, `core_services`, `core_recipes` and `core_particles` are
  `r3d_core::module`, `hooks`, `services`, `recipes` and `particles`; the registry is
  `r3d_core::registry`. Each crate compiles its own `shaders/`; stock's sprite shaders
  (`vs_sprite`/`ps_sprite`) moved from `ordnance.hlsl` into the core's `sprites.hlsl`,
  since the core draws them. Every generated shader array is word for word as before.
- `tools/settings`: the core's rows are generated into `core/src/settings_table.rs`
  (`rows_in = "core"`); `Render3D_Settings` is the cdylib's.
- The `[Lights]` scene keys (`Replace`, `KeyColour`, `KeyAxis`, `FillColour`,
  `FillAxis`, `SkyLight`) are read by the core (`scene/scene_lights.rs`, was the hulls
  module's): they light every module, so a build without hulls keeps them.

### Added
- One Cargo feature per module, all on by default (the DLL that ships has every one):
  `build.sh --features hulls,sky` builds a subset into `build/subset/Render3D.asi`. A
  module not built in has no rows in `Render3D_Settings` (neither its fragment's nor
  its `[Modules]` row), and Render3D.log says `modules not built in: ...`.
- `bench-asi.sh` (a clone's Render3D.asi from this working copy, full or
  `--without`/`--features`, from a `Setup:` line) and `subset-scenario.sh`
  (`render3d-build-subset`'s variant for another module).

Installed, not yet seen in game.

### Changed
- The frame: `MSAA` and `HDR` are `apply = "live"`. `Render3D_SettingsChanged` swaps a
  new `Config` in (`config::reload`) and, when either moved, `frame::formats_changed`
  drops the scene targets and post passes and calls every present module's
  `formats_changed`; the next view makes them again. `Bloom`, `BloomThreshold`,
  `BloomKnee`, `Exposure` and `Shoulder` follow live as well. The settings log line
  names them (`Render3D.MSAA=0`).
- The menu scene's map recipe moved from `render3d/menus/map.ini` to
  `render3d/content/menu/map.ini` (still built in; `Menu\map.ini` in the game
  directory still takes precedence). `install.sh` takes every recipe from
  `render3d/content/`.

Installed, not yet seen in game.

## 0.41.0 — 2026-11-03

### Changed
- effects and sky on the module layers: every engine read and hook of effects is in
  `effects/hooks.rs` (sub-modules per part) and of sky in `sky/hooks.rs`, tables in
  `content.rs` (effects: `SITE_NAME`, `FAC_KEY`, `FAC_DEF`; sky: the built-in recipes,
  now `include_str!`d from `render3d/content/sky/`, moved from `render3d/skies/`, and
  the point-star texture), `render3d/lint-layers.sh --strict src/mods/effects src/mods/sky`
  is 0, and effects left `NOT_YET_LIVE`. `Modules.effects` is `apply = "live"`.
- effects' hooks go through the registry, behind the module's gate, and chain: the
  debris' `Fragment` calls (the Evolver's four cocoon sites no longer call a constant),
  the repair welds' two calls and the shield hits' four (a passive tap). They are put in
  at the module's first frame (`device()`), as before, not at load: other plugins
  check the stock bytes of those sites when they load (the bench's `Scene.asi` does).
  Debris is taken only while the gate is open (its flag was never cleared); each
  part's switch is decided in `configure`, not latched at `device()`. A flip off drops
  the debris in flight and leaves the other parts' records, which the engine keeps.
- `configure` reads effects' sections (`[Explosions]`, `[Shields]` with the
  `[Shell.<name>]` recipes read once from the engine's table of shell names,
  `[Repair]`, `[Debris]`, `[Construction]`, `[Bees]`, `[Sparks]`) and sky's `[Sky]`;
  `[Sky]` is read at load and at every settings change, no longer every 60 views.
  `config::Section` can borrow the ini `configure` was given (`Section::of`).
- sky: `formats_changed` drops the view pipelines and keeps the baked cube.
- `assets::odf_stem`, the one `.odf` strip, for effects' two.
- `render3d/install.sh` and `perf/sky-*.py` read the sky recipes from `content/sky/`.

Installed, not yet seen in game.

## 0.40.0 — 2026-11-03

### Changed
- world and overlays onto the module layers (phase 7, W-B): `world/hooks.rs` and
  `overlays/hooks.rs` hold every engine address, call, slot and field read behind named
  accessors; `lint-layers.sh --strict src/mods/world src/mods/overlays` is clean.
  Their built-in names and recipes are in `content.rs`; `configure` reads `[World]`,
  `[Nebulae]` and `[Overlays]` (no `config::ini()` call is left in either directory).
- world's three hooks (the nebulae's lights, the nebula frustum test, the ghosts'
  `Render`) and overlays' five `DrawHeightPost` calls install through `core_hooks` at the
  module's first switch-on (not in `settle`), each behind the module's gate. The
  nebulae's light hook, which replaces `Nebula::Simulate_Nebula_Lights` whole, now calls
  the previous target while the gate is shut or `NebulaLights=0`; the frustum scale is 1
  when it is shut.
- `Modules.world`, `Modules.overlays`, `World.Planets`, `World.Rocks` and `World.Nebulae`
  are live rows (`apply = "live"`); `NOT_YET_LIVE` loses world and overlays. `off()` stops
  the ghosts' hiding, the planets' glows and the height posts' collecting.
- The surfaces' and nebulae's recipes moved to `render3d/content/surfaces/` and
  `render3d/content/nebulae/` (the game-side `Surfaces\` and `Nebulae\` are the same) and
  are built into the plugin too, read through `core_recipes`: the game directory's copy
  wins, else the built-in one. `render3d/install.sh` and `publish/package.sh` read the
  new paths. New dependency: `a2ini` (the recipes' parser).
- No change to what is drawn: the base's frames at RMSE 0 (see the scenarios below).

### Added
- Scenarios `render3d-live-world` and `render3d-live-world-nebula` (testbench 3.92.0).

Installed, not yet seen in game

## 0.39.0 — 2026-11-03

### Changed
- particles and ordnance on the hook registry (module interface v2): every engine
  read and hook of the two modules is in `particles/hooks.rs` and `ordnance/hooks.rs`
  (`ordnance/eng.rs` became the latter), `render3d/lint-layers.sh --strict` is 0 for
  both directories, and the two left `NOT_YET_LIVE`. `Modules.particles` and
  `Modules.ordnance` are `apply = "live"`: a flip off hands every particle, shot, wave
  and warp flash back to the engine, and a flip on takes them again. Their hooks go in at the
  module's first switch-on behind its gate (particles' no longer from its first
  `lights()`), and call what was there before, never a constant: the billboard
  effects' call, the warp flash's sprite (its `Prev` was seeded with the constant) and
  the per-class `Entity::Render` slots (taken lazily through the registry, each with
  its own `Prev`). The warp flash's hook goes in whatever `[Warp] Enable` says and
  hands on while it is 0 (read again with the settings).
- `configure` reads `[Particles]` and ordnance's sections (`[Ordnance]`,
  `[Ordnance.<odf>]`, `[Model.<odf>]`, `[Wave.<odf>]`, `[Warp]`, the `[Lights]` beam
  keys) at load and at every settings change; a change that leaves the recipes as they
  were drops nothing. `off()` clears particles' simulation, owned set, drop count and
  feed queue, and ordnance's flashes, effect ages, muzzles and class choices.
- Built-in tables in `content.rs` (the damage systems' node names, the vent sprites,
  the two free emitters; ordnance's recipes, now `render3d/content/ordnance/ordnance.ini`
  with `git mv`). The nebula puffs ask `core_recipes` whether a nebula has a recipe.
- If the billboard effects' call is not stock's, particles no longer draws them over
  the engine's own.

## 0.38.0 — 2026-11-03

### Changed
- hulls and specials onto the module layers (phase 7, W-A): `hulls/game.rs` is
  `hooks.rs` (every engine read behind a named accessor, including the instance's
  fade, face limit and sphere, the cocoon, the class effect nodes and the planets the
  shadow map sees), `specials/hooks.rs` holds the effect list's reads; their built-in
  names and tables are in `content.rs`. `configure` reads `[Hulls]` and `[Specials]`
  (`look::load(&Ini)`), and `render3d/lint-layers.sh --strict src/mods/hulls
  src/mods/specials` is clean.
- The scene lights' keys (`[Lights]` `Replace`, `KeyColour`, `KeyAxis`, `FillColour`,
  `FillAxis`, `SkyLight`) and the `Lighting.asi` test moved from `scene/lights.rs` to
  the hulls module (`scene_lights.rs`); the core applies them through the new
  `core_services::SceneLightSource`, as before whether the hulls module runs or not.
  `SceneConfig` stays in `scene/lights.rs`, with `Style`.
- No change to what is drawn: the base's frames at RMSE 0 (`port-showcase`,
  `port-firing`, hulls and specials off at load and live). `Modules.hulls` and
  `Modules.specials` were live already.

### Added
- `specials::off()` drops the per-effect ages. Scenarios `render3d-live-hulls` and
  `render3d-live-specials` (testbench 3.90.0).

Installed, not yet seen in game

## 0.37.2 — 2026-11-03

### Changed
- Light semantics as data (`scene/media.rs`): what a light is to the nebulae's gas is a
  registered medium (`lights::register_medium`, `set_medium`, `scene::add_point(..,
  medium)`, `Pool::register_in`) in place of the `Gas` enum, `set_gas` and
  `SPARK_POINT`; effects names `fireball` and `spark`, ordnance `bolt`, world
  `own_glow`. Each slot is fixed by the gas's response, so the packed word is
  unchanged; `render3d/hosttest` asserts it on the host (in CI) and pins the shaders'
  falloff, which is untouched.
- The planets' day side is a generic light flag (`FLAG_SHARE`, the same bit) that
  world sets; `is_day_side` is world's. The beam keys (`[Lights] Beam*`, `ConeSpot`,
  same names and defaults) are read by ordnance's `configure` and passed per light
  (`clusters::LineParams`, `SpotParams`); the core keeps the cluster keys. The sky
  light's read of the map's background name moved to `engine::light`. No change to what
  is drawn.

Installed, not yet seen in game

## 0.37.1 — 2026-11-03

### Changed
- The helpers other modules took from the hulls, effects, ordnance and particles
  modules moved into the core, with no change to what is drawn: `scene::instance`,
  `scene::anim`, the posed objects in `scene::pose`, `scene::camera`,
  `scene::lights::Pool`, `scene::sprites`, `gfx::fx`, `math::rand`, `core_particles`
  (the particles' feed queue), and their engine reads in `engine::{instance, light,
  sprite}`. The four duplicate address groups (`RegisterLight` and its two neighbours,
  the DX8 device vtable, `DrawScaled3D`, the device's clear-depth and flush slots) are
  one constant each.
- hulls, ordnance, overlays and world register their draws as services
  (`HullDraw`, `WaveDraw`, `PostDraw`, `GhostDraw`); effects, ordnance, specials and
  overlays ask `core_services` in place of `crate::mods::<other>` and
  `running("hulls")`. hulls' `ShadowMap` is an alias of `core_services::ShadowRef`.
- `lint-layers.sh --rules cross,byname` (with `--strict`), run over every module in CI.
  Cross-module paths 41 -> 0, by-name asks 7 -> 0, engine reads 909 -> 835.

Installed, not yet seen in game

## 0.37.0 — 2026-11-02

### Added
- The module interface, version 2 (`src/core_module.rs`): every module answers
  `priority`, `claims`, `rows` and `configure`, and may answer `provide`, `hooks`,
  `on`, `off` and `formats_changed`. The registry orders by priority (world 10 ...
  overlays 80, as before) in place of a name list, and `mods::claims` is the OR of the
  running modules' `claims()`.
- The hook registry (`src/core_hooks.rs`: `Site`, `Hook`, `Hooks::install`, `Prev`,
  `Gate`), which checks a call site or vtable slot against stock before writing and logs
  `hooks: <owner> <used by> at <symbol> (0x..) installed`. The frame's seams,
  `DestroyRenderSurfaces` and the device's `Reset` go through it.
- The services' traits (`src/core_services.rs`: `HullDraw`, `WaveDraw`, `PostDraw`,
  `GhostDraw`), the recipe lookup (`src/core_recipes.rs`; the sky's recipes, the menu's
  `map.ini` and the built-in `ordnance.ini` are read through it) and `config::Section`
  (effects' `Sec`, re-exported).
- Per-module settings fragments: world's rows in `src/mods/world/settings.toml`,
  overlays' in `src/mods/overlays/settings.toml`; `Render3D_Settings` returns the
  core's rows and then each module's, one table in the same order as before (21 rows).
  Render3D.log: `settings: N rows exported`.
- `lint-layers.sh`, which counts engine reads, cross-module paths, modules asked for by
  name and content names outside their layer (report mode; `--strict DIR...`).

### Changed
- Every module's `[Modules]` flag is read at each settings change. One whose hooks do
  not hand back yet (world, particles, effects, ordnance, overlays) refuses the change,
  logged `settings: Modules.<m> changes at the next launch (hooks not yet live)`.
- A module that stands down beside its superseded plugin keeps its place in the
  registry (it is never dispatched); a module switched on after its device was made
  gets its `device()` at its next view.
- With `Enable=0` only the core's ten rows are exported (no module is made).

Installed, not yet seen in game.

## 0.36.0 — 2026-11-02

### Added
- `Render3D_SettingsNote`, the optional settings export that says why a part is not
  running: a plugin-wide note when Render3D stood down (`Enable=0`, a seam that is not
  stock, no usable device) and, on a module's row, "Stands down while <Plugin>.asi is
  loaded". NULL when nothing is wrong.

### Fixed
- `Cargo.toml` and `Cargo.lock` said 0.16.3 while this changelog said 0.35.0 (the log
  line `Render3D <version>` read the old one); now 0.36.0.

Confirmed in game 2026-11-03.

## 0.35.0 — 2026-11-02

### Added
- `settings.toml` and the generated `Render3D_Settings` table (`MSAA`, `HDR`, the eight
  `[Modules]` flags, `World.Planets/Rocks/Nebulae/Surfaces`, `Nebulae.Lit`,
  `Overlays.Fog/Grid/Selection/Ghosts/HeightPosts/Pivot`); the code reads those keys
  through their rows. `Render3D_SettingsChanged` re-reads the live rows.
- The shipped `Render3D.ini` has `[Modules]` with every module at 1.
- Live without a restart: `Modules.hulls`, `.sky`, `.specials`, `World.Surfaces`,
  `Nebulae.Lit` and the six `Overlays` keys. The others (`particles`, `effects`,
  `ordnance`, `world`, `overlays`, `World.Planets/Rocks/Nebulae`, `MSAA`, `HDR`) are
  latched at load: their engine hooks are installed once.

### Changed
- Every module is made at load behind an `on` flag, in place of the `MenuOnly`
  wrappers; a module that is off is skipped by dispatch and is not `running()`, and one
  without a menu-scene part and off at load is given no device, so installs nothing as before.
- The height-post hook is installed whether or not `HeightPosts` is on (it hands every
  call to the engine's while off).

Confirmed in game 2026-11-03.

## 0.34.0 — 2026-11-02

### Added
- `Render3D.ini`'s keys may now come from the player's overlay, `RefitPlayer.ini`
  (`[Render3D.<Section>]`, e.g. `[Render3D.Modules] sky=0`, read first; the a2rt
  overlay lookup, `tools/settings/README.md`), with a `settings: RefitPlayer.ini [...]`
  line in Render3D.log for each value it supplies. Read at load as the ini is; applied
  live from 0.35.0. Without the file nothing changes.

Confirmed in game 2026-11-03.

## 0.33.1 — 2026-10-30

### Changed
- Built on `gpu` 1.2.0 (contract 1.2): on Direct3D 12 the graphics pipelines are loaded
  from and stored into Display8's pipeline library (display 0.15.0); nothing changes
  on Direct3D 9 or Vulkan, or against a Display8 of minor 1.0 / 1.1.

Confirmed in game 2026-10-30. (d860f47)

## 0.33.0 — 2026-10-30

### Changed
- `Render3D.log` moves out of the game directory: `%LOCALAPPDATA%\armada2-refit\logs\Render3D.log`, beside `Display8.pipelines`'
  folder, made if missing; the game directory when `LOCALAPPDATA` is unset or not
  writable. The first line says where (`log: <path>`). An uninstall leaves the folder.

Confirmed in game 2026-10-30. (9a5e70a)

## 0.32.0 — 2026-10-30

### Added
- Registers with Display8 (contract 1.1, `gpu` 1.1.0) and claims each module running,
  and `fog` and `selection` by `[Overlays] Fog` and `Selection` (`mods::claims`); a
  module at 0 claims nothing.

### Changed
- The selection bubbles are left out while another plugin claims `selection` (the UI's
  `[HUD] SelectionBubbles=0`), asked through `display8v1_claims`; the `ret` in
  `RenderSelectionEffects` is read only on a `d3d8.dll` without claims.

Confirmed in game 2026-10-30. (cf3dec5)

## 0.31.0 — 2026-10-30

### Changed
- Built on `gpu` 1.0.0, contract 1.0 with Display8: asks `display8_abi` before it draws
  on Display8's device and logs both versions; with a Display8 before 0.12.0 it logs
  "too old" (apart from "no Display8") and stands down to Direct3D 9 only. Needs
  display 0.12.0's `d3d8.dll`: ships with it.

Confirmed in game 2026-10-30. (c470071)

## 0.30.0 — 2026-10-30

### Added
- The overlays module draws the tactical camera's pivot marker (the feature
  `TacticalCamera`, `src/mods/overlays/pivot.rs`) in place of the engine's `dummy.sod`
  lens: a thin ring with four ticks on the map's plane, a soft halo under it, drawn as
  the height posts' lines. `[Overlays] Pivot=` (1; 0: the engine's), `PivotRadius`,
  `PivotWidth`, `PivotGlow`, `PivotColour`, `PivotBrightness`. Taken only while
  `TacticalCamera::Render` is stock's. Confirmed in game 2026-10-30 (1da0694)

## 0.29.5 — 2026-10-30

### Changed
- `src/rt`, `engine/hook.rs`, `dump.rs` and the shared part of `clock.rs` moved into
  `crates/a2rt` (shared with ui and display); `a2fmt` moved to `crates/a2fmt`. Pure
  refactor: same pixels, same log lines. Confirmed in game 2026-10-30 (3030e0c).

## 0.29.4 — 2026-10-30

### Changed
- `install.sh` merges `Render3D.ini` with `tools/ini-merge.py` (`--before-comments`, the old
  inline merge's placement; its retired-default table moved there), byte-identical on the
  fixtures of `tools/ini-merge-selftest.sh`. Confirmed in game 2026-10-30 (1973ecb).

## 0.29.3 — 2026-10-30

### Changed
- `hlsl-rs.sh` and the scripts it runs moved to `tools/shaderc/`; `build.sh` calls them
  there. The generated `src/shaders/*.rs` differ in their "generated by" header line
  only, and the stamps are unchanged. `64aee16`. Confirmed in game 2026-10-30

## 0.29.2 — 2026-10-30

### Fixed
- `[Modules] world=0` and `sky=0` no longer empty the menu scene of its planet, gas and
  sky: like `hulls=0`, the module stays loaded for the menu scene only
  (`mods::MenuOnly`; `World::menu_scene()` has every `[World]` part on and installs none
  of the mission hooks), so `[World] Planets=`, `Nebulae=`, `Rocks=` and `Enable=` at 0
  leave the menu alone too. Missions keep the engine's look for a module at 0.
  Particles, effects, ordnance, specials and overlays draw nothing in the menu scene.
  `e23d234`

Confirmed in game 2026-10-30.

## 0.29.1 — 2026-10-30

### Fixed
- `[Modules] hulls=0` no longer empties the menu scene of ships: the engine draws
  nothing in the front end, so the module stays loaded for the menu scene only
  (`hulls::MenuOnly`) and draws its ships as with `hulls=1`; missions keep every hull
  the engine's. `b1ff913`

Confirmed in game 2026-10-30.

## 0.29.0 — 2026-10-29

### Added
- A frame of render3d's `gpu` calls can be recorded for a replay with no game
  (`gpu/README.md`, "Record and replay"): asked for with `Gpu.record` beside the exe, on
  Display8's Vulkan backend only, written to `Dumps\<name>\Render3D.asi.a2s`. Idle, it
  costs two flags per call and a read of the clock per command list. `239f29c`

Confirmed in game 2026-10-30.

## 0.28.0 — 2026-10-29

### Added
- On Vulkan its `gpu` publishes its pass now to Display8 at each `Cmd`'s end
  (`gpu::Gpu::with_display_publishing`), so Display8's passthrough draws what render3d
  hands back to the engine (a module off, a SOD it cannot draw) into the scene pass,
  tone-mapped and bloomed with the rest (display 0.10.0). `24be351`

Confirmed in game 2026-10-30.

## 0.27.1 — 2026-10-29

### Changed
- A new device or a `Reset` is learned from Display8's `display8_device_epoch` when
  Display8 is present (`frame::begin`, through `Gpu::device_epoch`); without the
  export, the device pointer is compared every frame as before.

Confirmed in game 2026-10-30.

## 0.27.0 — 2026-10-23

### Changed
- A fireball's light in a nebula's gas has a core and a wash: about full within
  `LitExplosionCore=` of the dead craft's radii, falling as the inverse square past
  them to a wash of `LitExplosionFar=` out to the light's reach (`nl_light` in
  `world_nebula.hlsl`). It lit the gas evenly to one radius and linearly to ten, so a
  breach inside the gas lit the whole cloud in view to the knee. The hulls take the
  fireball's light as before.
- `[Nebulae] LitExplosion` defaults to 1.5 (was 0.75), for a hotter core; an install
  replaces a player's old default (`install.sh`, `RETIRED`), a value they set stays.

### Added
- `[Nebulae]` keys `LitExplosionCore` (2) and `LitExplosionFar` (0.12), in the shipped
  `Render3D.ini`.

Confirmed in game 2026-10-23.

## 0.26.1 — 2026-10-23

### Fixed
- On Vulkan and Direct3D 12, the emitter flare of a `Kind=cone` beam (the
  Assimilator's auto-assimilator, `borgbor`) drew as an opaque black square at the
  beam's start. The fx pixel shader (`fx_ps` in `ordnance.hlsl`) now `select`s the
  quad's kind instead of weighting every kind by 0 or 1 and adding them up: the cone's
  term, run on a flare's parameters, overflowed to infinity, and 0 x infinity is NaN
  there (Direct3D 9 takes it as 0). The cone's rim no longer overflows past its outline.

Confirmed in game 2026-10-23.

## 0.26.0 — 2026-10-23

### Changed
- A phaser beam's line light (`cl_line` in `clustered.hlsli`) emits alike along the
  whole beam: what reaches a surface is the integral along the segment, the angle it
  spans over the distance from it, so it is even beside every part of the beam, falls
  as 1 / distance and dims past the ends by itself; its direction is the segment's
  representative point. It was a point light stretched along the beam, full to 4
  units and gone at 40 (falloff squared), so a beam passing a hull or a rock 30 units
  off lit almost nothing. The ends stay brighter by the emitter and impact lights.
- `BeamLight` is the brightness at `BeamCore` beside a long beam (default 4.0, was
  1.0), `BeamReach` where it has faded to nothing (default 200, was 40). An install
  replaces a player's old defaults (`BeamLight=1.0`, `BeamReach=40`) with the new ones
  (`install.sh`, `RETIRED`); values a player set stay.

### Added
- `[Lights]` keys `BeamCore` (8) and `BeamGasReach` (40: the beam's reach in a
  nebula's gas, which keeps the look the march was tuned to).

### Fixed
- `install.sh` adds a key missing from a player's `Render3D.ini` at the end of its own
  section, before the next section's comment, not after it.

Measured on the bench (`battle`, 1280x720, Vulkan on the GPU): the objects step 1.27 ms
of GPU time against 0.91 before, the cull 0.22 against 0.18. Direct3D 9 and
`Clustered=0` are unchanged (the two ends light, the length does not). Confirmed in game 2026-10-23.

## 0.25.0 — 2026-10-23

### Added
- `scene::lights::Gas::Spark`: a small light at a hull, which the gas takes as a
  corona like a torpedo's, at `[Nebulae] LitSpark=` (0.3). Used by the shields-down
  discharges, the repair, construction and bee welds (`LightPool::register_spark`),
  and the damaged sensors.
- The damaged sensors' green spark nodes (`sensor*` sprite nodes, shown while a
  craft's sensors are down, as in the Mutara) are held on one frame at half size, each
  with a small steady point light of its colour. New `[Hulls]` keys: `SensorSteady`
  (1), `SensorSize` (0.5), `SensorLight` (2) and `SensorLightReach` (4, in drawn half
  sizes).
- The shipped `Render3D.ini` lists `LitSpark=0.3` and `LitCorona=0.25` under
  `[Nebulae]`.

### Fixed
- In the Mutara, the shields-down discharges lit a wide round ball of gas, blue, beside
  every shielded Borg ship. They now give a tight corona.
- Points that a module adds to the scene's list by hand (`scene::add_point`) are kept
  when the ordnance module reads the engine's lights again (`regather_lights`). Before,
  that read dropped the points of every module whose `begin_view` ran before it.

Confirmed in game 2026-10-23.

## 0.24.0 — 2026-10-23

### Added
- Light inside the nebulae on Vulkan and Direct3D 12: the march's lit twins
  (`PS_MARCH8_LIT` .. `PS_MARCH24_LIT`, `PS_TEMPORAL_LIT`, dxc's alone) scatter the
  view's clustered lights in the gas at every step, of one colour only the strongest;
  fireballs, torpedoes and pulses (a corona about the shot), phaser beams' line lights,
  vents and the rest light the vapour, the nebulae's own lights and planets' day sides
  none. Where lights make the gas's light, the frames added up clip harder and take
  more of this frame, so a moving light does not trail; the light along a ray rolls
  off at a soft knee, so a breach does not white the frame out.
- `[Nebulae]` keys `Lit`, `LitStrength`, `LitTint`, `LitExplosion`, `LitBolt`,
  `LitBeam`, `LitCorona`, `LitKnee`, `LitTemporal`, `LitClip`; the shipped `Render3D.ini` lists
  `Lit`, `LitStrength`, `LitExplosion`, `LitBolt`, `LitBeam` and `LitKnee` in a `[Nebulae]`
  section.
- `scene::lights::Gas`: what a light is to the gas, in the cluster list's `t.z`.

### Changed
- The clustered lights' cull (`CS_CLUSTER`) reads the lights through group-shared
  memory, 64 at a time, each put into view space once per group, leaves out for a
  group the lights that cannot reach its slices, keeps each group's count and weakest
  as it goes and sorts only a list an entry was replaced in; the clusters it writes
  are the same (the `battle` frame at RMSE 0 on Vulkan and Direct3D 12), 0.19 to
  0.16 ms at 3440x1440.

Confirmed in game 2026-10-23.

## 0.23.2 — 2026-10-23

### Fixed
- `hlsl-rs.sh` (which ui and display build their shaders with too) compiles in a scratch
  directory beside its outputs instead of `/tmp`, so each generated file goes into place
  by one rename: two builds of one tree at once never read one half written.

Confirmed in game 2026-10-30.

## 0.23.1 — 2026-10-23

### Changed
- The shipped `Render3D.ini` has a `[Lights]` section with the clustered lights' keys
  at their defaults (`Clustered=1` and the rest), so an install adds them to a
  player's file and `Clustered=0` is there to switch.

Confirmed in game 2026-10-30.

## 0.23.0 — 2026-10-23

### Added
- Clustered lights on Vulkan and Direct3D 12 (`scene/clusters.rs`,
  `shaders/clusters.hlsl`, `shaders/clustered.hlsli`): every light of the view lights
  the hulls, planets, moons and asteroids per pixel, binned by a compute pass into
  screen tiles by depth slices, with no cap per mesh; of one colour only the strongest
  at each pixel. Clustered twins `PS_HULL_CL`, `PS_BUMP_CL`, `PS_ROCK_CL`,
  `PS_GROUND_CL`, `PS_CITY_CL`, `PS_CLOUD_CL`, `PS_SURFACE_CL`; the cull `CS_CLUSTER`.
- Phaser beams as line lights, and the tractor and assimilation beams' wash as a spot
  confined to the cone (ART-DIRECTION.md), where clustered lights run.
- `[Lights]` keys `Clustered`, `ClusterTile`, `ClusterSlices`, `ClusterLights`,
  `ClusterPerColour`, `BeamLight`, `BeamReach`, `BeamWrap`, `ConeSpot`.
- `hlsl-rs.sh`: an entry marked `:dxc` is dxc's alone (no d3dbc), for compute and
  storage. `bench-ini.sh` sets a bench clone's `Render3D.ini` keys from a `Setup:` line.
- gpu (no changelog of its own): compute on Vulkan and Direct3D 12 (`Gpu::compute`,
  `Gpu::storage`, `Gpu::compute_pipeline`, `Usage::STORAGE`, `Cmd::dispatch`,
  `bind_storage`, `bind_storage_rw`, `bind_storage_image`, `cs_block`, `write_storage`,
  `barrier`), storage bindings in the SPIR-V reflection, a compute root signature and a
  storage table in the graphics one on Direct3D 12; nothing on Direct3D 9.

### Changed
- With clustered lights, the ordnance module's phaser, muzzle and effect lights are
  points of the scene's own list, not engine light objects.

With `Clustered=0` and on Direct3D 9 every frame is as before (the port scenes at
RMSE 0 against `pre-gpu-*` on Direct3D 9). Installed, not yet seen in game.

## 0.22.1 — 2026-10-23

### Fixed
- Every nebula class was baked twice, on its first two frames (the menu's and a
  mission's start): a class made in the middle of a gather started its signature from 0
  (`SIG0`). The log says why a class is baked again.
- On Vulkan a nebula bake took 0.4 to 1.5 s a class, a stall of seconds at the main
  menu and at a mission's start: `gpu`'s read-back read the card's memory through the
  PCIe window; it reads host-cached memory now (`gpu/README.md`, "Read-backs").

Confirmed in game 2026-10-23.

## 0.22.0 — 2026-10-23

### Added
- The frame and every module on `gpu`'s Direct3D 12 backend when Display8 draws on
  Direct3D 12: every pipeline carries its DXIL (`gpu::code!`'s third name), and a pixel
  shader drawn with an alpha test its alpha-tested twin, built from entries marked `:at`
  (`hlsl-rs.sh`, `dxil-alpha.py`: `PS_SOD`, the particles' `PS_SPRITE`, `PS_GHOST`).

### Fixed
- In the DXIL, the post passes', the sky's quad shaders' and the ghosts' pixel shaders
  take the position first, so their inputs sit at the registers their vertex shaders
  write (`DXIL_POS`, `DXIL_POS_FIRST`, `DXIL_POS_MEMBER`); the d3dbc and SPIR-V are
  unchanged.

Measured on the bench against Direct3D 9 on the GPU: showcase RMSE 0.97, nebula 0.39,
firing 0.41 (/255). Installed, not yet seen in game.

## 0.21.0 — 2026-10-23

### Added
- The frame and every module on `gpu`'s Vulkan backend when Display8 draws on Vulkan:
  every pipeline carries its SPIR-V beside its d3dbc (`gpu::code!`), the engine's
  targets, viewport and transforms come from Display8 (`gpu::Gpu::engine_target`,
  `engine_viewport`, `engine_transform`), the engine's textures from its mirrors
  (`gfx::engine_texture`), and the post passes leave out Direct3D 9's half-pixel offset
  there (`gpu::Gpu::half_pixel`). The menu scene's viewport is handed to its pass.

Measured on the bench against Direct3D 9 on the GPU: showcase RMSE 0.87, nebula 0.36,
firing 0.40 (/255). Confirmed in game 2026-10-23.

## 0.20.0 — 2026-10-23

### Added
- `hlsl-rs.sh` also writes DXIL for Direct3D 12 (`src/shaders/<name>_dxil.rs`,
  `pub static <SYMBOL>_DXIL`, and each module's `CB_SPACES`), signed, from the pinned
  dxc v1.9.2609 (`gpu/shaders/fetch-dxc.sh`, `DXC=`); `dxil-spaces.py` puts each
  cbuffer of a stage in a register space of its own for it. Every shader's DXIL is
  committed. `gpu/shaders/compat.hlsli`: `STAGE(n)`'s `vk::` attributes only in SPIR-V.
  The d3dbc and SPIR-V are byte for byte as before (only their stamps changed).

Confirmed in game 2026-10-30.

## 0.19.0 — 2026-10-23

### Changed
- Every shader (`shaders/*.hlsl`, `chain.hlsli`) is modern HLSL (`Texture*` and
  `SamplerState`, `cbuffer`, `SV_Position`/`SV_Target`, `Sample*`, `select`), with what
  differs between vkd3d and dxc in `gpu/shaders/compat.hlsli` (729f9bb). The Direct3D 9
  bytecode is the same code with the same constant registers and sampler stages:
  `d3dbc-slots.py` puts vkd3d's renumbered samplers back on their stages, and
  `d3dbc-fix.py` narrows vPos's declaration to `.xy`.
- The frame's targets, scene pass, resolve, bloom, tone map and frame dumps go through
  `gpu/`, the graphics interface shared with UI.asi; the black line at the view's right
  and bottom edges is cleared after the body (`frame::black_lines`) (caef061).
- Every module draws through `gpu` (1b63c7f): a pipeline per combination of states,
  uniform blocks at the registers the shaders declare, gpu textures and buffers, the
  frame's vertex rings; `gfx::Guard`, `Shader`, `Decl`, `DynVb` and `gfx/chain.rs` are
  gone (the bake chain is `gpu::chain`); game textures, models and nebula volumes keep
  no CPU copy (`Contents::Transient`); timing uses `gpu::Timer`. Every pipeline sets the
  depth bias and the clip planes to 0.

### Added
- `hlsl-rs.sh` also writes SPIR-V for each shader (`src/shaders/<name>_spv.rs`: dxc,
  Vulkan 1.1, checked by `spirv-val`, stamped), not linked in yet.

### Removed
- `src/d3d` (now `gpu::d3d9::raw`).

Measured on the bench: `port-showcase`, `port-nebula`, `port-firing` draw the same pixels
as before (39 frames, scene targets, bloom levels and presented frames: RMSE 0).
Confirmed in game 2026-10-23.

## 0.18.0 — 2026-10-22

### Added
- Frame dumps on request: a `Render3D.dump` file beside the exe (its text names the
  dump; looked for every 30 frames, deleted when seen) makes the next main view write
  the resolved HDR scene target and each bloom level (down and up) as PFM, and the back
  buffer after the tone map as PPM, into `Dumps\` with a sidecar
  `<name>-<frame>-render3d.txt` (`src/dump.rs`; README.md, "Frame dumps").
- `perf/ab.sh A B`: frame time of two checkouts or revisions on the bench, each side
  installed whole, `RUNS=` alternating the order; `perf/summary.py` takes any sides and
  shows the spread across runs (PERF.md, "A/B"; 2ccf6ee, 8162070).

Confirmed in game 2026-10-22.

## 0.17.0 — 2026-10-22

### Added
- `[Bench] Clock=` in `Render3D.ini` (default `wall`, as before): `bench` runs every
  animation of render3d's own on the simulation's time through one function,
  `clock::anim_now()` (hologram and extras phase, vent and special-effect flicker, the
  co-player selection blink, the menu scene), gives the debris a fixed seed
  (`clock::seed()`), settles the nebulae's `Temporal=` history on a standing view (32
  frames from the sequence's first step, then held), and makes the nebula pulse and
  lightning functions of the sim time; per-object seeds come from the object's handle,
  not its address (`clock::entity_key`), and stock shots' sprite animation is evaluated
  at the sim time. For pixel-identical frames on the test bench (ecbb58b, 4a64407).

Confirmed in game 2026-10-22.

## 0.16.4 — 2026-10-21

### Changed
- `Render3D.asi` carries a version resource (`VERSIONINFO`): product, description, the
  layer's version as the file version and the project's as the product version, which
  Windows shows in the file's Properties. Built by `platform/versioninfo/build-res.sh` (915cad8).

Confirmed in game 2026-10-22.

## 0.16.3 — 2026-10-21

### Fixed
- A black hole (`mblkhole`) under the fog of war was a near-black disc in the fog
  (issue #3): a fog-of-war ghost's `additive` meshes (the black hole's disc, a
  wormhole's swirl) were drawn opaque and lit like a hull. They are added now, unlit,
  at half strength after the opaque ones, as stock draws them; an `alphathreshold` mesh
  of a ghost is cut out at one half. `Render3D.log` says how many additive meshes the
  first ghost of each class has (7e7a184).

Confirmed in game 2026-10-22.

## 0.16.2 — 2026-10-21

### Fixed
- A freighter carrying latinum trails the golden latinum spray again (GitHub issue #2,
  7c49408): the emitter `FreighterClass` adds at the ODF's `latinum_spray_hardpoint`,
  which is in no SOD, now fires from the particles module at the hold's latinum fill
  (`particles::sim`'s `object_set`, `cargo_rate`).
- A freighter's cargo pods light in the colour of what it carries (dilithium pale blue,
  latinum yellow), up to the fill, as `Freighter::UpdateDilithiumMarkings` lights them;
  they were drawn as plain plating whatever the hold (`hulls::game::cargo_markings`).
- Meshes of SODs older than 1.7 whose texture material is alpha-tested in the engine
  (modes other than `2 2`) are drawn as cut-outs (`scene::mesh_blend`): the Klingon and
  Romulan freighters' pod lattices, which hid the markings, and other old models'
  alpha holes (`Kfreight`, `Rfreight`, `Bcruise1`, `Bbuildng`, `Fbee`, `Fresear`,
  `Mdmoon` textures).

Confirmed in game 2026-10-22.

## 0.16.1 — 2026-10-21

### Fixed
- On Windows, on a card that takes shaders of 4096 instruction slots (Direct3D 9's own
  runtime refuses a longer one; DXVK takes any), the procedural sky and the planets'
  surfaces were never baked: black space behind the main menu and every map, and the
  planets in their stock texture. The sky's bake (8761 slots) is now a chain of five
  passes and the height of a planet (17812) one of up to eight (`gfx/chain.rs`), none
  over 2940; they bake what the one shader did (0bd0a97).
- `Render3D.log` names each shader Direct3D refuses, with its slots and the card's
  (`gfx: Direct3D refused ...`), and the card's slots at start; the version it logs is
  the changelog's again (it said 0.6.1).

### Changed
- Build: `d3dbc-fix.py` refuses a shader past 4096 slots unless its entry is marked
  `:long` (the code stands down without it: the `Face=0` reference sky and the nebulae's
  march, which falls back to slices on such a card). `hlsl-rs.sh` compiles from the
  shader's directory, for `#include "chain.hlsli"`.

Confirmed in game 2026-10-21.

## 0.16.0 — 2026-10-17

### Added
- A torpedo or pulse lights its own ship where it leaves it (`lights.rs`, the muzzle
  light): the shot's light colour at the point it is first seen, held on the owner,
  reaching its bounding radius and gone over 0.5 s. `[Ordnance] MuzzleLights` (1),
  `MuzzleBrightness` (4), `MuzzleTime` (0.5), `MuzzleReach` (1).

Confirmed in game 2026-10-30.

## 0.15.2 — 2026-10-17

### Changed
- The auto-assimilator's beam (`[Ordnance.borgbor]`) reads clearly against dark space
  at 720p: `Brightness` 2.0 (was 0.9) and `Fall` 0.4 (was 0.12), so a long, thin cone
  (a target far off, met edge-on) no longer fades out before it reaches it. Still a
  translucent cone with fans; the other beams are unchanged.

Confirmed in game 2026-10-30.
## 0.15.1 — 2026-10-18

### Fixed
- On Windows, switching away from the game and back ended in "Fatal error in
  Direct3D": the hulls' posed views and the ghosts' queue kept models, whose buffers
  are in the default pool, through the device's `Reset`. Both are dropped at a `Reset`
  now, and the log names any default-pool object still held then.

Confirmed in game 2026-10-18.

## 0.15.0 — 2026-10-18

### Added
- `d3dbc-fix.py`: rewrites the vkd3d bytecode Windows' own Direct3D 9 refuses (two
  constant or two input registers in one instruction, `sincos` into its own source,
  `oDepth` with a partial mask, temporaries past r31, given out again by liveness);
  `--check` also lists a `texld` inside an `if`, which it cannot rewrite.
- `hlsl-rs.sh --fix`, which puts each shader through it; `build.sh` and `ui/build.sh`
  use it.

### Fixed
- On Windows, without DXVK, render3d stood down ("the post shaders could not be made:
  stock") and the engine drew the 3D view and the menu scene: 51 of the 86 shaders were
  refused. Every one is made now. No sampling inside a branch any more: the fire, the
  rocks, the surfaces' albedo and the nebulae read with gradients, at level 0, or
  flattened, each sampling what it did before (README.md, "Shaders on Windows").
- On Windows, starting a mission ended in "Fatal error in Direct3D. Please reboot.":
  what render3d held kept the shell's device alive, and the engine could not create the
  mission's. `DestroyRenderSurfaces` is chained now, and render3d lets go of every object
  on the device, its shaders and declarations included, before the engine releases it;
  the log counts the references left, and names by kind any object of ours still held
  (`d3d::live`). The hulls' posed views and the ghosts' queue held 49 models' buffers.

Confirmed in game 2026-10-18.

## 0.14.0 — 2026-10-17

### Added
- `[World] MoonCrackDepth` (0.02, in the moon's radii) and `MoonCrackFalloff` (3).
- `[World] Reload=1` reads the rocks' keys (`Rock*`, `Moon*`) again too.

### Changed
- A dilithium moon's light lies at the bottom of its cracks (`ps_rock`, c36.zw): each
  crack is a ravine the view is marched into, its walls dark rock and its light fading
  up them, the near wall hiding it at an angle. Fades to the flat look where a crack is
  only a pixel or two wide.

Confirmed in game 2026-10-17.

## 0.13.0 — 2026-10-17

### Added
- `[World] PlanetGhost` (0.2): a fog-of-war planet ghost's brightness against the
  planet seen, stock's measured dimming.

### Fixed
- A planet left under the fog of war is drawn by the world module on its own sphere
  with its own surface, dimmed, instead of falling back to the engine's low-polygon
  sphere and stock texture (`world::planet_ghost`, called from `overlays::ghosts`).

Confirmed in game 2026-10-17.

## 0.12.0 — 2026-10-17

### Added
- The menu scene's decloaking ships are heard: each plays its group's `CloakSound`
  (default `clkdisen.wav`, the Klingon cloaking device's `offSound`; empty for none)
  as its decloak begins, through the game's 2D sound (`engine/sound.rs`).

Confirmed in game 2026-10-17.

## 0.11.0 — 2026-10-17

### Added
- `[World] MoonCracks` (4), `MoonPulse` (0.5), `MoonCrackCore` (1), `MoonNimbus` (0).

### Changed
- A dilithium moon's cracks are lit from inside (`ps_rock`, c35/c36) instead of cut
  through to the dark shell: the nimbus's colour at its peak (purple on the infinite
  moons), half of its pulse, times the share of dilithium left, whiter at a crack's
  core and bright enough to bloom. The recipe's `Glow=` beside the cracks takes that
  light's hue and pulse. The `oreglow` nimbus, the round halo behind the moon, is no
  longer drawn (`MoonNimbus=1` brings it back).

Confirmed in game 2026-10-17.

## 0.10.1 — 2026-10-17

### Fixed
- A departure's warp flashes drew as hard-edged boxes: the `warp_sprite` art's core is a
  flat white diamond and its halo reaches its border, and at four times stock's light
  both showed. The flash now fades round from near its centre (`flash_ps`) at 1.5 times
  stock's light.

Confirmed in game 2026-10-17.

## 0.10.0 — 2026-10-17

### Added
- The departure: a second export, `Render3D_MenuWarp(name, x, y, w, h)`, draws a view of
  the menu map whose ships go to warp front first (the hull's warp stretch, forward
  only, then a flash of the stock `warp_sprite`), the camera swinging behind them and
  following, and a warp tunnel coming in once they have gone (`tunnel_ps`, a new pass
  `Warp` drawn by the sky module). Keys `[Warp]` (`Delay`, `Stagger`, `Ramp`,
  `Stretch`, `Speed`, `Turn`, `Follow`, `TunnelAt`, `TunnelIn`, `Dim`, `TunnelSpeed`,
  `Widen`, `Flash`, `FlashTime`), `[Group] WarpColour`, `[View] WarpEye`/`WarpTarget`,
  `[Ship] Warp`; the map's starbase stays (`Warp=0`) and the Borg's tunnel is green.

Confirmed in game 2026-10-17.

## 0.9.0 — 2026-10-16

### Added
- The menu scene is one map, `menus/map.ini` (`Menu\map.ini` in the game directory
  takes precedence), asked for by view: `main`, and for the campaign selection
  `tutorials` (a starbase over the planet), `federation` (the fleet), `klingon` (Klingon
  ships decloaking before a Metreon nebula), `borg` (a fusion cube casting its shadow
  through a Mutara nebula, the queen's Diamond close to the camera). The camera moves
  across the map between views (`Blend`, 1.6 s).
  Keys `[Group.*]` (with `CloakView`, `CloakDelay`, `CloakTime`, `CloakStagger`),
  `[View.*]`, `[Ship.*]`, `[Nebula.*]` (its `KeyShare`, `Scatter`, `Opacity`,
  `Brightness` over the recipe's, for the menu's own class).
- The world module draws the menu's gas (`Nebulae::draw_menu`, menu pass `Gas`), and
  the hulls module decloaks menu ships with the cloak's look.

### Removed
- The recipes `menus/main.ini` and `menus/single.ini` and their keys (`Base`,
  `[Camera]`, `[Orbit]`, `[Ship<n>]`): the map's views take their place.

### Fixed
- Menu hulls cast no shadow: the shadow map read a model with no instance as having
  no faces to draw.
- The shadow map leaves no cut-out mesh out: it casts what the hull pass draws of it,
  its texture's alpha cut at 0.5 (half of a Borg fusion cube cast nothing, in missions
  too).

Confirmed in game 2026-10-16.

## 0.8.0 — 2026-10-16

### Added
- The menu scene: a fleet in orbit over a planet, drawn behind the front end's main
  menu and campaign selection on request through `Render3D_MenuScene`, render3d's first
  export (UI.asi's `[Menus] Scene`). Recipes `menus/main.ini` and `menus/single.ini`,
  built in; `Menu\<name>.ini` in the game directory takes precedence and is read again
  when it changes. The sky, world and hulls modules draw their parts (`Module::menu`):
  the recipe sky with stars and a sun, a class planet with no game object behind it
  (`Planets::draw_menu`), lit hulls with shadows and running lights; then the frame's
  bloom and tone map (`frame::menu_frame`).

Confirmed in game 2026-10-16.

## 0.7.1 — 2026-10-16

### Fixed
- Planets, moons, asteroid fields and nebulae drawn by the world module are marked on
  screen (the byte `Entity::Render` writes), so the pointer picks them again: a planet
  could not be clicked, nor a colony ship right-clicked onto it, only drag-selected.
- The selection bubbles are not drawn while the engine's `RenderSelectionEffects` has
  been made to return at once (UI.asi's `[HUD] SelectionBubbles=0`).

Confirmed in game 2026-10-17.

## 0.7.0 — 2026-10-16

### Added
- A third view: a non-main view from a camera other than the action camera's (UI.asi's
  info-panel close-up) keeps per-view state of its own (`View::key` 2), and is logged as
  "other".

Confirmed in game 2026-10-17.

## 0.6.1 — 2026-10-16

### Fixed
- `build.sh` writes `src/shaders/mod.rs` and `build/Render3D.asi` through temp files of
  its own run (`mktemp`, then a rename), so installs of one checkout running at once
  (`a2film --jobs`, `a2test -j`) no longer collide on `src/shaders/mod.rs.new`.

Tooling only; nothing to see in game.

## 0.6.0 — 2026-10-16

### Changed
- Tractor, holding and assimilation beams (`tracbm`, `tracbmo`, `holdbm`,
  `techassimbeam`, `borgbor`) are drawn as a translucent cone through vapour, after
  `ART-DIRECTION.md`: fans of light with hard bright edges that fade in and out each on
  its own, drifting vapour, no core, widening to the
  target's width across the beam (its meshes' boxes, not its bounding sphere), with a
  soft light of the beam's colour on the target's hull.
- Shield hits: an uneven mist about the impact, a soft curved edge at the shell's
  silhouette, rays on Klingon shields only, and a point light on the struck hull.
- Shield colour defaults: Klingon green (was orange), Species 8472 orange (was purple).
- The auto-assimilator (`borgbor`) is the cube's assimilation beam in its blue, with a
  fan of green now and then (`Sprinkle`, `SprinkleShare`).
- The cloak's waves sway the hull across the screen only, never in depth, smaller and
  finer: `CloakWave` 0.03 (was 0.08), `CloakWaveBands` 3 (was 1.5).

### Added
- `Kind=cone` in ordnance recipes, with `Spread`, `Beyond`, `Pull`, `Fans`, `Sprinkle`,
  `SprinkleShare`,
  `EdgePixels`, `Vapour`, `Fall`, `WashBack`, `WashReach`.
- `[Shields]` keys `Mist`, `MistReach`, `MistShare`, `MistContrast`, `MistScale`,
  `MistDrift`, `Edge`, `Rays`, `RayLength`, `RaySharpness`, `RayFactions`, `HitLight`,
  `HitLightWhite`, `HitLightReach`, `HitLightPower`, `HitLights`.

Confirmed in game 2026-10-17.

## 0.5.0 — 2026-10-16

### Changed
- Cloaking in `ART-DIRECTION.md`'s look: while a ship cloaks or decloaks, waves sway
  its geometry so the silhouette bends, and the hull thins into the glass band by band
  in place of the noise threshold; the green front is a thin line along the bands.
- Fully cloaked, slow waves drift in the refraction.
- `CloakHull` is the floor for the player's own ships, now 0.1 (was 0 for every ship).

### Added
- Keys `CloakHullOthers` (0), `CloakWave`, `CloakWaveBands`, `CloakWaveSpeed`,
  `CloakWaveTilt`, `CloakShimmer` and `CloakShimmerSpeed` in `[Hulls]`.

Confirmed in game 2026-10-17.

## 0.4.0 — 2026-10-16

### Added
- Nebulae: `[Nebulae]` `Steps=` (8, 12, 16 or 24; default 8), `Denoise=` (the
  filter's sigma in pixels, default 1.5) and `Footprint=` (the noise reads' footprint,
  default 1) (d367dca).
- Nebulae: the marched gas is added up over frames (`temporal_ps`): the thresholds walk
  on from frame to frame, and each frame is blended into a history per view, reprojected
  by the gas's depth, clipped to the neighbourhood and dropped where the scene's depth
  changed (a ship moved over the gas); the clip's top reaches the brightest pixel two
  out, so thin gas keeps its history; and each pixel's ray moves within the pixel from
  frame to frame, so gas finer than a pixel adds up instead of leaving a still grain.
  `[Nebulae]` `Temporal=` (this frame's share, default 0.1; 0 off), `TemporalClip=`
  (default 1.25) and `Jitter=` (the ray's walk within its pixel, in pixels, default 1;
  0 keeps it at the centre) (9a54a5f, 65b182b, a9da7c5, 0e2f6ed).

### Changed
- Nebulae: the gas is ray-marched at full size, a ray per pixel through the baked
  volume up to the scene's depth (RESZ into INTZ), into a half-float target, and put
  onto the frame through a depth-aware 5x5 filter with `Opacity=`/`Obscure=`. 5.6 ms
  against 3.4 ms on the bench at 1080p, same grain (README.md, "Nebulae"). The slices
  remain only without INTZ and for `Bake=0`.

### Removed
- Nebulae: the half-size pass, its depth-weighted composite and the edge refinement;
  the `[Nebulae]` keys `Resolution=` and `EdgeRefine=`.

### Fixed
- Nebulae: the march's rays were mirrored top to bottom, so the gas was drawn
  reflected about the view and slid as the RTS camera panned (316df23).

Confirmed in game 2026-10-17.

## 0.3.0 — 2026-10-16

### Added
- Sky: recipe key `Octaves=` (1-6, default 6): how many of the gas's octaves count, a
  fraction fading the finest out.
- `perf/sky-shape.py`: measures a recipe's shape against the stock faces (log
  luminance in octave bands and standardised tones, at 512 a face with stock magnified
  bilinearly), and with `--fit` fits `Scale`, `Detail`, `Octaves`, `Coverage`,
  `Softness` (only wider) and `Warp` (only lower) on a running bench game.

### Changed
- Sky: every recipe in `skies/` has its shape fitted to its stock sky (distance from
  stock's structure 0.12-0.45 after grading, from 0.13-1.16), then its grade refitted:
  many mid-sized clumps where most skies had one or two large billows (a40a752).
- Sky: Aqua and `mbgrg` are graded to stock like the rest (0.92 and 0.90 of stock's
  luminance); `perf/sky-grade.py`'s POLICY is empty.

### Fixed
- Sky: a recipe star's halo was cut at the lattice cells' borders, a faint square
  about `mbgpur`'s big star; it now fades out within the cells searched (a40a752).

Confirmed in game 2026-10-17.

## 0.2.1 — 2026-10-16

### Changed
- Sky: the recipes draw the broad gas only. `Grain=` and `Dust=` are gone from every
  recipe in `skies/`, and `Fine=` (the draw-time detail) defaults to `0` (off); the keys
  remain. `GradeGain=`/`GradeCurve=` refitted to the stock faces without them, three
  passes of `perf/sky-grade.py` (0.89-0.92 of stock's luminance; `mbgrg` 0.55, Aqua
  ungraded, as before) (712a09a).

### Fixed
- `perf/sky-grade.py`: `mbgblue` is no longer fitted again on top of the `mbg02` grade
  it is given (it wrote gains of 18-19) (712a09a).

Confirmed in game 2026-10-17.

## 0.2.0 — 2026-10-15

### Added
- The body runs as render3d's own sequence of the engine's steps (`Steps=1`), each
  called through its call site, and modules take steps, objects and game features
  over (`[Modules] <name>=0|1` in `Render3D.ini`); a module stands down while the
  plugin it replaces is loaded.
- `a2fmt`: SOD, TGA (stock mip chains), `.spr` and ODF readers; the scene layer draws
  SOD instances lit by the engine's registered lights.
- Module `hulls` (on; stands down while `Lighting.asi` is loaded, which still draws
  the planets): ships, stations and other plain-SOD objects from their SODs in
  `Lighting.asi`'s look (scene lights, team colours, the Borg bump profile, point lights
  picked as its `pick_points`, the highlight shoulder), fades, cloak glass, warp
  stretch, sprite nodes, billboard effects and flipbooks, and a shadow map of this
  frame's hulls. The scene layer's culling and normals fixed (back faces showed).
- Module `effects` (on): explosions (`Explosions.asi`'s fireball and ring, the core
  blooming by energy in HDR), shield hits and the shields-down arcs (`Shields.asi`),
  repair welding (`Repair.asi`), and hull debris as render3d's own simulation; keys in
  `[Explosions]`, `[Shields]`, `[Repair]`, `[Debris]`.
- Module `ordnance` (on): every shot and the warp flash: phasers, pulses, torpedoes,
  plasma, polerons and artillery in stock's look (near-white texels lifted past white,
  `Hot`/`HotFrom`), the special weapons' beams, bolts, fields and waves and the warp
  flash from `Specials.asi`'s recipes (`ordnance.ini`, built in; Render3D.ini sections
  of the same name override), torpedo lights recoloured and phaser lights as
  `Lighting.asi` makes them. Shots drawn as models (mines, colony pods, the commando
  team, the myotron, black hole, flux wave, stasis, transwarp) through the scene layer,
  in their owner's team colour, with a pulsing glow and light by `[Model.<odf>]`, their
  emitters as render3d's particles (`[Ordnance] Models=`); the mining, orbital and
  repair beams as recipes flowing back to the ship, their impact where the beam enters
  the target (`ImpactAtSurface=`).
- Module `sky` (on): all of `Background_Render`: `Sky.asi`'s recipe skies (built in
  from `sky/skies/`; a `Sky\<name>.ini` in the game directory wins), baked on the GPU
  into a half-float cube; stock faces where there is no recipe; the starfield from the
  engine's octree, point stars lifted into HDR (`StarLift`); background planets and
  suns (`SunLift`); keys in `[Sky]`.
- Module `overlays` (on): the fog of war and shroud with `Fog.asi`'s smooth edges over
  the engine's own scan textures, the tactical grid, the selection bubbles and the
  fog-of-war ghosts of stations, and the height posts (`DrawHeightPost`) as
  antialiased lines (`HeightPosts`, `HeightPostWidth`, `HeightPostSoft`,
  `HeightPostBrightness`); keys in `[Overlays]`.
- Module `hulls` also draws the Evolver and its cocoon (whose burst is render3d's
  debris), holograms as the class they show in a shader of scanlines, flicker and an
  edge glow (`ps_holo`), and the build ghost as stock's pale layered hull
  (`[Hulls] BuildGhost=1`, `GhostDepth=`); `Reload=1` is read while the game runs. The
  log names, once per class, any object no module draws.
- Module `effects` also draws construction (the station rising face by face in the
  hull look, each edge welded with an arc, hot spot, sparks and a seam cooling from
  white to red, lighting the hull; `[Construction]`), the worker bees and drones as
  lit models with a crackling welding beam (`[Bees]`), every spark that is not a weld
  (systems blowing up, phaser flares, beam hits) with stock's sprite and a burst of
  streaking sparks and a flash (`[Sparks]`), and a cancelled construction's debris.
- `Timing=1` (`TimingEvery=`): CPU and GPU time per view and per step, CPU zones per
  module, and any view over 8 ms with its steps, in `Render3D.log`; with `Enable=0` it
  times the engine's own view, which `perf/compare.sh` uses to measure render3d against
  the plugins it replaces (`PERF.md`).
- Module `specials` (on, after `particles`): `Step::Specials`, the timed special-object
  effects (Computer Override, Energy Dissipator, Psionic Disruption): their cloud
  emitters as render3d's particles, plus a shader layer over the struck hull (Borg
  circuitry, a film of veins, violet ripples) and a light at each cloud; an effect it
  does not know is the engine's; keys in `[Specials]`.

### Changed
- The plugins the modules replace were removed from the repository (MSAA, Lighting,
  Sky, Nebulae, Surfaces, Explosions, Shields, Repair, Venting, Specials, Fog, and
  models' `Planets.asi`); a module turned off leaves its part to the engine. The
  recipes moved here: `skies/` (built in), `surfaces/` and `nebulae/` (installed into
  the game's `Surfaces\` and `Nebulae\`); `perf/sky-grade.py` was `sky/grade.py`.
  `install.sh` takes out any old plugin an earlier install left, and Sky.asi's recipe
  copies.
- The state saved around each draw is a recorded block of what our draws change, not
  the whole device state (`gfx::Guard`): render3d's CPU time in a battle 4.7 -> 1.3 ms
  a view, below the plugins' 1.8 ms (`PERF.md`).
- Ini files are kept parsed and read again only when they change: the sky's
  once-a-second re-read cost a ~20 ms hitch under Wine.
- `install.sh` removes each plugin a module that is on replaces (effects: Explosions,
  Shields, Repair; ordnance: Specials; sky: Sky; overlays: Fog), reading `[Modules]`
  from the game's `Render3D.ini`; `./install` runs it last. Its ini merge keeps each
  added key in its own section. The log names each module loaded at start.
- Module `particles` (on): render3d's own particle simulation (spray, cloud and the
  global list, after the engine's rules and quirks, on the sim clock) for every
  emitter node of every object; damage vents in `Venting.asi`'s look with their hull
  lights, other particles, nebula puffs (classes without a volume recipe) and
  billboard effects in stock's; keys in `[Particles]`. `install.sh` also takes out
  `Venting.asi`.
- Module `world` (on): planets on our own sphere in `Lighting.asi`'s planet look with
  `Surfaces.asi`'s grounds (city lights by `cfgPOP_HEAVY`), asteroid fields and
  dilithium moons with their recipe surfaces, nebulae as `Nebulae.asi`'s volumes with
  ships' shadows from the hulls module, and the planet and nebula lights
  `Lighting.asi` added; keys in `[World]`. `install.sh` takes out `Surfaces.asi` and
  `Nebulae.asi` (and copies their recipes into the game), and `Lighting.asi` while
  hulls and world are both on.

### Fixed
- An object's SOD is its ODF's `baseName` when it sets one (`assets::sod_of`), as the
  engine reads it: Martok's Negh'Var (`kbattle`), Caddebostan's ship, the Ferengi
  scavenger, the 8472 hero mothership, Martok's base and the prison were left to the
  engine (stock light and cloak), as were their emitters, debris, ghosts and holograms.

Confirmed in game 2026-10-17.

## 0.1.0 — 2026-10-15

### Added
- `Render3D.asi`: both calls of `Armada_RenderAllOurStuff` (0x59886e, the main view;
  0x4e60f6, the HUD's action camera) go through render3d. The engine's steps draw into
  a half-float target with 4x MSAA, which is resolved, bloomed by energy and
  tone-mapped into the back buffer at the view's rectangle; the engine's depth is
  cleared there for the HUD. `Render3D.ini`: `Enable`, `Log`, `MSAA`, `HDR`, `Bloom`,
  `BloomThreshold`, `BloomKnee`, `Exposure`, `Shoulder`, `Inset`.
- `install.sh` builds it (rustup, `i686-pc-windows-msvc`) and removes `MSAA.asi`, which
  it supersedes; `./install` runs it in MSAA's place, and `a2mod` switches it as the
  `render3d` layer.

Confirmed in game 2026-10-17.
