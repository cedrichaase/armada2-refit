# render3d — the 3D view through our own renderer

`Render3D.asi` takes over the engine's 3D view: both calls of `Armada_RenderAllOurStuff`
(0x598110) come to it, so the scene is drawn into render3d's own targets and put into
the back buffer by render3d. It is written in `no_std` Rust, as crates ("Crates"), and the plan, its
milestones and every change to it are in [`PLAN.md`](PLAN.md).

What it does today:

- **The frame is ours.** The view is drawn into a half-float (`A16B16G16R16F`) target
  with 4x MSAA and a depth of its own, then resolved, bloomed and tone-mapped into the
  engine's back buffer at the view's rectangle. Additive light (explosions, weapons,
  shield hits, glows) now sums past white instead of clipping, and what lies past
  white glows by how bright it is: bloom by energy, which an 8-bit back buffer cannot
  hold. Below the tone map's shoulder every colour is stock's.
- **The body is ours**: `Armada_RenderAllOurStuff`'s steps run in render3d's own
  sequence (`frame/steps.rs`), and eight modules (`## Modules` below) draw every
  category of the 3D view from the game's own assets, read by `a2fmt`: hulls, the sky
  and background, particles, ordnance, effects, the world (planets, moons, asteroids,
  nebulae) and the overlays (fog, grid, selection, ghosts). The cosmetic simulation
  (particles, debris, animation, LOD) is render3d's own, on the sim clock; the game's
  simulation is only read. A part no module draws yet is the engine's, called where
  stock calls it.
- **It replaces the per-part plugins**, which are no longer in the repository: `MSAA.asi`,
  `Lighting.asi`, `Sky.asi`, `Nebulae.asi`, `Surfaces.asi`, `Explosions.asi`,
  `Shields.asi`, `Repair.asi`, `Venting.asi`, `Specials.asi`, `Fog.asi` and
  `Planets.asi`. Their looks are ported here, with the findings the code needs written
  beside each hook below. A module stands down while a stale copy of its old plugin is
  loaded (`SUPERSEDED` in `core/src/lib.rs`), and `install.sh` takes any such copy out.
- **New, beyond the plugins:** bloom by energy (explosions' cores, weapon cores, star
  cores), MSAA on render3d's own targets, and shadows from this frame's hulls (the
  plugin's were the last frame's).

Still the engine's: nothing in a whole match by default. A module hands an object or
effect back to the engine only where it cannot draw it: a SOD that does not load or
is not a plain database (a hologram's class among them), an effect class it does not
know, or a module switched off (construction and the bees need the hulls module, the
emitters the particles module). Nanites have no draw of their own: `NaniteEffect` is
never made, and what shows is the ordnance and `Step::Specials`.

## Building and installing

    render3d/install.sh            # build, install Render3D.asi, Render3D.ini and the recipes; take out any old plugin
    render3d/install.sh --remove   # take it out again

On the test bench, `render3d/bench-ini.sh` sets keys of a clone's `Render3D.ini` from
a scenario's `Setup:` line (`Lights:Clustered=0`, `Render3D:Timing=1`) and then runs the
setup that follows it, as `display/bench-mode.sh` does for `Display8.ini`.

`./install` runs it. Building needs rustup with the 32-bit Windows
target (`rustup target add i686-pc-windows-msvc`) and `lld-link` (LLVM); `build.sh`
uses `~/.cargo/bin/cargo` first, since a distribution's cargo has no such target. The
crate links with `/nodefaultlib`, `kernel32` its only import (an import library made by
`llvm-dlltool --kill-at` from `kernel32.def` in `build.rs`), and brings its own
allocator, memory functions and stack probe (`crates/a2rt`, shared with the UI and
Display8). Shaders are HLSL in each
crate's `shaders/` ("Crates"), compiled by `tools/shaderc/hlsl-rs.sh` with `dxc` into SPIR-V in
`<crate>/src/shaders/<name>_spv.rs` and DXIL in `<crate>/src/shaders/<name>_dxil.rs`
("One source, two forms"), both committed with a stamp so a machine without the
compiler builds as long as they are current.

### Crates

render3d is a Cargo workspace of ten crates, one DLL:

| Crate | Where | What |
|---|---|---|
| `render3d` (cdylib) | `src/lib.rs` | `Render3D.asi`: `DllMain`, the allocator, the panic handler, the exports (`Render3D_Settings`, `Render3D_SettingsNote`, `Render3D_SettingsChanged`) and the list of modules built in, one line per Cargo feature |
| `r3d-core` | `core/` | the renderer core: the frame, gfx, scene, assets, the shared engine glue (`core/src/engine/`), the menu scene, the module interface (`module`), its registry (`registry`), `hooks`, `services`, `recipes`, the particles' feed (`particles`), the configuration, clock and timing, the core's settings table; its shaders `sod`, `clusters`, `post` and `sprites` (stock's sprites, which ordnance's shots and effects' sparks queue) |
| `r3d-<m>` | `mods/<m>/` | one module (world, particles, hulls, effects, ordnance, specials, sky, overlays): `src/lib.rs` (its `Module`), `src/hooks.rs` (its engine glue), `src/content.rs` (its built-in tables), its other sources, its `shaders/`, and its settings fragment (`settings.toml`, generated into `src/settings_table.rs`) |

Recipes are data in `content/<kind>/`, `include_str!`'d by the crate that reads them.

- **A module depends on r3d-core and on no other module** (the cross rule of
  `lint-layers.sh` reads every module's `Cargo.toml` too). What one module draws for
  another goes through `r3d_core::services`, the particles' feed through
  `r3d_core::particles`. Besides the core a module may use the shared crates below it
  (`gpu`, `a2fmt`, `a2ini`, `libm`); the runtime is the core's (`crate::rt`, `log!`,
  `sfmt!`, re-exported at each module's root).
- **Each static lives in one crate**; state shared between crates is the core's.
- **Features.** The cdylib has one feature per module, all on by default, and
  `build.sh` builds that: the DLL that ships has every module. `build.sh --features
  hulls,sky` builds only those, into `build/subset/Render3D.asi` (its own target
  directory, `target-subset/`), a source build for modders and the bench; there are no
  per-module DLLs. CI builds the full set and each module left out.
- **A module not built in** is not in the DLL: the registry never has it, so the
  engine draws its part in a mission exactly as with `[Modules] <m>=0` at load
  (`render3d-build-subset`), and `Render3D_Settings` has none of its rows: neither its
  fragment's nor its `[Modules]` row. Render3D.log says `modules not built in: ...`.
  The menu scene is the exception: without hulls, sky or world built in there are no
  ships, sky or planet in the front end, since nothing else draws them, whereas
  `<m>=0` keeps the menu's part (`in_menu`).
- **On the bench**, `render3d/bench-asi.sh [--without LIST | --features LIST]` builds
  this working copy's DLL (full or a subset) into a clone from a scenario's `Setup:`
  line, then runs the setup after it; `render3d/subset-scenario.sh <m>` writes
  `render3d-build-subset`'s variant for another module.
- **Shaders.** `build.sh` compiles each crate's `shaders/*.hlsl` into that crate's
  `src/shaders/` (a module's with `hlsl-rs.sh --headers core/shaders`, so the core's
  headers are in its stamps); `tools/shaderc/verify-generated.sh` and
  `gpu/spirv-selftest.sh` go over every crate.
- **Settings.** The plugin table stays `render3d/settings.toml`; its rows are generated
  into the core (`rows_in = "core"`, `tools/settings/README.md`), and the cdylib
  defines the export. Fragments stay under it, one per module crate.

### One source, two forms

Every shader of render3d and UI.asi is modern HLSL (`Texture2D`/`Texture3D`/
`TextureCube` with a `SamplerState`, `cbuffer`, `SV_Position`/`SV_Target`, `Sample`,
`SampleLevel`, `SampleGrad`, HLSL 2021's `select`), and `dxc` writes two forms of it:
the SPIR-V for `gpu`'s Vulkan backend (`-spirv`, Vulkan 1.1, which
`tools/shaderc/hlsl-rs.sh` puts through `spirv-val`) and the DXIL for its Direct3D 12
one (below). There is no third: the Direct3D 9 bytecode that vkd3d wrote (`d3dbc`, with
`d3dbc-fix.py` and `d3dbc-slots.py` to make Windows' own Direct3D 9 take it) went with
the Direct3D 9 backend in render3d 1.0.0, and with it the vkd3d fetch, the 4096-slot
rewrite and the `:long` and `:dxc` marks of an entries line. What the measurements in
the changelog say about shaders on Windows' Direct3D 9 is history. The source keeps what
that route taught it, because the rules cost nothing under dxc: loops of a fixed count
and branch-free bodies (`step`/`lerp`/`select`), constants at fixed registers.
`gpu/shaders/compat.hlsli`, at the root because both plugins include it, holds the few
places the two forms part (dxc predefines `__hlsl_dx_compiler`, and `__spirv__` when it
writes SPIR-V); a module's shaders include the core's headers, `core/shaders/*.hlsli`,
by a relative path:

- **Constants keep their registers**: `CREG(n)` is `packoffset(cn)`, so the Rust code
  sets registers, and in SPIR-V a member sits at byte 16 n of its buffer
  (`-fvk-use-dx-layout`): each stage's buffer is its register file. dxc wants a
  cbuffer's members in rising order, and refuses two that overlap; entry points of one
  file that use one register for different things have a cbuffer each.
- **Samplers keep their stages**: `register(tn)` on the texture and `register(sn)` on
  its sampler, both marked `STAGE(n)`, which is the combined image sampler of set 1,
  binding n in SPIR-V and the registers as declared in DXIL.
- **VPOS**: a pixel shader's `SV_Position` is the pixel's centre; `d3d9_vpos()` gives
  the corner, whole numbers, as the shaders were written for.
- **0 x infinity is NaN on SPIR-V and DXIL** (the Direct3D 9 shaders took the legacy
  rule). So a branch-free shader that evaluates every kind and keeps one must `select`
  it, never weight each by 0 or 1 and add them up: a kind run on another kind's
  parameters may reach an infinity, and its 0 weight then turns the pixel to NaN, which
  an additive blend carries into the frame as black. The ordnance module's fx shader
  did that until 0.26.1: the auto-assimilator's emitter flare (a flare quad, whose cone
  term divided by the flare's zero streak colour and overflowed an `exp` past the
  cone's outline) drew as an opaque black square
  (`testbench/scenarios/render3d-cone-emitter.md`).

**SPIR-V's layout**, for the backend that will draw with it: every cbuffer of a vertex
shader is `register(b0)` (set 0, binding 0) and of a pixel shader `register(b1)` (set 0,
binding 1); several cbuffers of one stage alias that binding, which Vulkan allows. A
texture and its sampler are one combined image sampler (`STAGE(n)`: set 1, binding n,
the stage a shader names). Vertex shaders are compiled with `-fvk-invert-y`, so clip space
is Direct3D's (y up) under a viewport of positive height; window coordinates, and so
the winding a triangle is culled by, come out as Direct3D's. Interface locations are
dxc's: each struct member in declaration order, unused ones included, so a vertex
shader's outputs and the pixel shader's inputs agree wherever both use one struct.
Direct3D 9's half-pixel offset (pixel centres on whole coordinates) is the caller's, and
only in the comparison mode `PixelCentres=d3d9`: the UI's `VP.zw` holds it.

**DXIL**, for Direct3D 12, from the same sources: `tools/shaderc/hlsl-rs.sh` writes it into
`<crate>/src/shaders/<name>_dxil.rs` with dxc too, but a pinned release, DirectXShaderCompiler
v1.9.2609, which `tools/shaderc/fetch-dxc.sh` fetches into `$A2_DATA/tools/` by version
and sha256 the first time a DXIL file is stale (`DXC=` names another copy of the same
release; the version is checked). Arch's dxc 1.10 asserts writing DXIL, and what one
release writes is not what the next writes, so the stamp names the release. The DXIL
is signed (the release's `libdxil.so` validates and signs it, which Windows' Direct3D
12 requires; `tools/shaderc/hlsl-rs.sh` checks the container's hash is set), without debug info or
reflection (`-Qstrip_debug -Qstrip_reflect`), shader model 6.0. Its layout: textures
at `tN` and samplers at `sN` of space 0 (`STAGE(n)` is empty there: the `vk::`
attributes are SPIR-V's alone); clip space is Direct3D's already. **The cbuffers need
one change**: DXIL refuses two cbuffers at one register of one space, and several
cbuffers of a stage alias its one register on purpose, some overlapping in it (one
entry point's `c0` is not another's). So the source is preprocessed once (`dxc -P`)
and `tools/shaderc/dxil-spaces.py` puts the k-th cbuffer at `register(bN)` in space k: a stage's
register file is bound at b0 (vertex) or b1 (pixel) in every space 0..k, and each
module's `CB_SPACES` says how many (at most 8, which it checks). The SPIR-V
is untouched by it: it compiles the source as it is.

Two more things are DXIL's alone, both for `gpu`'s Direct3D 12 backend
(`gpu/README.md`, "The Direct3D 12 backend"):

- **The alpha test.** Direct3D 12 has none, and DXIL cannot be patched as the SPIR-V
  is, so a pixel shader drawn with one (`PS_SOD`, the particles' `PS_SPRITE`, the
  ghosts' `PS_GHOST`) is marked `:at` on its entries line, and `tools/shaderc/dxil-alpha.py` adds
  `<entry>__alpha_test` beside it in the DXIL source: the entry called, then the
  fragment discarded by the interval gpu passes at b2. It is `SYMBOL_AT_DXIL`, the
  fourth name in `gpu::code!`. The mark changes nothing in the SPIR-V.
- **Stage linkage by register.** Direct3D 12 links a pixel shader's inputs to its
  vertex shader's outputs by register, as dxc packs each signature in declaration
  order; Vulkan goes by semantic. A pixel shader whose inputs leave out the
  position its vertex shader writes first (the post passes) or take it last (the sky's
  quad shaders), or whose input struct is another (the ghosts'), takes the position
  first in DXIL alone: `DXIL_POS`, `DXIL_POS_FIRST`, `DXIL_POS_MEMBER`
  (`gpu/shaders/compat.hlsli`).

**Entries only some forms can run.** Compute shaders, and pixel shaders that read
storage buffers, are `cs_5_0` entries or sit under `#ifdef __hlsl_dx_compiler`, which dxc
predefines, so the guard is true in every form there is (it was the line vkd3d, which
wrote the Direct3D 9 bytecode and is gone, was kept from). The clustered lights' twins
(`ps_hull_cl`, `ps_bump_cl`, `ps_rock_cl`, the planets' `*_ps_cl`) are such entries:
each sets a static `cl_on` and calls its plain twin, whose light loop takes the
cluster's lights when it is set; in the plain entry `cl_on` is a constant false and dxc
folds it away (their SPIR-V and DXIL are as long as before).

## `Render3D.ini`

| Key | Default | |
|---|---|---|
| `Enable` | 1 | 0: the engine draws the 3D view, as without the plugin |
| `Log` | 1 | `Render3D.log` in `%LOCALAPPDATA%\armada2-refit\logs` (the game directory if that is unset or not writable; its first line says where) |
| `MSAA` | 4 | samples per pixel on the scene target: 0, 2, 4 or 8 |
| `HDR` | 1 | 0: an 8-bit scene target (no light past white, no bloom), for comparison |
| `Bloom` | 0.12 | how much of the light past the threshold spreads into a glow |
| `BloomThreshold` | 1.0 | where bloom starts, in stock's white |
| `BloomKnee` | 0.5 | how soft that threshold is |
| `Exposure` | 1.0 | scales the scene before the tone map |
| `Shoulder` | 0.8 | where the tone map starts to roll off towards white |
| `Inset` | 1 | 0: the HUD's action-camera view is left to the engine |
| `Timing` | 0 | 1: CPU and GPU time per view and step in the log (below); with `Enable=0`, the engine's own view timed |
| `TimingEvery` | 5 | seconds between timing reports |

**`[View]`** ("View distance" below):

| Key | Default | |
|---|---|---|
| `Distance` | 1000000 | the far plane and the object culling distance are raised to at least this; 0: stock (20000 and 2800). Takes effect with the next mission |

**`[Lights]`**, the clustered lights ("Clustered lights" below; Vulkan and Direct3D 12):

| Key | Default | |
|---|---|---|
| `Clustered` | 1 | 0: every lit draw takes its lights from its constants, the strongest few per mesh, as before the clustered lights, when the engine's Direct3D 9 ran them (for an A/B in game) |
| `ClusterTile` | 64 | a tile's side in pixels (larger when the view is wider than 64 tiles or taller than 40) |
| `ClusterSlices` | 24 | slices of view depth, exponential from the near plane to the far one |
| `ClusterLights` | 24 | the most lights a cluster lists (at most 32), the strongest at its box |
| `ClusterPerColour` | 2 | the most of one colour (one group: a volley's bolts) a cluster lists |

The beam keys below are in the same section and read by the ordnance module, which sets
them per line light (`clusters::LineParams`, `SpotParams`):

| Key | Default | |
|---|---|---|
| `BeamLight` | 4.0 | a phaser beam's line light, times the beam's colour, beside the middle of a long beam at `BeamCore` from it (0: none); every unit of the beam emits alike ("Clustered lights", "Beams") |
| `BeamCore` | 8 | world units: closer to the beam than this its light grows no brighter |
| `BeamReach` | 200 | world units from the beam where its light on surfaces has faded to nothing (it falls as 1 / distance before) |
| `BeamWrap` | 0.4 | how far round the plating it wraps |
| `BeamGasReach` | 40 | world units: how far into the gas of a nebula the beam's light reaches (full to a tenth of it, falloff squared), as the march was tuned |
| `ConeSpot` | 1 | the tractor and assimilation beams' wash as a spot confined to the cone (0: the point it was) |

The scene lights (`Replace`, `KeyColour`, `KeyAxis`, `FillColour`, `FillAxis`,
`SkyLight`) share the section ("Scene lights", under hulls).

**`[Modules]`.** Each module has a key of its own, `<name>=1` (the names are in the
sections below: `effects`, `ordnance`, `specials`, `sky`, `overlays`, `particles`,
`hulls`, `world`). **`<name>=0` leaves that part to the engine: its stock look, drawn by
`Armada2.exe` at the step as without the module.** There is no plugin to fall back to
any more, so the old plugins' looks exist only as these modules.

The menu scene is the exception, since it is render3d's own and has no engine draw to
leave anything to: in the front end there are no game objects and the engine draws
nothing (`Display8.log`: `0 engine draws` in `"start"`), so Display8's passthrough has
nothing to show either. A module at 0 that has a part in the menu scene (`hulls`: the
ships; `world`: the planet and the gas; `sky`: the sky, stars, sun and the warp's
flashes) is therefore loaded for the menu scene only (Render3D.log: `module hulls: on,
for the menu scene only`): `hulls::MenuOnly` for the hulls, `mods::MenuOnly` wrapping
the sky and `World::menu_scene()` for the others. The menu is drawn pixel for pixel as
with the module on, whatever `[World] Planets=`, `Nebulae=`, `Rocks=` or `Enable=` say
(they belong to missions; with `world=1` too, the menu ignores them). In a mission the wrapper takes nothing (no step, object,
feature, light or view, no `draw_instance` or `placeholder`, no shadow map lent to the
gas, no hook installed, not `running(name)`), so that part is the engine's. It still
stands down beside the old plugin that draws the part. `particles`, `effects`,
`ordnance`, `specials` and `overlays` draw nothing in the menu scene, so at 0 they are
simply off. A stock look in the menu would need the engine to draw SODs in the shell,
which it never does.

## View distance

`[View] Distance` (default 1000000; 0 leaves the view stock; 4000, 8000 and 20000 are
steps between) makes the view reach across the map. It was `QOL.asi`'s `ViewDistance=`
(QOL-9) and moved here with `moved_from = "QOL.QOL.ViewDistance"`, because the core
already owns the far plane and the culling distance the sky and the hulls read. The row
is MISSION: the far plane is read when a mission's camera is made.

**Today.** Zoomed out, or with the camera tilted towards the horizon, everything past
about 2500 units fades to nothing: ships, stations, planets, asteroids, nebulae. The
map is still there, explored and in sight; the engine just stops drawing it.

**Why.** Two values in `ART_CFG.h` end the view. `cfgOBJECT_CULLING_DISTANCE` (2800)
is the one that shows: `GameObjectInstance::ComputeFadeOut` fades an object from 0.9 of
it to all of it, by its distance from the camera with the vertical scaled by a quarter,
and for some classes scaled by the object's radius, and every instance type's own
`ComputeFadeOut` goes through it (craft, planets, asteroids, cocoons, placeholders, and
the nebulae, whose additive glow is darkened by the fade rather than the instance).
`DistanceCull` culls asteroid fields, Evolvers and building effects with the same
distance, and `Ordnance::RenderAll` reads it too. `FAR_CLIPPING_PLANE` (20000) is the
far plane `PresetView::Init` gives the game camera; `ST3D_Camera::CheckSphereVisibility`
rejects a sphere past it, and the projection clips there.

**What the core does** (`core/src/engine/viewdist.rs`, hook `view.distance`).
`RTS_Configure` parses both keys (a missing key keeps the compiled default) and then, at
the site `view_site`, loads the culling distance to copy it to the word after it, the
value a mission script's `RestoreObjectCullingDistance` puts back after its own
`SetObjectCullingDistance`. That load becomes `call stub; nop`: the stub keeps both
values as parsed, raises both to at least the distance, and loads `ecx` as the `mov`
did, so the copy carries the raised value too. A change while the game runs stores the
raised values again, from the ones kept, and only where a script has not moved them
since. Both values are only ever read to draw, so this stays compatible with stock
players. Fog and shroud are decided elsewhere (`GameObject::CanUserSee`), so what they
hide stays hidden. The cineractive sequences swap in their own far plane and restore
this one, and are left alone.

The patch is made when render3d runs; with `Enable=0` the engine's view is stock. The
profile's hook `view.distance` must be ready (the 21 bytes at the site are checked), or
nothing is patched and the settings note says so.

**On Fleet Operations: Roots** (FO-5 step 7, verified). Roots' file leaves `RTS_Configure`
as stock's; FleetOpsHook re-points one call in it at run time, the last key read before the
parser is freed (+0x1d9d), where it reads keys of its own. The load at `view_site` (+0xebf)
and the 21 bytes checked there are stock's in the file and in memory, FleetOpsHook names
neither the far plane nor the culling distance anywhere, and the function is still called
after the plugins load, so the patch is made and the stub runs at Roots' parse as at
stock's (bench: `view: ART_CFG.h parsed: far plane now 1000000, object culling now
1000000`). Roots' own culling exception (its object loop draws an object the engine culled
for distance when its class sets `renderIgnoreCameraDistance`) is decided after this, by
the same distance.

**Cost.** More is drawn on a wide view. The near plane stays at 20, so the depth
buffer's precision at a given distance hardly changes with the far plane. The shadow map
is fitted around every hull drawn, so far-off ships on a wide view make its texels
coarser, as a big battle already does.

## Timing

`Timing=1` (`core/src/timing.rs`) times each view as a run of marks: the seam, our targets,
each step of the body (`setup` is the lights, the distance cull, the modules' lights
and the scene layer's shadow map; then `background`, `terrain`, `objects`, `ordnance`,
`specials`, `features`, `warp`, `hud prep`, `chunks`, `particles`, `translucent`,
`selection, end`) and `post` (resolve, bloom, tone map). Each mark notes the CPU clock
and issues a GPU timestamp query (`gpu::Timer`); a span is booked to the mark that ends it.
The queries are read back from a ring four frames deep without waiting, so timing
does not stall the GPU; a set not finished when its turn comes again is counted as
late. Every `TimingEvery` seconds the log has the frame's period (from one main view
to the next), each view's CPU and GPU time with their maxima, the time outside the 3D
views (the HUD, the simulation, `Present`), and the split by step.

With `Enable=0` and `Timing=1` nothing of render3d draws, but both call sites of
`Armada_RenderAllOurStuff` go through a body that only marks around the engine's
(`frame::install_measure`), with every plugin hooked into it at work. Installed beside
the plugins render3d replaces, that measures them the same way: the comparison in
`PERF.md`.

## The animation clock

`[Bench] Clock=` (`core/src/clock.rs`) decides what time every animation of render3d's own
runs on. **`Clock=wall`** (the default, the player's) is the wall clock, as it always
was: a paused game still flickers, pulses and blinks. **`Clock=bench`** is for the test
bench (`a2test --clock bench`, `testbench/README.md`, "A frame that repeats"): two
sessions of one scene paused at one tick must draw the same picture, so a port of the
renderer can be checked by a pixel diff.

**The rule: anything that moves on screen reads `clock::anim_now()`, never
`rt::now()`; a cosmetic random sequence takes its seed from `clock::seed()`; and a
per-object seed is made from `clock::entity_key(object)`, never from the object's
address alone.**
`rt::now()` is for what is not seen: timing logs, how often a file is looked at again
(`Reload=1`, the A/B files), how long a noise took to make. With the bench clock,
`anim_now()` is the simulation's time (`Get_Time`, the float at 0x7b0fc8, written once
a tick by `Time_Simulate`), which stands while the game is paused, and `seed()` is a
fixed number. What reads it: the hologram's and the extras' phase (hulls), the vents'
flicker, the special-object effects' flicker and light, a co-player's selection blink,
the debris' random sequence (`seed()`), and the menu scene's map (in the front end the
simulation's time stands, so there a move to a new view is settled at once). A seed
made from an object's address (a beam's or torpedo's flicker and spin, a wave's, an
ordnance model's pulse, a hologram's phase) differs from launch to launch, since the
heap does: `entity_key()` is the address under the wall clock, as before, and under the
bench clock the object's handle (+0x28 of every `Entity`, given out in order). A hull
spark is no entity; under the bench clock its seed is when it was born (the sim time
less its age) and its owner's handle. Stock's own shots are animated by the engine's
`ST3D_Sprite::Animate`, whose channels are evaluated at the float 0x7a8e74
(`ST3D_AnimationChannel::SetStartTime` sets it from Storm3D's wall clock); under the
bench clock the ordnance module writes the sim time there for its call and puts the old
value back after. Without it a torpedo's starburst turned and the phasers' texture ran
on while paused, the last difference between two launches of `showcase`.

What the sim clock (`View::time`) already drove stays as it was, with three changes
under the bench clock, each because a value was summed or drawn **once a frame**, which
depends on how the ticks fell between frames on that run:

- **The nebulae's `Temporal=`.** The thresholds and the ray's jitter walk a sequence on
  the frame count, and the history adds the frames up; paused, the history kept moving
  about its mean. Under the bench clock, a view that stands (the same sim time and the
  same VIEW x PROJECTION as the frame before) drops its history, walks the sequence from
  its first step, and after `SETTLE` (32) frames holds the history as it is: the same
  32 frames added up the same way on every launch, then a still picture. While the view
  moves, nothing changes. The cost: in a running game under the bench clock the history
  starts again each tick, so the gas is grainier than in play.
- **The nebulae's pulse** is a closed form of the sim time (the same wander, integrated)
  instead of a sum of frame steps, and **the lightning's** draws are a hash of the sim
  time and the class instead of a sequence advanced each frame.

Left as it was: render3d's particles are simulated once a frame with the sim clock's
step, so a frame that sees two ticks at once emits as one; the paused frame depends on
that only where the ticks bunched (a hitch) on one run and not the other. The engine's
own selection pulse runs on its display tick; the bench's scenes select nothing.

## How the frame works

**The seam.** The main view calls the body from `Armada_RenderFrame` (`call` at
0x59886e), and the HUD's action camera from `CinematicView::RenderView` (0x4e60f6). Both
calls are re-pointed at render3d's own function, each only if it still calls 0x598110,
so a different `Armada2.exe` or another plugin at either site makes render3d stand
down and the game draws as stock. The body is `cdecl` with three arguments (the
camera, the `Render_Stuff` flags, `isMain`), which render3d passes on unchanged.

**Into our target.** Before the body, render3d binds its scene target and depth in
place of the engine's, keeping the viewport the caller set, and clears colour and
depth: the engine never clears colour in a mission (it relies on the sky to cover every
pixel), and our target has no last frame to fall back on. The body then runs as it
always did. The two views share one set of targets the size of the back buffer, since
each view is a rectangle of it (`camera+0x1b0`, four floats in back-buffer pixels).

**Out again.** After the body, for that view's rectangle only:

1. resolve the samples (`StretchRect`);
2. bloom: a prefilter to half size with a soft threshold and a 1/(1+luma) weighting of
   its four taps, so one hot pixel cannot flicker as a blob; four more levels down a
   dual-filter chain, and back up adding each level onto the one above;
3. the final pass into the engine's back buffer: the scene a texel per pixel plus the
   bloom, through the tone map (unchanged below `Shoulder=`, rolling off towards white
   per channel above it);
4. the engine's depth cleared to 1 in the rectangle, for the HUD drawn over the view
   next.

Every tap is clamped to the view's rectangle, so neither view bleeds into the other's
glow. The engine's own target and depth are bound again after the passes, so the engine
sees what it left.

**Through `gpu`.** Everything render3d draws goes through `gpu` (`gpu/README.md`),
the interface shared with `UI.asi`: the targets, the scene pass and the passes after
it, and every module's draws, each through a `gpu::Cmd` (`gfx::cmd()`) with pipelines
made once per combination of states (`gfx::Pipes`), dynamic geometry in the frame's
rings, and the scene's models in static buffers. No module calls the engine's device; the frame's
glue in `frame/` hooks its `Reset` and `DestroyRenderSurfaces`, and Display8 answers its
transforms and viewport itself (below). The scene pass
is begun before the body and stays bound while the engine draws into it; a module that
draws into a target of its own (a bake, the shadow map, the nebulae's march) begins
the view's pass again after it (`gfx::view_pass`), with the viewport the engine had
set. Stock's black line along the view's right column and bottom row is cleared in it
after the body (`frame::black_lines`).

**The first row and column.** In the comparison mode `PixelCentres=d3d9` the viewport is
offset by half a pixel, as DXVK offset a Direct3D 9 one, so with MSAA half the samples
of the view's first row and column are never drawn; the final pass reads those pixels
from their neighbours instead. Otherwise there is no offset and no such fill, and the
post passes' quads do not move by half a pixel (`gpu::Gpu::half_pixel`).

**On Vulkan.** In Display8's stub mode with `Backend=vulkan` the frame records into
Display8's command buffer through `gpu`'s Vulkan backend (`gpu/README.md`): the engine's
back buffer and depth are Display8's images, the view's transforms and viewport
Display8's record of the engine's calls (`engine_transform`, `engine_viewport`), the
fog's textures Display8's mirrors (`gfx::engine_texture`), and every pipeline is made
from the SPIR-V compiled from each shader. The frame is told a new device by
`gpu::Gpu::engine_id`, which on Vulkan never changes; `Reset` and
`DestroyRenderSurfaces` find nothing to let go of. The menu scene's viewport is handed
to the pass rather than set on the engine's device, so the modules there read the
engine's own. **On Direct3D 12** (`Backend=d3d12`) it is the same, through `gpu`'s
Direct3D 12 backend and the DXIL beside each shader ("One source, two forms", above).
**Without Display8's device** (`Backend=none`, the bail-out to the system's d3d8, or a
Display8 of another contract) render3d stands down: `gpu` is stood down
(`Backend::None`), the log says `frame: standing down: no Vulkan or Direct3D 12 device`
once, and the engine draws the 3D view.

**Reset.** Every target is in the default pool and must be gone before the device
resets. Slot 14 of the d3d8 device's vtable (`Reset`) is chained (as `Lighting.asi`
chained it): render3d drops its targets and passes on the call, and makes new ones at
the next frame, at the new size. A display mode chosen in Graphics Settings is not a
`Reset`: the engine releases its device and creates another. The first frame on a
device render3d has not seen ("frame: a new device") drops the frame's state, hooks
the new device's `Reset` and hands the device to every module, which makes its
resources again; scenario `render3d-mode-switch` checks it, switching to
800x600 in a running battle.

**Letting go of the device.** The display change goes through
`ST3D_DeviceDirectX8::DestroyRenderSurfaces` (slot 28 of its vtable, 0x6bc6ac), which
releases the device in `DestroyDevice`; render3d chains that slot and first lets go of
everything it made for the device (`frame::forget`): the targets, the post passes, every
module's resources, the sky's noise volume. The posed hulls of the last views and the
ghosts' queue go too: they hold models, and a model holds its vertex and index buffers.
Display8's devices are not lost with the engine's, so nothing of this is needed for the
device to go (it was, under native Direct3D 9, where every object held its device and
the engine's next `CreateDevice` failed with "Fatal error in Direct3D"); it keeps the
next frame from drawing with what was made for the old one. `UI.asi` does the same
(`ui/src/engine/device.rs`).

**`MSAA`, `HDR` and the tone map are live** (0.42.0). `Render3D_SettingsChanged` reads
`[Render3D]` again and swaps the whole `Config` in (`config::reload`; the old one is
leaked, a view may still hold it; `Enable`, `Log` and `Timing*` stay as loaded), so
`Bloom`, `BloomThreshold`, `BloomKnee`, `Exposure` and `Shoulder` follow at the next
view, too. When `MSAA` or `HDR` moved, `frame::formats_changed` drops the scene targets
and the post passes (made for the target's format) and calls every present module's
`formats_changed`, which defaults to its `reset`; the next view makes the targets with
the usual fallback list (HDR with MSAA, HDR, 8 bits). `gfx::Pipes::view` re-keys on the
scene pass's formats by itself, sky's view pipelines are the only thing it drops of its
own, and on Vulkan and Direct3D 12 the published pass carries the new samples to
Display8's passthrough. The log line is `settings: applied Render3D.MSAA=0` (and
`.HDR`, `.Bloom`, ...). Checked on the paused showcase at RMSE 0 against the baselines of
the same keys at load, and back (`testbench/scenarios/render3d-live-msaa.md`).

## Frame dumps

A grim shot is 8 bits, taken after the swapchain and the compositor, so it cannot say
where in the frame two renderers part. `a2rt::dump` (`crates/a2rt/src/dump.rs`) reads render3d's own targets back
from inside the process, byte for byte, when asked: for checking a port of the renderer
(the shared `gpu` crate, then Vulkan and Direct3D 12) pixel for pixel, stage by stage.
The files are what `a2test baseline compare` is meant to read (PFM and PPM, as well as
PNG).

**Asking.** Write a file `Render3D.dump` beside `Armada2.exe`; its text, if any, names
the dump (letters, digits, `_ . -`; `dump` when empty). The main view looks for it every
30 frames, as D3DTrace does, deletes it, and dumps in that same view; with no file the
cost is one `GetFileAttributesA` every 30 frames. On the bench, `a2test drive dump NAME`
writes it (`testbench/README.md`, "Frame dumps"). Only the **main** view dumps: the HUD's
action camera draws a second view into the same targets in the same frame, and every
file name says which view it is.

**What is written**, into `Dumps\` in the game directory (made when needed), each file
`<name>-<frame>-<what>`, `<frame>` the main views drawn since the device was made:

| File | Stage | Format |
|---|---|---|
| `main-scene.pfm` | the scene target after the MSAA resolve, before bloom, at the view's rectangle | PFM, the half floats as they are |
| `main-bloom0-down.pfm` … `bloom4-down` | each bloom level after the way down (level 0 is the prefilter) | PFM, at the level's own rectangle |
| `main-bloom0-up.pfm` … `bloom3-up` | levels 0 to 3 after the way up has added the one below (level 4 is as the way down left it) | PFM |
| `main-tonemapped.ppm` | the engine's back buffer right after the tone map, whole: the 3D view as render3d leaves it, before the HUD | PPM (P6), the bytes as they are |
| `render3d.txt` | the sidecar: frame, targets' size, format and samples, the view's rectangle, and one line per file (or why it was skipped) | text |

PFM rows are bottom-up with scale `-1.0` (little-endian), as the format has it; values
are in stock's units, 1.0 its white, unclamped. An 8-bit scene target (`HDR=0`, or the
fall-back) is written as PFM too, divided by 255, and has no bloom levels.

**How.** Each target is copied with `GetRenderTargetData` into a system-memory surface
of its size and format (a multisampled one is first resolved by `StretchRect` into a
plain target of its own), locked, and converted a row at a time as it is written:
`A16B16G16R16F` to f32 on the CPU, 8-bit formats byte for byte. The read-backs sit
inside the post passes' state block, between the passes that make each stage, and
change no state of the device. A format without a conversion is logged and skipped,
not guessed at. **Depth is not dumped**: Direct3D 9, which the dump was written for, could not read a depth surface back
(`GetRenderTargetData` takes render targets only), and a copy through an `INTZ`
texture would be a pass of its own. `Render3D.log` has a `dump:` line per file and one
for the whole.

The frame that dumps stalls for the read-backs (the GPU is waited for at each copy),
so its time in `Timing=1` is not a measure of anything.

Measured on the bench (`SCENE=showcase`, 1280x720, 4x MSAA, 2026-10-22): the dumping
frame took 134 ms; the 5 s reports either side of it were 16.67 ms a frame, the main
view's GPU time 0.83 ms in both, and nothing was written while no trigger was there.
The scene's PFM ran from 0 to 3.07 (mean 0.092); bloom level 0 peaked at 1.77 on the
way down and 2.25 on the way up. With the scene paused, two dumps 2790 frames apart
were byte-identical (scene, bloom, back buffer). The tone-mapped PPM was identical to
the UI's presented frame (the scene has no HUD).

## The menu scene

`core/src/menu/`, the map in `content/menu/map.ini`: a map of render3d's own behind the front end's
main menu and campaign selection, every scene on it under one sun. A Federation fleet
in orbit over a class M planet, a starbase beside it, Klingon ships before a Metreon
nebula far out on one side, a Borg fusion cube and the queen's Diamond in a Mutara nebula
on the other. `UI.asi`
asks for a **view** of it by name (`[Menus] Scene=1`, ui/MENUS.md, "The menu scene")
through render3d's export `Render3D_MenuScene(name, x, y, w, h)`: inside its own
scene, with the back buffer bound, it gets the view drawn into that rectangle and draws
the menu over it. A 0 back (no such view, no device of Display8's, no targets) leaves the
UI's own backdrop. The views UI.asi asks for: `main` (the main menu), and on the
campaign selection `tutorials`, `federation`, `klingon` and `borg`, the campaign pointed
at.

**Nothing of a mission is read.** In the front end there are no game objects, no
camera, no registered light and no sim clock, so the map is a recipe posed on a clock
of render3d's own (`rt::now`, restarted after ten minutes without a frame), the view
and projection are made here, and the lights (the map's Key and Fill) are put into the
scene layer in place of the engine's list (`scene::set_lights`). The frame goes through
`frame::menu_frame`: our targets bound at the rectangle, cleared, drawn, then the same
post passes as a mission's view (resolve, bloom, tone map), and the back buffer bound
again.

**The modules draw it**, each its own part, in `Module::menu(pass, shot, view)`, with
the looks they give a mission:

| Pass | Module | |
|---|---|---|
| `Sky` | sky | the recipe sky of `Sky=` (`content/sky/`), turned about +x by `SkyTurn` a second; a starfield of our own (`Stars` quads of the stock `bstar.0` and `mstar.0` sprites, seeded, the brighter ones lifted past white); the sun, `Sun=`'s SOD (a background object, `Msun1`) and its sprite nodes where the Key comes from, drawn about the eye as a map's suns are, so it stands in the same place from every view |
| `Planet` | world | `Planets::draw_menu`: the class's ground (`groundTextureName`, or its surface recipe) and cloud shell (`atmosphereTextureName`, stock's `PA_ECFR0` when the ODF names none) on render3d's sphere, the same body draw as a mission planet's (`draw_body`) without cities |
| (depth cleared) | | the ships and the gas are drawn on a projection of their own, 2 to 16000 |
| `Ships` | hulls | `menu_begin` poses every ship of the map (finest level of detail, its animation looping, `hardpoints` and the damage, crew and Borg nodes hidden by name, as no class says which) and draws the shadow map from those in view, with the menu planet in place of the engine's; then the hull look, opaque, of the ships in view, and their running lights collected |
| `Gas` | world | `Nebulae::draw_menu`: each `[Nebula.*]` field's nebulae as the class's gas, marched and lit by the Key with the ships' shadows, as a mission's; before the cloak, which bends a copy of what is behind a hull |
| `Translucent` | hulls | additive meshes, decloaking hulls (the cloak's glass, as the engine's state 3), hulls held cloaked (state 2) and sprite nodes, as at a mission's `Step::Translucent` |
| `Warp` | sky | a departure's flashes (the stock `warp_sprite`) and its tunnel (`tunnel_ps`), below |

**One map, one sun.** Every view looks at the same map: asking for another moves the
camera there from where it is (eye, target and field blended over `Blend` seconds,
smoothstepped), across the map, and the clock runs on, so nothing jumps. The Key is the
same for every view, so each camera is set against the sun rather than the sun moved
for it: the Borg camera looks up at the cube with the sun behind it, off to the left; a
moving Key would also bake the gas again every frame. The planet's centre is the
origin; it turns about its axis (`Orbit`, times `1 + Spin`) and the sky about +x (the
fleet's orbit). A **group** is a place on the map: its ships, gas fields and views are
given in its frame (+z forward, turned by its `Yaw`), and a ship's nose points along its
+z. Each ship bobs, rolls and pitches a little on a phase of its own; each camera sways
on three slow sines. The themed groups are far out (16 000 to 20 000 units), each along
its own camera's line from the planet, so the globe is behind the camera.

**Decloaking.** A group with `CloakView=<view>` is cloaked until that view is asked
for; then, after `CloakDelay` seconds (the camera arriving), each of its ships
decloaks in turn (`CloakStagger` apart, over `CloakTime`), drawn with the hulls'
cloak as the engine's decloaking state 3 with that much of the hull back, its running
lights only once it is whole. When the camera leaves, the group stays as it is until
the camera has gone (`Blend`), then cloaks again for the next arrival. A group's
decloak is heard once, as its first ship's decloak begins (fully cloaked in one frame,
part of it back in the next, while the rest of the group is still hidden; one sound per
ship was too much, issue #12): its group's `CloakSound`, by default `clkdisen.wav`, the `offSound` the Klingon
cloaking device's ODF (`gkcloak.odf`) gives for "the ship decloaks". It is a 2D sound
of the game's own sound system (`engine/sound.rs`: an `AudioSound2D` made with the
game's `operator new` at priority 8, as `ShellButton::SetButtonMode` makes its hover
sound), so the player's effects volume applies; it is a sound, not a simulation write,
and plays only in the menu scene.

**Locked campaigns.** `UI.asi` names the views whose campaigns the player has not yet
opened through a third export, `Render3D_MenuLocked(names)` (a comma-separated list,
held until the next call; Render3D.log: `menu: locked views: <list>`;
ui/MENUS.md, "The menu scene"). Every group a locked view looks at is locked: its
ships sit at their `LockedAt=` (where they always sit, without one), turned to their
`LockedYaw=` (the ship's, else its `Yaw=` plus the group's), and with `LockedCloak=1`
they are held cloaked: drawn whole in the cloak's glass, as the hulls draw a fully
cloaked ship in a mission (the engine's state 2: refraction with the colour split, rim
and shimmer, no hull left, `CloakHullOthers`), never decloaking, so no sound plays
(`ShipAt::cloaked`; a ship merely waiting to decloak is not drawn at all). The built-in
map holds the Klingons cloaked, shapes shimmering in the Metreon gas, and sinks the
Borg cube and the Diamond into the Mutara gas, the cube turned a corner towards the
camera (`LockedYaw=70`, judged on the bench among 20..145: the left face catches the
Key, the right one stays in shade and the bottom corner shows; at its `Yaw=20` a face
met the camera square on and it read as a flat tile).
A departure ignores the list (a mission is loading, so its campaign is open).

**The departure.** While a mission loads from the campaign selection, `UI.asi` asks for
the view under its loading screen through the second export,
`Render3D_MenuWarp(name, x, y, w, h)` (ui/LOADING.md, "Over the menu scene"). The first
call after one of `Render3D_MenuScene` begins a departure from that view (Render3D.log:
`menu: departure from <view>`), and it goes on as long as the calls come:

- The view's group's ships leave front first (`Stagger` apart, after `Delay`; a ship
  with `Warp=0`, the starbase, stays), each along the mean of their noses: impulse
  (`Speed`), then over `Ramp` seconds the hull's warp stretch, the shader's own (a
  plane across the motion sweeping from bow to stern, the hull in front of it
  stretched by up to `Stretch`), and it is gone, with a flash of the stock
  `warp_sprite` where it was (`Flash` times its radius, for `FlashTime`), faded round
  from near its centre by a shader of its own (`flash_ps`) at no more than 1.5 times
  stock's light: the art's core is a flat white diamond and its halo reaches its
  border, and brighter than that both showed as a hard-edged box. The
  stretch is forward only: the engine's shift of the hull back, which suits a ship
  seen from the side in a mission, threw the stretch across the screen towards a
  camera behind the ships.
- The camera swings from where it was to behind the ships, looking the way they go
  (`Turn`; over the top, so it does not pass through them; `WarpEye`/`WarpTarget` set
  it by hand), then follows them, accelerating (`Follow`).
- Once the last ship has gone (`TunnelAt`), the tunnel comes in over `TunnelIn`:
  `tunnel_ps` over the whole view, premultiplied over (1 - alpha), so it darkens the
  scene under it by `Dim` as its streaks come up, and the field of view widens by
  `Widen` degrees. Its streaks are stars on cylinders about the camera's axis rushing
  at the eye (a star at radius rho and depth s is seen rho / s off the axis, so the
  depths it covers in a frame draw a radial streak), one star per angular cell in four
  rings, so each pixel looks only at its own cell of each ring: a fixed loop with no
  branch (the rule the shaders learned from vkd3d, which wrote the Direct3D 9 bytecode until 1.0.0). A soft wall of gas in the group's `WarpColour` streams past
  behind them (the Borg's green). The tunnel runs as long as the load does.

The departure keeps the map's clock and the scene's passes; it only poses the
departing ships and moves the camera, so the frame costs what a menu frame does, plus
one full-screen pass.

**The gas.** A `[Nebula.*]` field lays a nebula class's centres on a jittered grid over
an ellipse in its group's frame, as a map's nebula is laid out, and the world module
draws them as that class's gas from its recipe in `nebulae/`. The menu's classes are
its own (`<odf>@menu`), so the map can set some of the recipe's keys for the menu
(`KeyShare`, `Scatter`, `Opacity`, `Brightness`) and a mission's nebulae of the class
keep theirs. **A ship's shadow in the gas shows only from the side and only where
nothing lit is behind it**: a view ray adds up all the gas along it, so a shadow seen
along its length, or in a thick field, or in front of lit gas beyond (seen through
it), is diluted to nothing. The Borg scene's Mutara is therefore a band about 600
units deep laid along the cube's shadow (its own group turned with the shadow's way,
36 degrees from +x), seen 71 degrees from the sun, with the recipe's own look (a
stronger `KeyShare`, `Opacity` and `Brightness` darkened the shadow but the user
preferred stock). The queen's Diamond (`bqueen`) hangs close to the camera, below and
left of the cube, a silhouette against the gas.

Two things the menu found in the hulls' shadow map, both in missions too: a model
that is no instance's (`inst` 0) read its face limit at 0 and cast nothing (now it
casts all its faces, as `draw` already did); and cut-out meshes (a `default` mesh with
groups that are not self-illuminating) were left out of the map, half of a Borg fusion
cube among them, and now cast what the hull pass draws of them, their texture's alpha
cut at 0.5 as `ps_hull` does.

**The map.** `content/menu/map.ini` is built into the plugin; `Menu\map.ini` in the game
directory takes precedence and is read again within a second of an edit (copy it there
to tune it live on the bench). Render3D.log lists what it read (`menu: map`), each
change of view (`menu: view <name>`) and each hull's radius and cut-out parts the first
time it is posed (`hulls: menu scene:`). Keys:

| Section | Key | |
|---|---|---|
| `[Scene]` | `Sky`, `SkyTurn` | a sky recipe's name; radians a second it turns |
| | `Stars`, `StarSize`, `StarSeed` | how many stars, their half size in radians, the seed |
| | `Sun`, `SunDistance` | the sun's ODF (none: no sun) and its distance from the eye |
| | `KeyAxis`, `KeyColour`, `FillAxis`, `FillColour` | the two lights: the way each travels, and its colour |
| | `Blend` | seconds the camera takes from one view to the next (2; the map's 1.6) |
| `[Warp]` | `Delay`, `Stagger`, `Ramp` | a departure: seconds before the first ship goes (0.9), between one and the next (0.18), from impulse to gone (0.75) |
| | `Stretch`, `Speed` | the warp stretch at the end of the ramp (30); the impulse's acceleration, units/s² (120) |
| | `Turn`, `Follow` | seconds the camera takes to swing behind the ships (1.3); its acceleration after them, units/s² (260) |
| | `TunnelAt`, `TunnelIn`, `Dim`, `TunnelSpeed`, `Widen` | the tunnel: seconds after the last ship has gone (0.1), to come in (0.9); how much it darkens the scene (0.85); how fast its streaks come (1); degrees the field widens by (18) |
| | `Flash`, `FlashTime` | a flash's half size times the ship's radius (2.2), and its seconds (0.45) |
| `[Planet]` | `Class` | the planet class's ODF (`PB_CLSSM`); none: no planet |
| | `Radius`, `At`, `Axis`, `Spin`, `Clouds`, `Orbit` | its radius, its centre, its pole, how much faster than `Orbit` its ground turns, the clouds' extra turn, the orbit's radians a second (the sky's turn is `SkyTurn`) |
| `[Group.<name>]` | `At`, `Yaw` | the group's place on the map and its turn about +y in degrees |
| | `CloakView`, `CloakDelay`, `CloakTime`, `CloakStagger`, `CloakSound` | the view whose arrival decloaks its ships; the seconds before the first starts (`Blend` x 0.7), each takes (2.5), between one and the next (0.5); the sound heard once as the group's first ship begins (`clkdisen.wav`; empty for none) |
| | `WarpColour` | the tunnel's tint when the group departs (0.45,0.62,1) |
| | `LockedCloak` | 1: its ships are held cloaked, drawn in the cloak's glass, while a view of the group is locked (`Render3D_MenuLocked`) |
| | `LockedYaw` | degrees each of its ships turns by, on top of its `Yaw`, while the group's view is locked (0); a ship's own `LockedYaw` takes precedence |
| `[View.<name>]` | `Group`, `Eye`, `Target`, `Fov` | in the group's frame; vertical field in degrees |
| | `Sway`, `SwayPeriod` | the drift's amplitude and period |
| | `WarpEye`, `WarpTarget` | the camera a departure from the view swings to, in the group's frame (default: behind the departing ships, looking the way they go) |
| `[Ship.<name>]` | `Group`, `Odf`, `At`, `Yaw`, `Scale`, `Bob`, `Phase`, `Team` | its group; the ship's ODF (its `baseName` and `ScaleSOD`), its place in the group, a turn in degrees, a scale on top of `ScaleSOD`, how much it bobs, its phase, a team colour for its `team*` groups |
| | `Warp` | 0: stays when its group departs (a station) |
| | `LockedAt` | its place in the group while the group's view is locked (default: `At`) |
| | `LockedYaw` | its turn in degrees while the group's view is locked, in place of `Yaw` (default: `Yaw` plus the group's `LockedYaw`) |
| `[Nebula.<name>]` | `Group`, `Odf`, `At`, `Size`, `Spacing`, `Jitter`, `Seed` | its group; a nebula ODF with a gas recipe; the field's centre (its plane's height is its y); half extents in x and z; the grid's step (the radius x 1.2) and jitter (0.35); the seed |
| | `Radius` | the class's radius (the ODF's `effectRadius`, else 300) |
| | `KeyShare`, `Scatter`, `Opacity`, `Brightness` | over the recipe's, for the menu's class only (one set per class: the first field of the class that sets a key) |

## Claims

render3d registers with Display8 at load (contract 1.1, `gpu::Gpu::register`, once its
frame seam is in; `display/README.md`, "Claims") with `registry::claims`, which says what it
draws now: the OR of every running module's `Module::claims()`. Each claims its own
category (`sky`, `hulls`, `world`, `particles`, `effects`, `ordnance`, `specials`), and
`overlays` claims `fog` and `selection` as `[Overlays] Fog` and `Selection` say. A module at 0 (kept for the menu scene only), one standing down
beside its old plugin, and anything before the first frame are not claimed. Display8
drops none of the engine's 3D draws for these: a module that draws a category skips
the engine's step, so whatever the engine still issues (a SOD handed back, a class no
module takes) is drawn by passthrough into render3d's published pass. `Display8.log`
names what render3d claims; `display-claims` runs it with `sky=0`. What another plugin
claims is asked through `gpu::Gpu::claimed_by_others`: the selection bubbles
(`overlays`).

## Modules

### Module interface

A module is one `Module` (`core/src/module.rs`, version 2), made at load and kept in
the registry (`core/src/registry.rs`) in `priority()` order: world 10, particles 20, hulls
30, effects 40, ordnance 50, specials 60, sky 70, overlays 80 (two modules never share
one; `init` asserts it). The first module whose `step`, `object` or `feature` returns
true draws that part; a part none takes is the engine's. Besides the per-view calls
(`lights`, `begin_view`, `step`, `object`, `feature`, `end_view`, the menu scene's
`menu_begin`/`menu`) and the device's (`device`, `reset`, `forget`,
`formats_changed`), a module answers:

| Call | What it is |
|---|---|
| `name()` | its `[Modules]` key and log prefix |
| `priority()` | dispatch order, ascending |
| `claims()` | the `gpu::display::CLAIM_*` it draws while it runs; `registry::claims` is the OR over running modules ("Claims") |
| `rows()` | its settings rows, its fragment's `ROWS` (below); `&[]` if none |
| `configure(ini, said)` | reads every section it owns, at load and at every `Render3D_SettingsChanged`, whether it runs or not; notes what changed for the log line |
| `superseded()` | the old plugin it stands down beside |
| `in_menu()` | has a part in the menu scene, so it keeps its device while off |
| `provide(id, services)` | registers what it draws for the others (`r3d_core::services`), under its own id, once, at init |
| `hooks(h)` | installs its hooks through `r3d_core::hooks` (below), once, at its first switch-on |
| `on()` / `off()` | switched on (load or live, after `hooks`) / switched off live (drop the cosmetic state; `forget` follows at the next view) |

The registry holds each module's on flag (an `AtomicBool`, which is also its hooks'
gate) and its `ModuleId` (its index, stable within a run: a module that stands down
keeps its place). `registry::running(name)` and `enabled(name)` remain for logging only:
module code asks for a capability (`services::hulls()` and so on, `Some` only while
the provider runs), never for another module by name (`lint-layers.sh`, rule 3).

**Hooks** (`core/src/hooks.rs`). Every patch of `Armada2.exe` goes through
`Hooks::install(Hook { owner, used_by, kind, site, to, chain })`. A `Site` is the
function's name in the game's map, its address and what stock has there (`expect`: the
call's target, or the slot's value; 0 for a slot of an object outside the executable,
such as the d3d8 device's `Reset`). It is checked before anything is written: a `Call`
must call `expect`, a `Slot` must hold it, and with `chain` another plugin's target (one
outside the executable's code) is accepted too, logged `(chained)`. One line per hook:
`hooks: <owner> <used by> at <symbol> (0x..) installed`, or `... not stock: <owner>
stands down for <used by>`. `install` returns the `Prev` the detour calls on, never a
constant; every detour starts with `if !GATE.open() { return prev(args) }`, and one that
replaces an engine function whole calls `prev` while its gate is shut. Hooks are
installed at the owner's first switch-on and never taken out (another plugin may have
chained on top since); a module that is off at load installs nothing. A module whose
`hooks()` could not install one it cannot draw without calls `h.stand_down(why)` and is
not switched on. The frame's own hooks (the two seams of `Armada_RenderAllOurStuff`,
`DestroyRenderSurfaces` and the device's `Reset`) are the core's, behind a gate that is
always open (`Enable=0` installs none).

**The engine profile** (`profiles/gog-1.1/render3d.toml` for the core and frame,
`render3d.<module>.toml` for each module; `tools/engine/README.md`, `crates/a2engine/README.md`).
No address of `Armada2.exe` is in the sources: each is a `sym::` entry of a generated
`syms.rs` (the core's in `core/src/`, a module's in its crate, so a module's symbols leave
with it in a subset build), read with `at(sym::X)` (0 unless the hook that uses it is
ready). `Hooks::ready(&TABLE, hook::X)` checks a hook's entries in live memory before the
owner writes or reads anything, and logs `engine: <hook> stands down: <symbol> <why>`; a
`Site` comes from `Site::of(&TABLE, sym::X)`. A module's `hooks()` readies its
`<module>.reads` first and calls `h.stand_down` when it is not ready, so the engine draws
that part as stock; its note reaches the Refit screen through the row `Modules.<module>`
(`Render3D_SettingsNote`). On an unknown `Armada2.exe`, or when `core.reads` or
`frame.seam` is not ready, render3d stands down whole. Engine reads live in
`core/src/engine/**` and each module's `hooks.rs` (`lint-layers.sh`, rule 1, strict).
**Struct offsets are the profile's too**: every field offset, vtable byte offset and
engine size those files read is a `field` or `value` entry of the core's or the module's
profile file (ids `<module prefix>_*`: `pt_`, `sky_`, `sp_`, `hl_`, `wd_`, `ov_`, `or_`,
`ef_`), read at run time as `field::X()` (or the module under an alias,
`use crate::syms::{field as off}`), the matched build's value, so a build whose structs
differ gets its own and a field it lacks stands its hook down (`crates/a2engine/README.md`,
"Fields and values"). An offset a module shares with the core (an object's instance,
class, flags, team and on-screen byte, an instance's scale, clock and database, a list's
or a sprite's layout, the light registry) is read through the core's helpers, never
re-declared. Code that walks many objects reads its offsets once, outside the loop.
`lint-layers.sh` rule 1 fails on a literal offset anywhere (`field(x, 0x..)`,
`rd(x + 0x..)`, `(x + 0x..) as *`, `const X: usize = 0x..`), the glue included; the
offsets written in comments here and in `hooks.rs` say where each was found.
Render3D.log names the executable (`engine: <identity>`), the build and
`engine: <part> symbols: N/M resolved`. Hook ids (`render3d.<id>`), what each gates and the
finding it rests on:

| Hook | Gates | Finding |
|---|---|---|
| `render3d.core.reads` | the engine state every module shares: the object, light and feature lists, instances, sprites, sound, team and game-setup reads | the renderer stands down whole if these are not stock |
| `render3d.frame.seam` | `Armada_RenderAllOurStuff`, called from `Armada_RenderFrame` and `CinematicView::RenderView`, both re-pointed at the renderer's body | "How the frame works" |
| `render3d.frame.steps` | the calls inside the stock body that the frame's steps replace one by one | "How the frame works"; not ready: the stock body runs whole |
| `render3d.frame.obj_draw` | Roots only (absent in stock, so it stands down there): the call at 0x598297 in the object loop, which FleetOpsHook writes as it loads and which it makes for each object, culling and then drawing through `GameObject::Render`; accepted as `chained` | PLAN-FLEETOPS.md, "FO-5: render3d on Roots". The object step (`frame/steps.rs`) reads the call's target every frame and calls it as `stdcall(object, camera)` for every object that is not a background object, never writing there; a module may still take an object the engine has not culled first (`registry::object`). The object is the step's current object (`steps::current`) around the call, for the hull claim. Offline (`a2test syms`) the file still holds stock's bytes there, so only the game's log says it is ready (`frame: the object step calls through the loop's draw site`) |
| `render3d.frame.destroy` | `DestroyRenderSurfaces` chained, so render3d lets go of the device before the engine does | kept for the device Display8 hands out; native Direct3D 9 (gone in 15.0.0) kept a device alive while anything made on it was held |
| `render3d.view.distance` | the load of the culling distance in `RTS_Configure` and the far plane and culling distance it parses (`View.Distance`) | "View distance"; not ready: the engine keeps its view limits |
| `render3d.hulls.instance_draw` | the call of `ST3D_Instance::RenderInternal` at 0x4d5a11 in `GameObjectInstance::RenderInternal`, and the callee's entry | where a hull is claimed on Roots, after the per-instance node and texture writes of stock and of FleetOpsHook, which neither Roots' file nor FleetOpsHook changes; on Roots every hull draw, FleetOpsHook's bump-mapped replacement included, is nested under this one call, and no other route reaches it with a hull. The instance does not point back to its object: the owner is the object the object step is drawing through the `frame.obj_draw` call, confirmed by the object's instance being the one at the call; planets (which reach the call through the craft instance), construction rigs (FleetOpsHook gives them the common `GameObject::Render`, so the stock rig draw is not called from the loop) and other modules' objects are skipped by that test. Holograms and FleetOpsHook's overlay models do not pass here and are the engine's. Installed only where the profile's value `hl_claim_instance` is 1 (Roots); stock (0) claims at the object and patches nothing here. Claimed, the hull is posed and drawn as in `object`, after the engine's own frustum test and on-screen byte; the engine's function is not called, as on stock with hulls on (it returns nothing its caller reads), and the object's billboard effect is left to the call in `GameObject::RenderInternal` (the particles module's, as on stock, since step 7). What the engine and FleetOpsHook wrote into the shared database for this instance just before is honoured (FO-5 step 5): for each node of the SOD the module looks the database's node up by name (`ST3D_Database::FindElement` on its node list, `fn_find_element`, `hl_node_list`); a node whose render flags (`+0xbc`, bits 0 and 1, the bits stock's `GameObjectInstance::RenderInternal` sets on the lights node) say hidden is hidden with what hangs from it, and a mesh whose first texture (`+0x114`, `+0x11c` entries; a texture is a database element, named at `+0x8`) is not its SOD's is drawn with that one: Roots' per-ship logo on the `stlogo` node, shown with the ship's own texture or hidden, comes through this way without a read of FleetOpsHook's memory. The sprite nodes are drawn by the module on Roots too (the database draw that drew them is the one replaced), and the wave light stays the module's (`mods/hulls/src/claim.rs`) |
| `render3d.hulls.reads` | the hulls module: class `Render` / `RenderInternal` functions and vtables, the fade, culling and wave-light reads and calls | hulls draws in `GameObject::Render`'s place and installs no hook; not ready: the engine draws the hulls |
| `render3d.sky.reads` | the sky module: the background name, database, starfield, far planes, `GameObject::Render` and the plain database vtable | the sky takes `Step::Background`; not ready: the engine's sky |
| `render3d.specials.reads` | the special-weapon effect list and the `Render` functions and vtables of the timed effects | only the timed effects draw; others go back to the engine |
| `render3d.particles.reads` | the particles module's object, ordnance, nebula and freighter reads and calls | plain reads |
| `render3d.particles.particle` | `ST3D_DeviceDirectX8::RenderParticle` (device vtable slot 50) | the engine's draw of an owned object's particles would double ours; required |
| `render3d.particles.billboard` | the call of `ST3D_Instance::RenderSpriteNodeEffect` in `GameObject::RenderInternal` | skipped while one of our views runs; optional |
| `render3d.effects.reads` | the effects module's list walks, vtables and calls, and the `RenderChunks` call site | the walks of the old Explosions, Shields and Repair plugins |
| `render3d.effects.repair` | the `AddParticle` call in `Craft::RepairSystem` and the `DrawScaled3D` call in `NodeParticleEffect::Render` | the new tail of the particle list is a weld |
| `render3d.effects.shields` | the weapons' calls of `CreateShieldHit` | records which weapon made a hit and its damage |
| `render3d.effects.shield_models` | the call FleetOpsHook writes in `ShieldEffect::CreateShieldHit` (+0xea) to pick a hit's model | on Roots the hit's model is the struck class's own (its ODF), never the shield database's; chained, the model each pick sets is noted against the shield type asked for, so the shields know a hit by its type; not ready (or stock, which has no such call): only the database's models are known, and Roots' hits are left to the engine (see "Shields on Roots") |
| `render3d.effects.debris` | the `Fragment` calls in `Craft::Explode`, the Evolver's cocoon and `BuildingEffect::SafeDestroyConstruction` | the faces are drawn as our own debris; all or none |
| `render3d.effects.debris_callee` | `ST3D_Instance::Fragment` itself | on Roots FleetOpsHook owns the call in `Craft::Explode` and decides whether it runs; the module takes over the callee at its entry, armed only while the build's companion module is the caller (`ef_debris_at_callee`; FO-5 step 5, see "effects: Hooks") |
| `render3d.ordnance.reads` | the ordnance module's `Entity` calls, list head, vtables and each stock `Submit` | a shot's class is told by its `Submit` |
| `render3d.ordnance.warp` | the `DrawScaled3D` call in `WarpSpriteEffect::Render` and the body's `RenderWarpSpriteEffects` call | notes stock's warp flash for ours; optional |
| `render3d.ordnance.shots` | `Entity::Render`, slot 11 of an ordnance vtable, taken per class when first met | the profile holds the `Render` check; required for the per-class slots |
| `render3d.overlays.reads` | the overlays module: terrain, fog and shroud grids, profile flags, game-setup, selection and team calls, the ghost list, the tactical camera | plain reads |
| `render3d.overlays.posts` | the five calls of `DrawHeightPost` | handed back to the engine while the gate is shut; not ready: the engine's posts |
| `render3d.overlays.posts_callee` | `DrawHeightPost` itself | on Roots FleetOpsHook owns its five calls (and skips them in a clean screenshot); the module takes over the callee at its entry, armed only while the build's companion module is the caller (`ov_posts_at_callee`; FO-5 step 5, see "overlays: Hooks") |
| `render3d.world.reads` | the world module's vtables, nebula and ghost lists, population tables, culling distance and name table | plain reads |
| `render3d.world.lights` | the call of `Nebula::Simulate_Nebula_Lights` in `Nebula::Simulate` | the nebulae's lights become ours; optional |
| `render3d.world.cull` | `NebulaInstance` vtable slot 3, `FrustumTest` | the nebulae's cull; optional |
| `render3d.world.ghosts` | `GameObjectSpirit` vtable slot 3, `Render` | hides a fogged nebula's billboards; optional |

**Services** (`core/src/services.rs`): `HullDraw` (hulls: `draw_instance`,
`draws_plain_sod`, `placeholder`, `shadow_map`), `WaveDraw` (ordnance's shockwave),
`PostDraw` (overlays' height posts) and `GhostDraw` (world's planet ghost). A consumer
that gets `None` hands the object or effect back to the engine, as it did when the
provider was not running: the bees and construction draw nothing of their own without
hulls, a shockwave is the explosions' own ring without ordnance, a build ghost's height
post is the engine's `DrawHeightPost` without overlays, and a fog-of-war planet ghost is
overlays' own sphere without world. Two asks are not tied to "running": the nebulae's
shadow map (`services::hulls_any()`) is lent to the menu scene's gas while hulls
only keeps the menu's part, as before, and `services::volumes()` tells the
particles module that world draws the nebulae.

**Shared helpers** (the cross-module paths are gone: `lint-layers.sh --strict mods
--rules cross,byname` passes and CI holds it). What other modules used to take from the
hulls, effects, ordnance and particles modules lives in the core, and takes the engine
reads from `core/src/engine/`:

| Helper | Where |
|---|---|
| `frustum_test`, `instance_scale`, `set_on_screen`, `team_colour`, `element_name`, `render_fn`, `instance_visibility` | `scene::instance` (reads in `engine::instance`) |
| the animation values `value` / `Value` | `scene::anim` |
| the per-view posed objects (`Posed`, `posed`, `norm_or`) | `scene::pose`; the hulls module fills the cache, effects and specials read it |
| `Cam` | `scene::camera` |
| the engine point-light pool (`LightPool`) | `scene::lights::Pool` (reads in `engine::light`) |
| `fx_desc` | `gfx::fx` |
| `hash32` / `rnd` (render-side only, never `g_random`) | `math::rand` |
| the ini section reader (`Sec`) | `config::Section` |
| the stock sprites' batch (`Sprites`) | `scene::sprites` (reads in `engine::sprite`) |
| the particles' feed | `r3d_core::particles`: producers push, the particles module takes the queue at its translucent step and at the end of the view; `live()` is false without it |

**Recipes** (`core/src/recipes.rs`): `recipes::text(kind, name)` finds a recipe in
the game directory (`Sky\<name>.ini`, `Surfaces\`, `Nebulae\`, `Menu\`) first, then the
one a module built in (`recipes::builtin(kind, &[(name, include_str!(..))])`).
The sky's recipes, the menu's `map.ini` and the built-in `ordnance.ini` (whose sections
`Render3D.ini` overrides one by one, ordnance's own rule) are read through it.

**Files** (`core/src/files/mod.rs`): every game file render3d reads (`assets`, `recipes`,
the world module's texture means) goes through one `a2fmt::vfs` file system, built at the
first lookup, by a virtual path: `textures\`, `sod\`, `sprites\`, `odf\`, the recipe
override folders `sky\`, `surfaces\`, `nebulae\`, `menu\`, and `data\`, the game directory
itself (`data\RTS_CFG.h`). Its `LAYOUT` is the mount list,
each virtual folder and the game-directory folders it searches in order (`Textures\RGB`,
`SOD`, `Sprites`, `odf`, `Sky`, ...); the first four rows are `games/armada2.toml`'s
`[files]`, which `tools/layers.py --check` holds them to. `assets` indexes each of its four
folders once (every file under it, recursively, by lower-case name), source by source in
search order, so a name is the top source's that has it wherever it sits under the
folder and, within a source, the first found; it keeps each lower version too, and reads
through the same file system, so on Armada II (one source a folder) the lookup is the
one it was before the file system, case-insensitive as the engine's. An ODF is read with
`a2fmt::odf`'s `load_layered`, so an include of its own name is its next version down
(Fleet Operations' mods; a loop, cut, where there is one version). An archive kind (`fpq`, `zfs`) would
mount behind a folder and log why it serves nothing (`files: fpq ... no reader ...`);
Armada II has none. Fleet Operations: Roots' executable (`ArmadaL.exe`, by name: `GAMES`)
takes `LAYOUT_FO_ROOTS` (`games/fo-roots.toml`'s `[files]`): the textures in `Textures\`
itself, TGA and DDS (a name with both is the TGA; `a2fmt::dds` decodes DXT1/3/5 and the
uncompressed forms, and sides that are not powers of two are allowed there), and
`ODF.fpq` mounted behind `odf\` (`files: ODF.fpq mounted at odf: N files`).

**Mods** (`core/src/files/mods.rs`, FO-5 step 6). Roots has mods: folders
`Data\Mods\<name>\` with an `info.ini`, one made active by `-mod <name>` or `/mod <name>`
on the game's command line (`games/fo-roots.toml` `[files] mods`; render3d reads the
command line with `GetCommandLineW`). The game's file system then finds each file by
name in the active mod's folder, then in its `ParentMod`'s and up the chain, then in
`Data\` itself, unless a mod in the chain is `standalone=1`, when `Data\` is not searched;
within each folder its `ODF.fpq` and its loose files. render3d mounts the whole
`LAYOUT_FO_ROOTS` list (and `ODF.fpq`) once per folder of that chain, each a level of the
file system above the one under it (`Layered::mount_level`), so textures, SODs, sprites,
ODFs and `RTS_CFG.h` all come from where the game takes them. It logs the chain
(`files: active mod 'X': X, then the game's own folder`, or `(standalone: ...)`, or `files:
no active mod ...: the game's own folder alone`). The rules were read off Roots 5.0Z11's
file system and its mod list: the shipped STA2 Classic mod is standalone and runs on
Roots' own `ArmadaL.exe` (it ships none), and its Enhanced Gameplay mod is a data-only
child of it. Within a folder render3d keeps loose over packed, as on Armada II; Roots'
`odf\` folders are empty, so the order between them is not yet seen to matter.

**Layers.** The engine's addresses, call sites, vtable slots and field reads belong in
`core/src/engine/` (the core's glue) or a module's own `mods/<m>/src/hooks.rs`; a module's
game data names in its content set's table (`render3d/content/sets/<set>/<m>.ini`, read
by its `content.rs`: "Content sets"); its recipe files under `render3d/content/`.
`render3d/lint-layers.sh` counts what is elsewhere (report mode; `--strict DIR...` fails
on any hit under those directories, rules 1-4: CI holds `core` and `mods` to it; `--rules
cross,byname` limits it to those rules). A name rule 4 finds that is the engine's file
format rather than a set's (`".odf"`, `".tga"` beside a name) stays with
`// lint-allow(content): <reason>` on its line or the line above.

### Modules and settings

Every module is made at load and has an `on` flag from `[Modules]` (and the overlay,
`RefitPlayer.ini`), read through the core's row `Modules.<name>`. Dispatch (`lights`,
`begin_view`, `step`, `object`, `feature`, `end_view`) skips a module that is off, and
`registry::running(name)` is false, so the engine's draw of that part runs (through
Display8's passthrough). The menu scene's calls (`device`, `reset`, `menu*`) still reach
`hulls`, `sky` and `world` while off (`Module::in_menu`), since nothing could hand the
front end's scene back; `hulls` clears the menu's shadow map and posed ships at each
mission view (`idle_view`).

`Render3D_SettingsChanged` (called by the UI hub after an overlay or ini change) reads
every module's `configure`, then every module's flag. A module switched off
gets `off()` and lets go of its GPU objects at the next view; one switched on gets its
`hooks()` the first time, then `on()`, and its `device()` at its next view if it missed
this device. A module whose hooks are not yet gated by its flag (`NOT_YET_LIVE` in
`mods/mod.rs`, empty now that every module hands back) refuses a live change of
its flag, logged `settings: Modules.<m> changes at the next launch (hooks not yet live)`.
Every `Modules.*` row is live, as are `World.Planets/Rocks/Nebulae/Surfaces`, `Nebulae.Lit`
and `Overlays.*`.
Checked on the bench at RMSE 0: live `hulls=0`, `sky=0` and `World.Surfaces=0` against
the baselines of the same keys at load (`testbench/scenarios/render3d-settings-live.md`),
and `Modules.world=0` and `Modules.overlays=0` against `-world0` and `-overlays0` on the
showcase, each flipped back to the base (in the nebula scene the flip is not exact, rmse 20 off and
1.2 back on: the engine's nebula light simulation did not run in the ticks before the pause)
(`render3d-live-world.md`, `render3d-live-world-nebula.md`).

**The rows are split per module.** `render3d/settings.toml` holds the core's: `MSAA`,
`HDR` and the eight `Modules.*`. A module with rows has a fragment beside its code,
`mods/<m>/settings.toml` (`module = "<m>"`, `tools/settings/README.md`,
"Fragments"; today world's `World.*` and `Nebulae.Lit`, and overlays' `Overlays.*`),
generated into `mods/<m>/src/settings_table.rs` (`ROWS`, one constant per row). Its
`rows()` returns them. `Render3D_Settings` returns one contiguous table, built at load
(`lib.rs`, `settings_rows`): the core's rows, then each built-in module's in priority
order, copied (a module not built in has none, and with `Enable=0`, when no module is
made, only the core's are exported). The code reads keys through the tables' own row
constants; `Render3D_SettingsNote` matches a row of the exported copy by its section and
key. Render3D.log says `settings: N rows exported (M the core's)`.

**Which module reads which section** of `Render3D.ini` (each section has one owner;
`[Lights]` is split by key, and `lint.py` rule 7 holds the fragments to it):

| Owner | Sections |
|---|---|
| core | `[Render3D]` (`Enable`, `Log`, `MSAA`, `HDR`, `Bloom*`, `Exposure`, `Shoulder`, `Inset`, `Steps`, `Timing*`), `[Modules]`, `[Bench]` (`Clock`), `[Lights]`: the cluster keys (`Clustered`, `ClusterTile`, `ClusterSlices`, `ClusterLights`, `ClusterPerColour`), the scene lights' keys (`Replace`, `KeyColour`, `KeyAxis`, `FillColour`, `FillAxis`, `SkyLight`; `scene/scene_lights.rs`, the hulls module's until 0.43.0: they light every module, so they must not depend on hulls being built in), their data type and their application (`scene/lights.rs`) |
| world | `[World]` (the planets', rocks' and nebulae's switches and keys: there is no `[Planets]` or `[Rocks]` section), `[Nebulae]`; the recipe files' `[Surface]` and `[Nebula]` |
| particles | `[Particles]` |
| hulls | `[Hulls]` (the `Borg*` look keys with it) |
| effects | `[Bees]`, `[Construction]`, `[Debris]`, `[Repair]`, `[Sparks]`, `[Explosions]`, `[Shields]`, `[Shell.<name>]` |
| ordnance | `[Warp]`, `[Ordnance.<odf>]`, `[Model.<odf>]`, `[Wave.<odf>]` (over the built-in `content/ordnance/ordnance.ini`); the `[Lights]` beam keys (`Beam*`, `ConeSpot`, read by its `configure` in `ordnance/lights.rs`, `Beam`) |
| specials | `[Specials]` |
| sky | `[Sky]` |
| overlays | `[Overlays]` |

`Render3D_SettingsNote` (the optional C5 export, `tools/settings/README.md`) tells the
Refit screen why a part is not running: with a NULL row, why render3d as a whole stood
down (`Enable=0`, a seam that is not stock's, or no targets or post shaders for the
device: the reasons `lib.rs` and `frame` log, kept in `DOWN`); with a module's row,
"Stands down while Sky.asi is loaded" for a module `mods::settle` stood down for its
superseded plugin (`mods::stood_down_for`). NULL when nothing is wrong, so a run with no
stand-down draws exactly as before. It only reads, into one static buffer.

### effects

`mods/effects/src/`, on by default. The game features that are effects, hull debris,
and their lights; it ports `Explosions.asi`, `Shields.asi` and `Repair.asi`, whose
looks were settled with the user (kept as they were; the settings keep the plugins'
keys, in `Render3D.ini` sections `[Explosions]`, `[Shields]`, `[Repair]` and
`[Debris]`, plus `[Construction]`, `[Bees]`, `[Sparks]` and the `[Shell.<name>]` recipes; read in the module's `configure`, at load and at every settings change). Each part stands down while a stale copy of its old
plugin is loaded. Everything noted while the features run is drawn at `Step::Chunks`, after
the debris and the engine's own `RenderChunks` (called through its call site, so
construction's chunks and any plugin chained there still draw), when every opaque hull
is in the depth buffer.

- **Explosions** (`explosions.rs`). `ExplosionFeature::Render` (0x464ba0) walks
  `Explosion::explosionList` (head node at 0x7367c4) and submits each explosion whose
  slot 2 says it is visible (slot 15, `Submit(cam)`). The module does the same in its
  place, noting `FireballExplosion`s (vtable 0x6afbd4) and `ShockwaveExplosion`s
  (0x6afdac) and submitting any other class (`OmegaExplosion`) to the engine. The
  fireball is ray-marched, the shockwave a band of fire, as in the plugin. New: the
  scene is HDR, so the light above `HdrFrom=` (1.5) is kept in part (`Hdr=`, 0.5) and
  the white-hot core and the flash bloom by their energy; below it the plugin's look is
  unchanged. Each fireball also registers a point light (stock gives it none): up
  to full in its first 0.15 s, then down with the square of the time left, full out to
  the explosion's bounding radius and gone at `LightRange=10` radii (no less than 40
  units), in `LightColour` times `LightBrightness=4`. It is an ordinary point light,
  which the engine's own CPU lighting (planets, cloaking ships) takes too. Skipped
  while a stale `Lighting.asi` is loaded, which adds its own.
- **Shield hits** (`shields.rs`). `ShieldEffect::Render` (0x474910) calls
  `ShieldHit::DrawShield(cam, [0x736e2c])` (0x474070) for each hit of `m_shieldList`
  (0x736de8) in order; the module does the same for every hit that is not its own
  (types 0 and 1, and a special weapon's shell with a `[Shell.<name>]` recipe, named
  from the engine's table at 0x6b00e0). The hits' strength comes from the four weapon
  calls of `CreateShieldHit` (0x58bb95, 0x58cad8, 0x58d502, 0x58dd65), chained through
  thunks that note the ordnance in `ebx`. The shields-down discharges register a point
  light on the hull.
  Since 0.6.0 the hits take `ART-DIRECTION.md`'s direction ("Shield hits"): the shell
  is implied, lit only about the hit. A **mist** of uneven density lies over the glow
  about each hit, like lit vapour: four octaves of 3D gradient noise on the unit
  sphere (no grid; value noise drew walls in the nebulae), the domain bent once so the
  dense parts run in wisps, fixed to the shell and drifting slowly through it, never
  re-seeded, so it does not shimmer. Its light falls off as a Gaussian of the distance
  from the hit (`MistReach`, in shield radii), wide but gone well before the far side.
  It also modulates the glow's lattice and rim (`MistShare`). Where the lit shell
  turns to its silhouette everything fades over the last `Edge` of facing, so the lit
  area ends on a soft but definite curve rather than at its brightest. **Rays** only
  on the factions in `RayFactions` (Klingon): a sunburst of clear spokes from just
  outside the spot out to `RayLength`, four sines of whole numbers of turns about the
  hit, seeded per hit, each beating slowly in strength, so the burst breathes in place
  and does not turn. The faction is the ODF's first letter (`f`, `k`, `r`, `b`, `c`,
  `8`), as for the colours, whose defaults now follow the direction: Klingon pure green
  (`12,255,6`), Species 8472 orange (`255,140,40`). **The hull under a hit is lit**: each
  hit (the brightest `HitLights`) registers a point light where it struck the shell,
  in its shield colour taken `HitLightWhite` of the way to white, as bright as its
  glow (plus a quarter of its spot) times `HitLight`, full out to a tenth of
  `HitLightReach` times the ship's bounding radius and gone at that reach, its falloff
  raised to `HitLightPower` so the far hull stays dark; styled distinct, so several
  hits on one ship each light it. The ring and the rim are as they were; the next pass
  settles whether they stay. `[Shields]` keys (defaults): `Mist` (0.45), `MistReach`
  (0.35), `MistShare` (1), `MistContrast` (1.6), `MistScale` (3, cells across the unit
  sphere), `MistDrift` (0.08 a second), `Edge` (0.15), `Rays` (1.2, 0 for none),
  `RayLength` (0.55), `RaySharpness` (2), `RayFactions` (`Klingon`; any of
  `Federation, Klingon, Romulan, Borg, Cardassian, Species8472, Other`), `HitLight`
  (2, 0 for none), `HitLightWhite` (0.5), `HitLightReach` (0.8), `HitLightPower` (2),
  `HitLights` (6, at most 16).
- **Repair welding** (`repair.rs`). `NodeParticleEffect` stays the engine's (its
  `Render` also prunes particles of dead objects): the call to `AddParticle` in
  `Craft::RepairSystem` (0x4c90c0) is chained to note each weld, and the call to
  `DrawScaled3D` in `NodeParticleEffect::Render` (0x473767) goes through a thunk that
  reads the particle's node from `esi` and skips a weld (and, while the sparks below
  are on, every other particle too). The welds are drawn as in the plugin, and each
  burning arc registers a point light.
- **Debris** (`chunks.rs`), `Step::Chunks`: our own chunks (`spec-chunks-anim-lod.md`
  §1). The call to `ST3D_Instance::Fragment` (0x62eb20) in `Craft::Explode` (0x4c6bfc)
  is chained: for a craft whose SOD we read, the module notes its ODF, transform,
  animation clock and team colour and does not call the engine's (so the engine makes
  no chunks of it); otherwise the engine's runs. At the next `Step::Chunks` each mesh
  of the LOD shown breaks into patches as `CreateChunks` does (a random seed face, every
  face with a corner within half the mesh's radius), flying and spinning on the sim
  clock with stock's `ART_CFG.h` numbers (4 ± 2 s, 90 units/s, 4 rad/s, at most 1500),
  drawn through the scene layer's hull shading, and gone without fading at the end of
  their life. The Evolver's cocoon bursting is taken the same way: its four calls of
`Fragment` (0x4b07b4, 0x4b0872, 0x4b0914, 0x4b09b4, in `Evolver::mDoRemoveConstructionEffect`
and the other construction-effect functions; each breaks the whole cocoon, faces -1),
each re-pointed only while it calls `Fragment` itself; the cocoon (a `CocoonInstance`,
vtable 0x6b2630, of the database at 0x73fccc) at its instance's transform and scale.
  A construction cancelled or destroyed breaks up the same way: the call to `Fragment` in `BuildingEffect::SafeDestroyConstruction` (0x46a0f1) is chained too,
  its instance the one embedded in the construction (class at its `Info`'s `+0xd0`,
  named by `GameObjectClass::GetOdfName` 0x4ce370), only the faces built so far (the
  call's face count) breaking, in the SOD's order.
- **Construction** (`construction.rs`, `weldfx.rs`, `mods/effects/shaders/effects_construction.hlsl`).
  `BuildingEffect` (vtable 0x6afe4c) taken through `feature()`, in place of
  `BuildingEffect::Render` (0x469660). Stock: `BuildingEffect::m_list` (a `std::map`,
  head node at 0x736bd4, `_Nil` at 0x736c1c, node `+0x14` the `Info`) holds a
  construction per station being built. The `Info`: `+0` the builder's handle, `+4`
  the teams that see it, `+8` its colour, `+0x20` the station's instance (a
  `CraftInstance`, its class at `+0xb8`), `+0xb4` the handle `ApplyScaleEffects` is
  asked for, `+0xc8` the station's faces, `+0xcc` those built so far, `+0xd0` the
  class, `+0xe0` `weldingSpeed` (an edge's progress a second, default 1), `+0xe8`
  `weldingRadius` (the spark's half size, default 1; `weldingSpriteName` is `xspark`),
  `+0x100` the welding edges' list head. `BuildingEffect::Simulate` (0x469490) moves
  the faces built with the build time and, for each new face
  (`AddNewWeldingEdges`, from the faces' corners in the world), adds its edges (the
  list node: start `+0x14`, end `+0x20`, progress `+0x2c` 0..1, erased at 1).
  `Render` draws, while the builder is seen (or its team is one the player sees,
  `Entity::GetTeamsVisibleToUser` 0x4d0070, or the editor is on, or `View_Record` is
  2), when faces are built and `GameObjectInstance::DistanceCull` (0x4d5bb0) passes:
  `ApplyScaleEffects` (0x599760), the frustum test (slot 3), the face limit
  (`SetRenderFaceLimit` 0x62ea80: `+0x10`) and the instance in the builder's team
  colour (`GameObject::GetTeamColor` 0x4d5000, into `Storm3D+0x34`); the construction
  object's on-screen byte (`ConstructionRig::GetConstructionObject` 0x4afe80, `+0x25`;
  `ConstructionObject::Render` 0x4a3790 draws nothing); and one `xspark` sprite per
  edge sliding from its start to its end. Ours makes the same calls (each writes only
  the effect's own instance) and draws the station with the hulls module's look
  (`hulls::draw_instance`), faces limited as stock limits them, and two-sided: the
  engine's limited draw shows the inside of the part-built hull, which culling hid
  (seen on the bench). Each edge is welded as the repair welding welds (Repair.asi's
  look): a flickering arc with turning rays at the weld (on a share of the edges at a
  time, `ArcShare`), a hot spot, sparks streaking off and cooling, and behind it the
  seam, white-hot at the weld and cooling along the edge through the black-body ramp
  to dull red, which goes on cooling once the edge is done (`CoolTime`); a construction gone before
  all its faces were built (cancelled, destroyed: its chunks fly off) takes its seams
  with it. A few arcs per
  construction light its hull (`Lights`). With the hulls module off, or its SOD
  unread, the engine draws it. `[Construction]`: `Enable`, `Brightness`, `Flare`,
  `FlareMinPixels`, `ArcShare`, `FlickerRate`, `Stutter`, `ArcColour`, `HotColour`,
  `HotSpot`, `Seam`, `SeamWidth`, `SeamMinPixels`, `CoolTime`, `Sparks`, `SparkSpeed`
  (times the station's radius), `SparkSpread`, `SparkLife`, `SparkDrag`, `SparkWidth`,
  `SparkMinPixels`, `SparkBrightness`, `StreakTime`, `Lights`, `Light`, `LightColour`,
  `LightReach` (times the station's radius); `ABTest=1` (bench): while
  `Render3D.construction-off` exists in the game directory, construction, the bees and
  the sparks are the engine's, for a comparison at the same moment.
- **Worker bees and drones** (`bees.rs`). `WorkerBee` (vtable 0x6b017c) and
  `DroneColony` (0x6afe98) taken through `feature()`, in place of `BeeColony::Render`
  (0x4688a0). Stock: the colony's bees (its `GetBeeList`, vtable `+0x40`: a multimap,
  `_Nil` at 0x736bc4, the bee at node `+0x10`) are `ST3D_Instance`s of the bee's SOD
  (`workerBeeName` of the construction ship's ODF, `fbee` by default, its database at
  `ConstructionRigClass+0x494`; the drones' from `gdrones`, `DronesClass+0x2f4`),
  drawn by `ST3D_Instance::Render` while `BeeColony::ShouldDrawBee` says so (the owner,
  `+0xbc`, is seen); a bee that welds (`+0x8c` = 4) draws `weldingBeamSprite` (`+0x84`)
  as a ribbon (`RenderAsPolygon`) from itself (`+0x68`) to its spot (`+0xac`),
  `weldingBeamWidth` (`+0x88`) wide. Ours: the bee through `hulls::draw_instance`, lit
  as a hull, its strobes drawn as running lights, its emitters (the drones' steam) fed
  to the particles module (`particles::feed`); the beam as a crackling arc of light
  with the welding's flare, hot spot and sparks at the spot, and a light on the hull.
  A bee is a plain `ST3D_Instance`: its vtable has no `IsCraftInstance` slot (slot 10
  is RTTI data, and calling it hung the game), so `draw_instance` is told. `[Bees]`:
  `Enable`, `Beam`, `BeamColour`, `BeamMinPixels`, `Flare`, `FlareMinPixels`,
  `HotSpot`, `HotColour`, `Sparks`, `SparkEvery`, `SparkLife`, `SparkSpeed`,
  `SparkSpread`, `SparkBrightness`, `SparkMinPixels`, `FlickerRate`, `Light`,
  `LightColour`, `LightReach`, `Brightness`.
- **Sparks** (`sparks.rs`): every particle of `NodeParticleEffect` that is not a weld.
  Stock adds them with `NodeParticleEffect::AddParticle` (0x4733c0 at a hardpoint,
  0x4734c0 at a point in the object's frame; at most 200, 4 per object): a system
  blowing up (`Craft::DamageAlloc`, `Craft::DramaticDeathSequence`: `systemexpl1..7`,
  half size the bounding radius times `SYSTEM_EXPLOSION_SIZE_FACTOR` 0.14, at least 4,
  a second), a phaser's flare at its emitter (`Phaser::Init`, its class's sprite at
  `+0x160` in its colour) and a beam's hit (`Beam::Simulate`, `beam explosion1`).
  `Render` draws each with `ST3D_Sprite::DrawScaled3D` (colour `+0x20`, half size
  `+0x2c`) at its hardpoint (`Entity::GetWorldPosition` 0x4cff70) or its point
  (`+0x10`) in the object's frame; age `+0x34`, life `+0x38`. Ours, while repair.rs's
  thunk is in place: stock's sprite and flipbook drawn as stock draws it (the ordnance
  module's `sprite::Sprites`, its hottest texels past white), and new, a burst of sparks
  streaking off it, cooling (a system's blast more than a weapon's), and a flash of
  light on the hull. `[Sparks]`: `Enable`, `Hot`, `HotFrom`, `BlastSparks`, `HitSparks`,
  `SparkSpeed` (times the sprite's size), `SparkSpread`, `SparkLife`,
  `SparkBrightness`, `SparkMinPixels`, `BurstTime`, `BlastLight`, `Light`,
  `LightReach`, `LightTime`.
- **Nanites**: nothing to take. `NaniteEffect` (vtable 0x6affdc, `Render` 0x473280: one
  sprite flying from an object towards the camera) is never made: nothing in
  `Armada2.exe` calls its constructor (0x472fe0). What a player sees of nanites is the
  `Nanites` ordnance (a `Bullet`, `Submit` through its instance) and the
  `TimedScrambleInterface` it leaves on its target: the ordnance module's and
  `Step::Specials`.
- **Lights.** `Module::lights()` runs once per view after `GameObject_PreRenderAll` and
  `Ordnance::PreRenderAll`, before `scene::begin_view` reads the light list: the module
  registers ordinary `ST3D_Point_Light`s (`operator new` 0x652710, the constructor
  0x62f240, `RegisterLight` 0x62d250), which the engine deletes the records of at the
  view's end, so both the engine's draws and the scene layer see them.
- **Still the engine's**: construction and the bees while the hulls module does not
  run. `Step::Specials` is the `specials` module's.

### ordnance

The `ordnance` module (on by default; `[Modules] ordnance=0` leaves it to the engine)
draws every shot and the warp flash, in place of `Ordnance::RenderAll` (0x58e580,
`Step::Ordnance`) and around `RenderWarpSpriteEffects` (0x59c560, `Step::Warp`). It
supersedes `Specials.asi`, and stands down while that is loaded.

**The loop.** `RenderAll` walks `Ordnance::ordnanceList` (the `std::list` at 0x771fa8,
its head node's pointer at 0x771fac) and submits a shot when its owner (handle +0x5c,
`Entity::Get`, a game object, not +0x113) is not culled for this view (+0x183) and the
player can see it (`CanUserSee`, vtable slot 2), else the same of its target (+0x60),
else when the shot itself is within `cfgOBJECT_CULLING_DISTANCE` (0x6fcac0) of the
camera and visible. Ours makes the same tests in the same order, **and the one write
stock makes there**: a shot whose target has +0x10f set, or whose target has +0x112
while its owner (`GameObject::GetObjEvenIfDestroyed`) has not, forgets its target
(+0x60 = 0). That write is the engine's own behaviour in its render pass; leaving it
out would change the game. A submitted shot is then drawn by, in turn:

1. a **recipe** by its ODF (the class +0x34 keeps its `cPrjID` at +0x150,
   `cPrjID::GetOdfName` 0x6593a0): `Specials.asi`'s beams, bolts and fields, ported
   unchanged (`fx.rs`, `fx_ps` in `mods/ordnance/shaders/ordnance.hlsl`), on the sim clock;
2. **stock's own look, drawn by us** (`sprite.rs`), for the classes whose `Submit`
   (vtable +0x5c) is `Phaser` 0x57edc0, `PulsePhaser` 0x581cb0, `PhotonTorpedo`
   0x57f100, `PlasmaCannon` 0x57f580, `PoleronTorpedo` 0x5800d0, `Artillery` 0x5730a0
   or `ShieldRemovingTorpedo` 0x5853b0. Those draw the class's sprite (+0x12c) as a
   ribbon (`ST3D_Sprite::RenderAsPolygon` 0x63a810: from the beam's start +0xbc to its
   end +0xc8, the class's width +0x148 wide, turned about its length by
   (a − eye) × (b − eye); a pulse from its head +0xc8 back towards the start, at most
   the class's length +0x144) or as a camera-facing square (`DrawScaled3D` 0x63aa60:
   at the shot's position, `Entity::GetTransform` +0x24, half size +0x148), coloured
   by the shot (phaser +0xf0, photon, poleron and shield-removing torpedo +0xd0,
   pulse and artillery +0xd4; the plasma cannon sets none and keeps the sprite's own
   +0x24). Both end in `ClipAndRender3D` (0x63b4b0); what we read of it: the sprite's
   rectangle +0x38 (u, v, w, h, fractions of the texture), its flags +0x50 (0x10 not
   animated, 0x20 tiled along the ribbon once per its width +0x40), its material +0x54
   (blend factors +0x1c source, +0x20 destination, Direct3D's numbers), its texture
   +0x58 (named at +0x8); the flipbook's frame is what `ST3D_Sprite::Animate`
   (0x63a580) leaves in `m_texture_coordinate_offset` (0x7ad6d0), called where
   `ClipAndRender3D` calls it, so every flipbook shows stock's frame; the colour is
   times Storm3D +0x4, the alpha the device's +0x48. Corners in the order stock gives
   their texture coordinates (u0 v0, u1 v0, u1 v1, u0 v1). The one change: texels near
   white are lifted past it (`Hot=`, `HotFrom=`), so beam cores and torpedo hearts
   bloom by their energy;
3. **the engine's own `Submit`**, called through the vtable, for everything else,
   with its draw of a model ours (below, "The models"): mines, colony pods, the
   commando team, the myotron, the black hole, flux wave, stasis and transwarp
   weapons. The chain pulsar's `Submit` (0x574f20) draws its class's sprite at its
   position (as a torpedo's, in the sprite's own colour) and renders its emitter node
   (+0xdc); while the particles module runs, which fires that emitter itself, the
   sprite is drawn by `sprite.rs` and the `Submit` not called.

The mining, orbital and repair beams (`MiningBeam`, ODF class `minebm`: stock stretches
`wmbeam.sod` and its kin from the ship to its target, `MiningBeam::Submit` 0x57d1e0) are
recipes (case 1): beams flowing back to the ship in stock's colours (dilithium blue,
latinum green, the Borg metal and orbital beams violet, the repair beam orange,
flowing into the hull). A beam ends inside the moon it mines (+0xc8), where a flare
and a light show nothing: `ImpactAtSurface=k` puts its impact flare and its light where
the beam enters the target's bounding sphere (+0x60's object, its instance's radius
times k), so the light falls on the moon's face. The world module's moons take it as
any point light.

**The tractor, holding and assimilation beams** (`tracbm`, `tracbmo`, `holdbm`,
`techassimbeam`, `borgbor`; since 0.6.0) are `Kind=cone` recipes, after
`ART-DIRECTION.md`'s direction: not a laser but a cone of light through vapour. The
cone runs from the beam's start (+0xbc), `Width` wide, to `Spread` of the target's
half width across the beam (+0x60's object; without one, an eighth of the beam's
length) about its end (+0xc8), and on for `Beyond` of its bounding radius as it fades,
so the target sits inside the beam rather than at its tip. The half width is the
target as the hulls module posed it this view: its shown meshes' boxes (`Part::lo`,
`hi`) measured along the direction the cone opens on screen. The bounding sphere made
a cone as wide as a Sovereign is long when the beam met it nose on (seen on the
bench). It is 16 camera-facing segments, each node with its own side vector so the
segments meet; each is a trapezoid, so the across coordinate goes to the shader as
(v w, w) and is divided per pixel (interpolated straight, it bent at every quad's
diagonal and the streaks zigzagged). A flat quad through the target's
centre would be cut in half by the hull in the depth test, so the wide end is moved
towards the eye along the line to it (`Pull` target radii, growing as the square of
the way along): that leaves it where it was on screen and puts it in front of the hull
it holds, while the narrow end stays behind the emitter's own plating. The shader
(`cone` in `ordnance.hlsl`) has no core. Its light is up to four fans (`Fans=count,
interior,rate`): wedges from the emitter, each a band of the cone's own -1..1 (so it
spreads with it), hard edged (`EdgePixels`, by `fwidth`), brightest along its edges
and `interior` of that between them, with a fine grain of lengthwise lines. Each
fades in, holds and fades out `rate` times a second (each at its own rate) and takes a
new angle and width from a hash of its cycle, which changes only while it is dark, so
nothing jumps: the series' beams, which come and go while the beam stays on. The light
gathers towards the emitter, where the fans converge. Then three octaves of value noise for vapour, running along the beam (`Vapour`'s speed;
the tractors' run back towards the ship) and slowly across; brightest at the emitter,
`Fall` of it at the target; black within `EdgePixels` of the outline. A small
flare (`Emitter`) marks the point on the ship; there is no impact flare. **The wash**
on the target is a soft point light (`Light` times the beam's colour), placed
`WashBack` target radii from its centre back towards the emitter and full out to
`WashReach` radii: the hull facing the emitter, where the cone falls, takes it, and
the far side, turned away, does not. A spot light limited to the cone's footprint
would match better, but no lit draw takes one yet. Seen head on, the cone's quads
close to a line (as every ribbon here does). Not yet seen in game.

**The models.** Every model-drawing class's `Submit` ends by calling `Entity::Render`
(0x4cfda0) through slot 11 of its vtable: the instance's frustum test (instance slot 3),
the on-screen byte (+0x25), the instance's draw (instance slot 5). Before that, some set
the instance's scale (`Entity::EnableScaling` 0x4cff50 writes instance +0x74,
`SetScaleFactor` 0x4cff20 instance +0x28): the mining beam along its length
(class +0x164 across, the length over class +0x160 along), the stasis field to its
target's size (class +0x148 over instance +0x40), the black hole and flux wave by their
growth (class +0x15c times +0xbc, drawn only from 0.01); the transwarp drive draws only
while class +0x154 is set. So the module does not redo any of that: for a class it hands
to the engine, slot 11 of the class's vtable is pointed at `models.rs`'s `ord_render`
(only if it holds `Entity::Render`; checked and logged), and the engine's `Submit` runs
as it always did. While the ordnance step has a shot in that `Submit`, `ord_render`
draws it: the frustum test called as stock calls it, +0x25 set as stock sets it, then the
SOD named by the ODF's `shotGeometry` (whose database must be the instance's plain
`ST3D_Database`) at `Entity::GetTransform` times the instance's scale, posed on the
instance's clock, through the scene layer (lit by the scene's lights; the `team*` groups, a
mine's whole hull, in the owner's team colour (+0x5c): stock tints them with whatever team
colour `GameObject::RenderInternal` last handed the engine (Storm3D +0x34), the last
object drawn as far as the bench shows (a red player's mine showed green); additive meshes
such as the rift's and the transwarp's queued for the flush). Its emitters (the commando
team's and the pods' rockets, the myotron's contrails, the rift's clouds, the flux
wave's plasma) go to the particles module (`particles::feed`, keyed by the shot and its
class) and are drawn with every other particle; without the particles module a model
with emitters is the engine's. A recipe bolt's SOD emitters (the micro-organisms'
contrails) are fed the same way. Anything else (no SOD, another database) falls through
to `Entity::Render`. On top, `[Model.<odf>]` in `ordnance.ini` gives a model a glow at
its heart, an HDR flare pulsing on the sim clock with its own light on the hulls about
it: the antimatter mine's white-gold, the gravity mine's violet, the myotron's green.

**The warp flash.** `RenderWarpSpriteEffects` still runs: `WarpSpriteEffect::Render`
moves each flash onto its craft's line of motion and removes it, which the game
relies on. Its one draw, the call to `DrawScaled3D` at 0x59cc6b, is re-pointed at us
(only if it still calls 0x63aa60): the effect (its handle +0, its point +4, its
distance +0x10) is noted, and the flash drawn after the step as `Specials.asi` drew
it (`[Warp]`), with its afterglow and ring once the engine has dropped the effect.

**The special weapons' waves** are `ShockwaveExplosion`s, which the effects module
draws: it asks `mods::ordnance::draw_wave(expl, view)` first. A wave whose class
(+0x30) names, through its `cPrjID` (+0x24), an ODF with a `[Wave.]` recipe is drawn
there (`ShockwaveExplosion::GetCurrentRadius` 0x467730, its end +0x50, the class's
`fadeFactor` +0xe0, in the plane normal to its transform's second axis) and the call
answers true; any other is left to the caller's ring.

**Light.** At the start of each view, after `Ordnance::PreRenderAll` has registered
every torpedo's and pulse's light: their records in the registry (Storm3D +0x60:
record +0 the light, +4 its colour) take the hue of their sprite's first frame,
scaled to the ODF colour's peak (the class's light +0x10, its colour +0xf4), which is
`Lighting.asi`'s recolouring; every visible phaser (vtable 0x6b8ee4, not expired
+0x27, visible +0x24) gets an emitter and an impact light in its texture's colour
times its tint, up over 0.06 s and down over its last 0.25 s (time left +0xac of the
class's `lifeSpan` +0x1c); every torpedo and pulse lights its own ship where it
leaves it (below); and the special weapons' and the warp flashes' lights of the
last frame are registered. New lights are `ST3D_Point_Light`s made once with the
engine's constructor (0x62f240) and registered with `RegisterLight` (0x62d250), so a
hull the engine draws takes them too; the scene's list is then read again. With
`Lighting.asi` loaded, it does the recolouring and the phaser lights itself.

**The muzzle light.** The engine's torpedo and pulse light rides with the shot and
reaches its ODF's `lightFalloffStart` plus `lightFalloffRange` (50 + 5 on every pulse in
the game): at 1000 units a second it is clear of the hull in a frame or two, so a
firing ship took no light from its own discharge, while a phaser has its emitter light.
A shot that is in the ordnance list and was not in the last main view's (a new one; its
owner +0x5c a live game object) leaves a light where it is first seen, at its hardpoint
or a little out from it (bench 2026-10-17: the mothership's pulses 60 to 98 from its
centre, its bounding radius 106), held at the same place on its owner as the owner
moves. It is in the shot's own light colour (recoloured as above), full to a tenth of
`MuzzleReach` times the owner's bounding radius and gone at that reach, falloff squared
(hard, distinct, wrap 0.4), and fades linearly over `MuzzleTime` seconds of the sim
clock. Every torpedo and pulse weapon has it, on every ship. On the bench, at the
defaults the mothership's volley washes its bow lime green for about half a second,
clear from 500 away; at 3, 0.35 s and half the radius (the first guess) it was not seen.

**Settings.** The recipes and `[Warp]` are `render3d/content/ordnance/ordnance.ini`, built into the
plugin; a section or key of the same name in `Render3D.ini` overrides it. The
module's own keys, in `Render3D.ini` `[Ordnance]`:

| Key | Default | |
|---|---|---|
| `Stock` | 1 | 0: stock's sprites are drawn by the engine's `Submit` |
| `Specials` | 1 | 0: no recipes for shots; they fall to stock |
| `Waves` | 1 | 0: `draw_wave` draws none; the special weapons' waves are drawn as any shockwave |
| `Hot`, `HotFrom` | 0.6, 0.7 | how far past white a sprite's hottest texels go, from which level |
| `Recolour` | 1 | torpedo and pulse lights in their sprite's colour |
| `PhaserLights` | 1 | the phasers' emitter and impact lights (`PhaserBrightness`, `PhaserStart`, `PhaserRange`, `PhaserLift`, `PhaserImpact`, `PhaserImpactStart`, `PhaserImpactRange`, `PhaserImpactLift`: `Lighting.ini`'s) |
| `MuzzleLights` | 1 | a light on the firing ship where each torpedo or pulse leaves it (`MuzzleBrightness` 4, `MuzzleTime` 0.5 s, `MuzzleReach` 1 times the ship's bounding radius) |
| `Models` | 1 | 0: the shots that draw a model are drawn by the engine (no vtable slot is taken) |
| `Survey` | 0 | log every ordnance ODF the first time it shows, with its vtable and `Submit` |

### specials

**Layout (phase 7).** `hooks.rs` holds the reads of `mAllEffects` and its effects, and
the engine's own `Render(cam)` for the classes it leaves to the engine; the module has
no hook to install (it takes `Step::Specials`, and the frame hands the step back to the
engine while it is off). `content.rs` holds the three kinds' names and their shipped
looks. `configure` reads `[Specials]` at load and at every settings change (`Reload=1`
reads it again every 120 frames, as before). `Modules.specials` is live, checked by
`render3d-live-hulls` (pixels) and `render3d-live-specials` (off at load, on live).

`mods/specials/src/`, `mods/specials/shaders/specials.hlsl`, on by default (`[Modules] specials=0`
leaves it to the engine). `Step::Specials`: the special-object effects, which stock
draws in `SpecialObjectEffect::RenderAll` (0x60be90, called at 0x598372).

**What stock draws there.** `RenderAll` walks `SpecialObjectEffect::mAllEffects` (its
head node pointer at 0x7a8c0c; each node's +8 is an effect) and calls each effect's
`Render(cam)` (vtable slot 0). The base `SpecialObjectEffect::Render` (0x49ea70) draws
nothing (`ConvertEffect`, `Disable`, `SlowEffect`, `VampireEffect`...).
`TimedSpecialObjectEffect::Render` (0x60c460, every `Timed*` class, `BoardingParty`,
`LineOfSightEffect`) draws the `ST3D_Instance` embedded at +0x2c only when the weapon
gave it a database (+0xac, the instance's +0x80) and a visual target (+0xb0): the
instance is put at the target's transform (`Entity::GetTransform`) and rendered.
`TimedChangeTeam::Render` (0x60e090) also wants a node (+0xc8) and a target that is a
craft (flags bit 8) and not expired (+0x27), and puts it at that node of the target
(`Entity::GetWorldTransform`, 0x4cff90). Three weapons give an effect a database, each
from its ordnance class: `ComputerOverride::Hit` (a `TimedChangeTeam`, class +0x190),
`MicroOrganism::Hit` (a `TimedDrain`, class +0x188) and `PsionicDisruption::Simulate`
(a `TimedCrewDisruption`, class +0x160): the ordnance ODF's `particleArt`, a SOD that
is a single cloud emitter (`wovremit`: `override`, `wmicremit`: `microorg`,
`psionicdisruption`: `psiondis`; Cardassian `microorgs` names `wmicremi2`, which no
SOD is). Everything else in the list draws nothing in this step: the boarding parties
(their transporter shimmer is the billboard effect, the particles module's), the
hologram (`TimedHoloEffect` keeps its own instance at +0xb4, drawn from
`GameObject::RenderInternal`) and the rest of the timed effects.

**What the module does.** Once a frame (in `lights()`, after the particles module has
simulated) it walks the same list. For each effect stock would draw it finds the SOD:
the effect's weapon class (+0x4) holds its ordnance class at +0x18
(`OrdnanceClass::Find` of the weapon's `ordName`, in `WeaponClass`'s constructor), whose
cPrjID (+0x150, `cPrjID::GetOdfName` 0x6593a0) names the ordnance ODF; and hands the
instance's emitters to the particles module at stock's placement (`particles::feed`),
so the cloud is our particles, drawn at `Step::Translucent` in stock's look. The step
then draws only what it cannot take: an effect of a class whose `Render` is not one of
the three above, or whose SOD does not load, is rendered by the engine (its own
`Render`), as stock. Without the particles module running the whole step is the
engine's.

**The looks** (`look.rs`, `specials.hlsl`), new, on top of stock's cloud, for as long as
the effect is listed (effect +0x28: seconds left; +0xc: finished): a layer of light
over the hull of the object the effect is on (+0x8, if it is still in the object list,
not expired and seen by the player; posed by the hulls module, `hulls::posed`; a
cloaked hull gets none), additive, depth-tested, pushed out a little along the normal,
in the object's own frame so it stays on the hull; and a point light at the cloud. Up
over `Rise` seconds, down over the kind's last `Fade` seconds; everything above 1 blooms.

- **Computer Override** (`TimedChangeTeam`): green Borg circuitry (traces laid out per
  face of the object's box, nodes, pulses running along them) creeping over the ship
  from the side the Borg ship is on, with a bright front. Stock's cloud of coloured stars
  sits on the Borg ship while the beam flies and on the victim after the hit.
- **Energy Dissipator** (`TimedDrain`): a pink film of veins and breathing blobs, in the
  spores' colour.
- **Psionic Disruption** (`TimedCrewDisruption`): violet interference ripples. The
  effect is renewed every half second while the beam holds, so it fades over its last
  quarter second only.

| Key (`[Specials]`) | Default | |
|---|---|---|
| `Enable` | 1 | 0: the engine draws the step (read again every 2 s with `Reload=1`, for comparing at the same moment) |
| `Overlay`, `Lights` | 1 | 0: no layer over the hull / no light |
| `Survey` | 1 | logs each effect class and name the first time it is seen |
| `Rise` | 0.4 | seconds the looks take to come up |
| `Push` | 0.004 | how far the layer sits off the hull, in radii |
| `RimPower`, `CoreWhite` | 2.5, 0.35 | the layer's rim; how much its hot cores whiten |
| `<Kind>` | 1 | `Override`, `Drain`, `Psionic`: 0 turns that kind's look off |
| `<Kind>Colour`, `<Kind>Gain`, `<Kind>Strength` | | the layer's colour (0..255), times the gain (light), and strength |
| `<Kind>Creep`, `<Kind>Scale`, `<Kind>Rim`, `<Kind>Fade` | | seconds to cover the hull, pattern cells per radius, rim, fade-out seconds |
| `<Kind>LightColour`, `<Kind>LightBrightness`, `<Kind>LightRange` | | the light at the cloud (range in the ship's radii) |
| `LogEvery` | 600 | frames between log lines |

Seen on the bench (2026-10-15, `specials` scene, 1280x720), against stock at the same
moment (`Enable=0` with `Reload=1`): the clouds sit where the engine puts them; the
layers come up on the Sovereign as described, main view and with the HUD's inset.
Scenario `render3d-specialfx`.

#### effects: Hooks, Content, Settings

**Hooks** (`effects/hooks.rs`; each part installs its own when its `Enable` is on and no plugin of its own is loaded, all through `r3d_core::hooks`, behind the module's gate, chained onto what is there):

| Symbol | Kind | What it is for |
|---|---|---|
| `Craft::Explode`'s call of `ST3D_Instance::Fragment` (0x4c6bfc) | call | a dying craft whose SOD we read is noted for our own chunks; the engine makes none |
| the Evolver's four `Fragment` calls (0x4b07b4, 0x4b0872, 0x4b0914, 0x4b09b4) | call, one hook each | the cocoon bursting, whole; each site calls what was at that site |
| `BuildingEffect::SafeDestroyConstruction`'s `Fragment` call (0x46a0f1) | call | a cancelled construction's faces built so far |
| `Craft::RepairSystem`'s `AddParticle` call (0x4c90c0) | call | the new tail of the particle list is a weld |
| `NodeParticleEffect::Render`'s `DrawScaled3D` call (0x473767) | call | the particle's node (in `esi`) is noted; a weld, or every particle while the sparks are ours, is not drawn by the engine |
| the four `CreateShieldHit` calls (0x58bb95, 0x58cad8, 0x58d502, 0x58dd65) | call, one hook each | which weapon made a hit, and its damage: a passive tap (it records and calls on, so it has no gate; a hit made while off keeps its damage if the module comes on) |
| the call FleetOpsHook writes at `ShieldEffect::CreateShieldHit`+0xea (0x47449a), on a build that has it (Roots; `render3d.effects.shield_models`) | call (chained) | the model the companion's pick gave each new hit is noted against the shield type it was asked for (a passive tap, no gate) |
| `ST3D_Instance::Fragment` itself (0x62eb20), on a build whose profile says so (`ef_debris_at_callee`, Fleet Operations: Roots) in place of the three `Fragment` calls above (`render3d.effects.debris_callee`) | entry: its first five bytes (whole instructions, `ef_fragment_moved`, within the entry's check) moved to a trampoline | a dying craft noted as above, only while the caller is the build's companion module (FleetOpsHook, which owns `Craft::Explode`'s call and breaks a craft up only when its class allows); every other caller (a cancelled construction, the Evolver's cocoon, render3d) gets the function as it was |

The hooks are put in at the module's first frame (`device()`), not at load: other plugins check the stock bytes of these sites when they load, and the bench's `Scene.asi` reads the shield sites. With the gate shut, or a part off, every detour hands the call to what was there, so the engine breaks craft up and draws welds and particles itself. The explosion, shield, bee and construction draws need no hook (`feature()` and `object()`); the registry hands them back.

**Content** (`effects/content.rs`): the weapons' names for the shield taps and the faction keys, colours and ODF initials of `[Shields]`.

**Callee hooks** (PLAN-FLEETOPS.md, "The rules of (c)", rule 4). On Roots, FleetOpsHook
re-points `Craft::Explode`'s call of `Fragment` at a callback of its own, which calls
`Fragment` through a pointer of its own when the class's `explosionChunks` allows; writing
that call site would take Roots' decision away, so the module takes the callee instead
(`Hooks::install_entry`): `Fragment`'s entry jumps to a naked stub that keeps the return
address and goes on to the detour, which notes the craft only when that address lies in
the build's companion module (the profile's `[[module]]`, found loaded) and otherwise runs
the moved instructions and the rest of the function. FleetOpsHook calls `Fragment` from
nowhere else, so "called from the companion" is "during its explode callback". The entry is
checked as the profile's check says before the bytes move, and the hook stands down if
another plugin has written there.

**Shields on Roots**. Stock's `CreateShieldHit` gives a new hit the model of its shield
type from `m_logical_db` (`ST3D_Instance::SetLogicalDatabase`, then the visible one);
FleetOpsHook writes over that pick (+0xe0, the call at +0xea) a call of its own with the
type and the struck object as arguments and the hit in `ecx`. It picks per ship class from
the class's ODF: `shieldHit` (`shieldHitCritical` past a threshold of 0.2 on a value it
reads of the object, presumably the shields' level), `shieldDown`, `ShieldAdd`, `ShieldSub` and `ShieldSpecial1` to `4` for types 0 to
7 (Roots' ODFs name sized sets, `shield_fed_hitL`, `shield_bor_down` and the like), each
database loaded once by name through `FindLogicalDatabase` and kept for the class, and falls
back to the database's model where the ODF names none. So in a Roots fight no hit carries
the database's models (the bench's `first live hit` line showed a model that was neither),
and the shields, which knew a hit by its model, drew none of them: Roots' own
`DrawShield` drew them all through `feature()`. The module chains that call
(`render3d.effects.shield_models`, `site_hit_model`, Roots only) and notes, after it
returns, the model it set against the type it was given: a hit is then ordinary, a
shields-down ring or a shell by its type, whichever model it carries. Nothing of
FleetOpsHook's memory is read: the type is the call's argument and the model the hit's.
The shell's size is not the model's: `ShieldHit::ShieldUpdate` divides the hit's frame by
the model's radius and scales it by the class's shield transform, so the engine's shell is
the class's ellipsoid whatever SOD the class names, and the module's ellipsoid (the same
transform) stays the one to scale by `SHIELD_EFFECT_RESCALE` (below).

**Shields and `RTS_CFG.h`**. Roots' `ShieldHit::DrawShield` (its file and FleetOpsHook)
scales a hit's shell by 2 while `RTS_CFG.h` sets `SHIELD_EFFECT_RESCALE = 1`, on top of the
instance's scale (which the stock body applies only while the instance is scaled), and
colours a hit by the shields' level only while `SHIELD_EFFECT_DAMAGE_COLORED = 1`, white
otherwise. On a build whose profile says it follows these keys (`ef_shield_rescale_pct`,
the factor in percent, and `ef_shield_damage_key`; 0 on stock, which reads neither), the
module reads them from `RTS_CFG.h` (`r3d_core::rtscfg`, through the file system's `data\`:
the active mod's copy over its parents' over the game directory's, the one the game
reads; FO-5 step 6) and scales its
shield ellipsoid about its centre (the repair sparks' outward direction keeps the unscaled
one) and, with the damage colour off, draws the stock colour mode white and keeps the
faction colours from shifting with the shields' level. A hit the module does not draw is
handed to the engine's `DrawShield` whole, Roots' version on Roots.

**Settings**: `Modules.effects` (live) is the only row. A flip off drops the debris in flight; the shield hits, welds and construction edges are keyed to the engine's own records and stay, so a flip on picks them up where they are. Effects already in flight finish in the look they started with.

### sky

Takes over `Step::Background`, the whole of `Background_Render` (0x590d90), and
supersedes `Sky.asi` (it stands down while that plugin is loaded). In stock's order:

1. **The far plane.** `ST3D_Camera::SetFarPlane` (0x6198a0) to `STAR_FAR_CLIP_PLANE`
   (0x6fcac8, 80000) for the step and back to `FAR_CLIPPING_PLANE` (0x6fcab0) after, as
   stock does. That changes only the camera's CPU projection, so our draws use the
   device's projection with its far plane moved out to the same distance: under the
   scene's far plane the starfield (10000-20000 out) would be clipped.
2. **The sky**, when the view shows one (`Render_Stuff` +4, and the detail
   configuration `Storm3D+0x104` +0x2c not 2) and the map has a sky database
   (`g_background_geometry_db`, 0x7720fc). The map's background name is the lower-cased
   string `Starfield_Load_Background_Geometry` (0x590c30) keeps at 0x738538:
   `<name>.sod` or a bare prefix.
   - **With a recipe** (`render3d/content/sky/<name>.ini`, built into the plugin; a
     `Sky\<name>.ini` in the game directory takes precedence and is re-read when it
     changes), the procedural sky of `Sky.asi`, each recipe's shape and grade fitted
     to its stock sky, with soft billows and no filaments ("Sky recipes" below): one quad over the
     viewport carrying each corner's world ray (from the inverse of view x projection,
     far point minus near point, which is affine in screen position), sampling a cube
     baked once per recipe. The bake is on the GPU only: each face is drawn by the
     `bake_*_ps` chain (five passes) at twice its edge
     (Sky.asi's texel mapping, edge texels on the edges), then `down_ps` averages each 2x2 with one bilinear tap into the face of a
     half-float cube render target. The cube is in the default pool: dropped before a
     `Reset`, baked again after. The gas is saturated as before; the recipe's stars are
     added after that, so a star core may pass white (to 2) and bloom a little.
     `fine_z`'s noise volume is `Sky.asi`'s, made the same way (managed pool).
   - **Without one** (`Mbgstars`, or `Recipes=0`), the stock faces, unlit, at the
     camera, depth off. A cube SOD's meshes through `scene::model`, culled as the
     engine culls them (`mbg02.sod` holds two cubes, and only the one wound inwards
     shows). A prefix sky as `CreateBackgroundFace` (0x590f10) builds it: face `i`
     shows `<prefix><i>` on a quad at +-100 about the camera, texture coordinates
     (0,0) (1,0) (1,1) (0,1) at its corners, clamped (`ClampTexturesRecursive`). A face
     whose texture is missing draws white, as the engine's does.
   - Otherwise a black clear of the view: stock's only colour clear.
3. **The starfield** (`Render_Stuff` +8; off in the HUD's action camera), read every
   frame from the engine's own: `g_starfield` (0x773d04) +4 is the root
   `OctTree_Node`, whose objects (+0x30) and children (+0x3c) are VC6 `std::list`s of
   `auto_ptr`s (the pointer at list node +0xc). A star (`Starfield::mCreateStarSprite`
   builds them once, at random from `g_asyncRandom`, so every session differs) holds
   its `ST3D_Sprite*` at +0 and its quad's four corners in world space at +0xc, which
   `OctTree_Node::Render` hands to `ST3D_Sprite::DrawQuad` unchanged. The sprite's name
   (database element +0x8) finds its texture and rectangle in the sprite table; its
   colour is at +0x24. Added whole (the texture's alpha plays no part), as stock's
   additive material. The point stars (`mstars`) are lifted past white near their
   cores (`StarLift`); the blobs and clusters are not.
4. **The background objects**: every object with flag `0x10000` (the
   `BackgroundObject` constructor sets it; the object loop skips them), unless its
   class is `hidden` (+0x19c; `dlight`) outside edit mode (`UserProfile` 0x76b5ac,
   +0x84). Stock ones are `background_obj`s (`BackgroundObjectClass`: the suns
   `Msun1-3`, `mwreck`, `mnebula`, `mgate`), drawn by `GameObject::Render`. Ours: the
   SOD's meshes now, lit, all of them (nothing may wait for the flush: the depth is
   cleared before the scene), and its sprite nodes as billboards. A sun is one
   sprite node (`sun1..3` on `Mmoon`, half extent 200): `ST3D_SpriteNode::RenderInternal`
   (0x63c300) colours it by the node data's colour times `s_diffuseColour` (white
   outside the nebulae) and sizes it by `m_billboard_render_scaling_factor` (1). The
   brightest texels go past white (`SunLift`). An object whose class overrides
   `Render`, or whose SOD does not load (`mgate.sod` is version 1.2), is the engine's.
5. The device's `Flush` (anything the engine queued), the far plane back, and
   `ClearDepthBuffer` over the view for the scene.

`Render3D.ini`, `[Sky]` (read at load and at every settings change):

| Key | Default | |
|---|---|---|
| `Draw` | 1 | 0: the engine's `Background_Render` (for comparing on the bench) |
| `Recipes` | 1 | 0: the stock faces where a recipe exists |
| `Face` | 1536 | the baked cube's edge; 0 computes the sky at every pixel (the reference) |
| `Reload` | 1 | re-read `Sky\<name>.ini` in the game directory every 30 views when it changes |
| `Dump` | 0 | 1: each bake writes its faces to `Sky\<name>.dump0..5.tga` for `perf/sky-grade.py` |
| `StarLift`, `SunLift` | 0.5, 1 | how far past white the brightest texels of the point stars and of the suns go (0: stock's light) |
| `Load` | | bench only: a background name to load in place of the map's (`Starfield_Load_Background_Geometry`), once per new value. With `Recipes=0`, a cube SOD loaded after another draws black (seen 2026-10-16 for `mbgbaku`, `mbgborg`, `mbgdom1` and `mbgkling` after `mbg02`); for a stock reference across several skies use `Draw=0`, the engine's own draw, which shows them all |

Seen on the bench (2026-10-15, `a2_borg01`, 1920x1080), each against the engine's own
draw in the same session and frame (`Draw=0`, paused): the stock faces of `mbgaqu`,
`mbgkling.sod`, `mbgpur`, `mbgbaku.sod`, `mbgborg.sod` and `mbgstars` match (sky regions
equal to 0.1/255; `mbgborg` means equal to 0.05), the starfield matches with
`StarLift=0`, two spawned suns match to 0.2/255 with `SunLift=0`. `mbg02.sod` (and
`mbgblue.sod`, the same model) is 2-5% brighter than the engine's: its material is lit
(model 1), and the engine lights the sky with the scene lights; ours is unlit. The
recipe skies match `Sky.asi`'s away from the (random) starfield blobs. The HUD's action
camera draws the sky without stars, as stock.

**Sky recipes** (`render3d/content/sky/<name>.ini`, section `[Sky]`, one per stock sky name;
built into the plugin with `include_str!`, so the sky needs no file in the game
directory; a `Sky\<name>.ini` there takes precedence). Colours are 0-255; directions
are yaw (degrees about +y; 0 looks along +z, 90 along +x) and pitch (up from the
horizon). Each recipe's header says which maps use it and the stock channel means it
aims at.

| Key | Default | |
|---|---|---|
| `Seed` | 1 | moves the noise domain: a different sky with the same character |
| `Scale` | 2 | the largest structure's frequency over the sphere; 2-3 gives a few billows per cube face |
| `Warp` | 0.4 | domain warp; over ~0.5 it drags streaks |
| `Detail` | 0.5 | amplitude ratio between octaves: lower is softer |
| `Octaves` | 6 | how many of the gas's six octaves count, 1-6; a fraction fades the finest out. Under 4 the value noise's lattice shows (blocky, axis-aligned clumps) |
| `Coverage` | 1.0 | where the gas begins, in units of the field's spread: higher is emptier |
| `Softness` | 2.0 | the width of the gas's edge in the same units; 2-3 is cloud (a wider one restores depth where dense skies saturate into plateaus) |
| `Patchiness` | 0.5 | a one-octave field at a third of `Scale` that gathers the gas in parts of the sky |
| `Ridge` | 0 | 0-1: towards soft bands |
| `Gamma` | 1 | on the density: above 1 thins the faint gas |
| `HueScale`, `HueMix` | 2, 0.5 | how sharply the two gas hues separate; 0: hue by region, 1: hue by density |
| `Brightness` | 1 | the whole sky |
| `Dither` | 1 | the per-pixel dither, in steps of the 8-bit back buffer |
| `Deep`, `GasA`, `GasB`, `Glow` | 0 | r, g, b |
| `Core1`, `Core2` | 0,0,10,0 | yaw, pitch, size (degrees), gather (field units); `Core1Colour`, `Core2Colour`: the core's own glow |
| `Band` | 0,90,20,0 | yaw, pitch of the band's pole (0,90: the horizon); half-width (degrees); gather (0: no band) |
| `Stretch` | 0,90,1 | yaw, pitch of the axis; how far structure is drawn out along it (1: none) |
| `Grain`, `Dust`, `Haze` | | detail: grain (amount, scale times `Scale`, gain between octaves), dust (amount, scale, coverage), haze (amount, coverage); amount 0 is off |
| `Stars` | 0,1,0.5,0 | how many over the sphere, brightness, size (degrees), in-gas weight; `StarA`, `StarB`: the two ends of the colours |
| `GradeGain`, `GradeCurve` | 1,1,1 | the grade per channel, `gain * in^curve`, after `Brightness` and before the stars |
| `Fine` | 0,0.3,0.55 | detail finer than the cube, at draw time: amount (0: off, the default: the skies keep to the broad gas, as stock), the largest octave's size (degrees), gain between octaves. The cube is magnified about 3.4x at 4K, so finer detail comes from here, not from a bigger cube |

Edit a recipe in the game directory's `Sky\` with the game running and it re-bakes
within half a second (`Reload=1`). **The grade** is fitted, not hand-tuned:
`perf/sky-grade.py` fits `GradeGain`/`GradeCurve` per channel by least squares through
the log percentiles 50..99.5 of the bake against the stock faces (the stock faces'
40-texel corner notches masked), then scales every gain together so the sky lands at
0.92 of stock's luminance, a little under and never over, every sky alike (Aqua was
left ungraded and `mbgrg` at 0.57 of stock until render3d 0.3.0, when the skies were
fitted to stock's shape). Refit after changing a palette or
`Brightness`; its docstring has the procedure (`Dump=1` writes the faces it reads).

**The shape** is fitted too, before the grade: `perf/sky-shape.py --fit` moves `Scale`,
`Detail`, `Octaves`, `Coverage`, `Softness` and `Warp` on a running bench game
(`SCENE=sky`) until the bake's structure matches the stock faces', in two measures the
grade cannot change: how the log luminance's variance spreads over octave bands (clump
size and edge hardness), and its standardised quantiles (mid-tones against plateaus on
black). Both are taken at 512 texels a face, with stock magnified bilinearly as the
engine shows it, so detail finer than a stock texel counts against a recipe as it would
on screen; measured at stock's own 256 the fit put that detail into crisp, swirling
edges. `Softness` may only widen and `Warp` only drop (soft, billowing gas, no streaks),
and every recipe is fitted, Aqua and `mbgrg` included. Most stock skies are many
mid-sized clumps, a few degrees across, and their fitted `Scale`s are two to five times
the hand-set ones (`mbgborg` 2.4 to 12.1); `mbgbaku`, `mbgdom1` and `mbgpur` are large
soft shapes and stayed near theirs. The grain, dust and draw-time detail the recipes had
before render3d 0.2.1 stood in for that structure as speckle; with the shape fitted
they stay off. The descent is greedy: started from the hand-set `Scale`, `mbg02`,
`mbggb` and `mbgred` stopped in a local minimum (0.3-0.6) and were fitted again from a
`Scale` of 7-9 (to 0.14-0.25). Read the bands before trusting a fit that stays far
from stock, and look at it: the measure is blind to a value-noise lattice (hence the
`Octaves` floor).


#### sky: Hooks, Content, Settings

**Hooks** (`sky/hooks.rs`): none installed. The sky takes `Step::Background` and the background objects from the seam, and hands an object it cannot draw back to the engine's `Render`; the file holds the reads (the background name, the sky database, `g_starfield` and its octree, the far planes, the detail byte that turns the sky off, edit mode, the background instance).

*On Fleet Operations: Roots* (FO-5 step 7, verified). The one Roots change in what the module
uses is FleetOpsHook's in `Starfield_Load_Background_Geometry`: nine bytes at +0x16, the
free of the previous background database after its destructor has run (it does the same in
`Background_Close`), so the old database's memory is kept rather than handed back. The entry
the module checks, the name the function keeps lower-cased (`STARFIELD_GEOMETRY_NAME`), the
database pointer it sets and `Background_Render` are stock's in the file and in memory, and
the module calls the function only on the bench (`[Sky] Load=`). `GameObject::Render` is
FleetOpsHook's trampoline at its entry (checked as nothing there, as for the core): the
module only compares a background object's `Render` slot with its address, which the
vtables still hold. Roots' skies are its own art, so no built-in recipe matches and the
module draws the map's own faces or cube (`sky: "env_dark_dust": no recipe, the stock
sky`), with the starfield and the background planets, on render3d's target.

**Content**: the built-in recipes (`render3d/content/sky/`, `sky/content.rs`; a `Sky\<name>.ini` in the game directory wins) and the point-star texture. **Settings**: `Modules.sky` (live); `[Sky]` is read in `configure`. `formats_changed` drops the view pipelines and keeps the baked cube, which does not depend on the scene target.

### overlays

`mods/overlays/src/`, `mods/overlays/shaders/overlays.hlsl`; on unless `[Modules] overlays=0`, and
standing down while `Fog.asi` is loaded (it supersedes it). It draws what the engine
draws over and about the map rather than as objects, and nothing it does is written
where the simulation reads. Settings in `Render3D.ini`, `[Overlays]`.

**`Step::Terrain`** (`Terrain_Geometry::Render`, 0x593200, on `g_terrain_geometry`
0x772140) is run as stock runs it. Until the engine's first call has made its meshes
(`Terrain_Geometry::m_initialized`, 0x772180) the step is the engine's, and so is the
whole step in the tactical view (`View_Record` 1) when the profile's shroud option
(`UserProfile+0x45`, 1 or 2) has the shroud drawn as a `TreeGrid` (`+0x28`), a path
not seen on the bench. In order:

1. the build footprints, the engine's own sprites
   (`PlaceholderFeature::RenderFootprints` 0x473930 on `placeholderFeature` 0x736d50,
   at the grid's height `+0x1c`);
2. the tactical grid, when `GridVisible()` (0x51e180, the current `View_Record`'s
   `GridRenderState`) says so: one line every `GRID_SIZE` (0x71a78c) across the map's
   rectangle (`+0x4` x0, `+0xc` z0, `+0x10` x1, `+0x18` z1, `+0x1c` y; `+0x20` + 1
   lines along z and `+0x24` + 1 along x, summed a step at a time as stock sums them),
   in `ST3D_Colour_Dark_Orange` (0x7a8ee8), depth-tested and written as stock's
   `ClipAndRenderLine` draws them. `Grid=0` calls the engine's `DrawGrid` (0x593480);
3. the fog of war, unless the map editor is on (`UserProfile+0x84`), only in the RTS
   view (`View_Record` 0) and when the game has fog (`GameSetup::isFog` 0x5462f0);
4. the shroud over it, under the same conditions with `GameSetup::isShroud`
   (0x546300).

**The fog and the shroud** are `Fog.asi`'s look (the cubic mask, the
pixel-wide smoothstep edge, stock's rim, the taps held inside the grid, which keeps the
shroud from thinning along the map's border; `Filter`,
`FogThreshold`, `ShroudThreshold`, `FogSoftness`, `ShroudSoftness`, `FogRim`,
`ShroudRim`, `RimWidth`, `Antialias` keep its keys and defaults), drawn on a quad of
our own instead of through the engine's draw. The quad is the map's rectangle at the
grid's height, with the UVs `Terrain_Geometry::SetVertexPositions` (0x592e10) gives the
engine's mesh: the map centred in the texture's unit square with square texels, v
running against z. The textures are the engine's: `fogTexture` (`+0x34`) and
`shroudTexture` (`+0x38`), `ST3D_Texture`s whose per-device texture sits at `+0x40` +
the device index (`Storm3D+0xc0`) times 4, and whose `+4` there is the
`IDirect3DTexture8`, which under Display8's stub device is the mirror of ours (`gfx::engine_texture`; until 15.0.0 d3d8to9 answered with its `IDirect3DTexture9`). They are never
resized (one texel per scan cell; resizing them froze the game). The grid in them is
`Scanner::m_fog` / `m_shroud` (0x738a54 / 0x738a58: width `+0xc`, height `+0x10`),
centred. Stock's states: the depth test always passes and nothing is written; the fog
blends with an opacity of 0.7 (its texture material `fog` is made 0.3 translucent:
`ST3D_TextureMaterial+0x24` = 1 - 0.3, read into the device's material opacity), the
shroud is opaque and alpha-tested, here blended so its edge can be soft. On the bench
(`SCENE=fog`, 1920x1080, against `Fog.asi` at the same moment) both layers read the
same grey levels to the unit, inside and at the edge.

**`Step::Selection`** (`RenderSelectionEffects` 0x599900, main view only, not in a
cineractive): the selection bubbles, from `SelectionEffect::sm_EffectHash` (0x773c88;
16 buckets at 0x773c8c chained by `+0x9c`), read and never written. An entry is an
instance of `select.sod` with `+0x84` its reasons (1 the player's selection, 2 a
co-player's, 4 under the pointer), `+0x88` the object's handle (`Entity::Get`
0x4cfff0), `+0x8c` the damage pulse's phase (which `SimulateSelectionEffects` advances)
and `+0x90` its base scale (the shield ellipsoid, each semi-axis at least 15, times 1.2
over the sphere's radius), times `Entity::GetScaleFactor` when `IsScaled`. The colours
are `SelectionEffect::Render`'s (0x599e80): relation to the player's team on a craft,
(0.7, 0, 1) enemy, (0, 0.7, 1) neutral, (0.6, 0.6, 0.6) ally
(`GameObject::GetTeamRelation` 0x4d1080 against `Transport::getUserTeam` 0x5578b0),
times `0.25 + 0.2 (1 - health) sin(phase)`; on anything else the team colour times
0.25; a second shell at 1.1 times the scale in the team colour (full under the pointer,
else times 0.25), all times `UserProfile+0x48`; a co-player's selection blinks on wall
time. While another plugin claims the bubbles (`CLAIM_SELECTION` through Display8's
`display8v1_claims`: UI.asi's `[HUD] SelectionBubbles=0`, whose brackets mark the
selection; "Claims"), none is drawn. Only on a `d3d8.dll` without claims (not Display8
of contract 1.1) does render3d read the UI's sign in the engine instead: the `ret` it
puts in `RenderSelectionEffects`'s first byte, which keeps the engine's own from
drawing either way. The material `selection` is constant: the colour is added as it is, depth-tested,
not written, front faces only (on the bench stock's bubble adds one layer of its
colour, not two). Seen on the bench: own ships grey with the team shell, an enemy
Klingon violet with a red shell, the hover shell on a cube in its team colour, each
matching stock. Stock stretches a warping craft's bubble with its hull
(`WarpInstance`); not done here yet.

**Ghosts** (the feature `Ghost`, vtable 0x6aff8c; `Ghost::Render` 0x472cc0):
`Ghost::ghostList` (its node pointer at 0x736c94) holds a spirit per object the player
has seen and no longer sees: `+4` its class (`GameObjectClass::GetOdfName` 0x4ce370
names the SOD, as for a hull), `+0xc` its team (`Team::GetTeam` 0x496340, colour at
`+8`), `+0x10` a copy of its instance (transform `+0x44`, position `+0x68`, scale
`+0x28`, visible database `+0x80`). Stock draws each (not in a cineractive or the
editor) with the lighting material `Ghost+0x24` in place of every mesh's own
(`light.odf`: ambient and diffuse 0.1 at the material's `+0x18` and `+0x24`) and the
team colour times `ghostTeamColorFactor` (`Ghost+0x28`) on team faces, culled by
`GameObjectInstance::DistanceCull` (0x4d5bb0, which writes only the spirit's own
instance), opaque: on the bench a ghost is near black and hides what is behind it.
Here: texture times (the material's ambient + its diffuse times the directional
lights), the team faces tinted, self-illuminated texels lit as on a hull (the windows
show on stock's ghosts too). That holds for a mesh drawn opaque; a ghost's mesh keeps
its own texture material's blend in stock. An `additive` (or `wormhole`) mesh, the
black hole's disc (`mblkhole`, `Mblackh.SOD`) or a wormhole's swirl, is added, unlit,
at the `Storm3D+0x4` stock leaves at 0.5 for the pass: on the bench
(`SCENE=fog-blackhole`, `scenarios/fog-blackhole.md`) stock's fogged black hole adds
0.49 of what the seen one does, its swirl showing through the fog. So the ghosts are
drawn in two passes, the opaque meshes (an `alphathreshold` one cut out at one half),
then the additive ones without depth writes, the texture times 0.5. Drawn opaque and
lit like a hull, the black hole's ghost was a near-black disc over the fog (issue #3:
at its rim 2 and 19 against 49 and 54 of the fog about it; now 53 and 61, stock 53
and 56). A planet's spirit (its instance copy a `PlanetInstance`,
vtable 0x6b2ce4, over a `Planet_Database`) goes to the world module, which draws it
on its own sphere with the planet's ground and shell, culled as the spirit's `Render`
culls (`DistanceCull`), without the race's tint or cities (they live on the object):
left to its own `Render`, a fogged planet came back as the engine's low-polygon sphere
and stock texture. Stock dims it: on the bench (`scenarios/render3d-planet-ghost.md`)
stock's planet ghost is 0.20 of the seen planet's brightness, so every pass's colour
is scaled by `[World] PlanetGhost` (0.2; `shine.w` in `world_planet.hlsl`, 1 for a planet
seen), which measures 0.22 there. Any other spirit that does not show a plain SOD (an
asteroid field's, a nebula's) is left to its own `Render`, under stock's state
(`Storm3D+0xa8` the material, `+0x4` 0.5, `+0x34` the tint).

**The build ghost** (the feature `PlaceholderFeature`, vtable 0x6b005c;
`PlaceholderFeature::Render` 0x4738a0): taken here and handed to the hulls module
(`hulls::placeholder`, "The build ghost" under hulls), in the main view only as stock
draws it: the class `+0x34`, where it would stand `+0x38` (a `Matrix34`), its colour
`+0x68`, and `+0x74` whether the placement is clear (set: the footprints' `valid_build`
sprite; clear: `invalid_build`, and stock lights the ghost with the dark red ambient at
0x736d40 in place of the scene's). With the hulls module off or `[Hulls] BuildGhost=0`
the feature is the engine's.

**Height posts** (`posts.rs`, `vs_line`/`ps_line` in `mods/overlays/shaders/overlays.hlsl`).
`DrawHeightPost(const Vector3&, ST3D_Camera*)` (0x593840, cdecl) draws a line from a
point straight to the grid's plane (`Terrain_Geometry+0x1c`, 0x77215c), in
`ST3D_Colour_Green` (0x7a8f68) when the point is above it and `ST3D_Colour_Red`
(0x7a8f78) when not, through `ST3D_Camera::ClipAndRenderLine` (0x618590): a pixel wide,
aliased. Its five calls are re-pointed (each only while it still calls 0x593840):
`PlaceholderFeature::sRenderInternal` (0x473bbc; the hulls module's build ghost notes
its post directly), and from `DisplayInterface::PreRenderAll`, inside the body, the
overview's `mNormalModeRender` (0x522080, the object under the pointer), `mRenderWhere`
(0x5222fd, the move target), `mDrawPosts` (0x524c02) and its one-point helper
(0x524c2d). While one of our views runs a post is noted and drawn at the flush; outside
one, the engine's runs. Ours: a quad about the segment in screen space, `HeightPostWidth`
pixels wide at 1080 lines (scaled with the view's height), covered by the distance from
its centre line with a soft edge `HeightPostSoft` pixels wide, so it is as crisp at any
resolution; the segment is cut where it passes behind the eye; depth-tested, not
written, blended. At most 4096 posts a view.

| Key (`[Overlays]`) | Default | |
|---|---|---|
| `HeightPosts` | 1 | 0: the engine's `DrawHeightPost` |
| `HeightPostWidth`, `HeightPostSoft` | 1.5, 1 | the line's width (pixels at 1080 lines) and its soft edge (pixels) |
| `HeightPostBrightness` | 1 | the colour times this (above 1 it blooms) |

**The pivot marker** (`pivot.rs`; the feature `TacticalCamera`, vtable 0x6b46c0, a
game feature like the ghosts, drawn in `GameFeature_RenderAll`'s loop through slot 8,
`TacticalCamera::Render` 0x4e5260). In the tactical view, once the camera has been
panned off the units, it turns about a point of the map, and stock marks that point:
`Render` draws the camera's `+0xe0`, an `ST3D_Instance` (vtable 0x6aff14) of
`dummy.sod`, through the instance's `Render` (its vtable `+0x18`), only in the main
view (`Render`'s bool), with the instance made, `+0xe4` set (the camera follows no
entity) and `View_Record` 1. `TacticalCamera::Simulate` (0x4e4850) makes the instance
the first time it is wanted (`FindLogicalDatabase` of `dummy.sod`) and moves it every
tick to the camera's point of interest (`SetOrigin`, the instance's `+0x68`);
`dummy.sod` is a small flat ellipsoid (some 30 by 23 by 8 units) with one additive
material, which on the bench at 1280x720 reads as a rose-red lens about 26 by 9 pixels,
srgb(163, 91, 89). It is drawn into the 3D scene, so `UI.asi`'s HUD never covered it
(`ui/HUD.md`, "the last leftovers").

Ours takes the feature under the same conditions (nothing is drawn where stock draws
nothing) and draws at the instance's origin a thin ring on the map's plane,
`PivotRadius` world units across its radius, with four short ticks pointing in at the
point along the world's axes: the height posts' lines (above), so the same pixel width
at any resolution, a faint wide halo (`PivotGlow`) under the ring's bright core, in stock's rose
times `PivotBrightness` (the core blooms a little), depth-tested so a ship in front
hides it, not written. It is fixed in the world: it turns only as the camera turns
about it. It is taken only while slot 8 holds 0x4e5260 and that function's bytes are
stock's (to its `ret 8`), and the instance's vtable is `ST3D_Instance`'s; otherwise the
engine's `Render` runs (Render3D.log says which: "pivot marker: ours" or "is not
stock's"). `Pivot=0` leaves it to the engine, drawn on the stub by Display8's
passthrough (`render3d-pivot-stock`); `render3d-pivot` checks ours, with the engine
drawing nothing in the tactical view.

| Key (`[Overlays]`) | Default | |
|---|---|---|
| `Pivot` | 1 | 0: the engine's marker |
| `PivotRadius` | 18 | the ring's radius, world units (stock's lens is about 15 across its half) |
| `PivotWidth` | 1.5 | the core line's width, pixels at 1080 lines |
| `PivotGlow` | 0.35 | the halo's opacity (0: none) |
| `PivotColour`, `PivotBrightness` | 0.64 0.36 0.35, 1.6 | its colour (stock's rose) and the factor on it |

**Move paths** (`paths.rs`, `vs_dots`/`ps_dots` in `mods/overlays/shaders/overlays.hlsl`;
issue #17). Stock sketches a move path with the `enter_waypoint` binding held (Tab in
`Input.map`; `cOverViewImp::Init` 0x51e4c0 keeps its live state's pointer at
`cOverViewImp+0x30c`, the overview being the object `g_pOverView` (0x768e3c) points to,
stock's static one at 0x768e40): each right-click
(`mNormalModeRightClick` 0x5229a0) adds the map point under the cursor (`+0xa4`) to an
`AiPath` at `+0x264` through `mAddPathPoint` (0x5255b0), and nothing is sent. Let go,
`mProcessWaypoints` (0x522aa0, every frame from `mNormalModeInput`) sends the whole path
as one order (`GameObject::QueueCommand` 0x4d4380, `NetOrderObjectsWhatPath`) and frees
the sketch; with `[QOL] PathStartsAtOnce=1` it goes out at each point instead
(`qol/README.md`, QOL-10). An `AiPath` holds its point count at `+0x0`, its `Vector3`s at
`+0x4` and its type at `+0x8` (2: a patrol's loop). `GameObject::DeQueueCommand`
(0x4d4650) gives every ship of the order a copy of its own; on the next tick the craft
process (`GameObject+0x44`) takes it over in `Handle_New_Command`: the command at `+0x38`
(GO, 4, for every kind of move; PATROL, 26), the path at `+0x88` and the index of the point
it heads to at `+0x12c`, from 0. `Move_To_Formation` (0x436ad0) moves the index on as each
point is reached (a patrol wraps it round); at the last one the ship stops and `STOP`
releases the path (null). The ship stands at `GameObject+0xac` and its team is `+0xec`.
Stock draws the sketch only while the key is held, at the end of `mNormalModeRender`
(`mDrawPath` 0x525180, its call at 0x522247: a one-pixel green line with boxes at the
points); a path being flown is never drawn.

Ours, in `Step::Translucent` of the main view: for each **selected** ship of the player's
team (the overview's `GetSelectNum` / `GetSelectList`, vtable `+0x94` / `+0x98`, entity
ids turned into objects by `Entity::Get`; not the selection bubbles' effect hash, which
also holds co-players' and hovered objects, and not the array at `+0xbc`, which QOL's
`MaxSelection` moves, so a selection above 16 is read where it is) with a path, a row of
fine dots from the ship to the point it heads to and on through the rest, and a bigger dot
at each point. An unselected ship's path is not drawn. The dots are placed from the far
end of each segment, every `MovePathSpacing` pixels, and interpolated in screen space, so
they stay put on the map as the ship flies and are cut off behind it: the part flown is
gone, and a point passed loses its dot. Ships of one order share points: a segment or a
point already drawn this frame is not drawn again.

*The source of the path.* When a loaded plugin (`QOL.asi`) exports the queue it keeps (the
appended legs, per-leg warp), that queue is drawn: the exports are found by name among every
loaded module's (`rt::find_export`, never by the file name), once, and again once a second
until found (not every frame). The contract, cdecl:

    int QOL_MoveQueueVersion(void);   /* 1; any other value: not used */
    int QOL_MoveQueue(const void *ship, float *xyz, int *flags, int cap);
    int QOL_MoveQueueGroup(const void *ship, float *own, float *pts, int *flags, int cap,
                           int *queue);       /* optional */

`ship` is the `GameObject`; the call returns -1 when QOL keeps no queues, 0 when the ship
has none, else the number of points (at most `cap`, 128 here), from the one the ship flies
to now to the last, as xyz triples; `flags[i]` bit 0 is a warp leg (drawn as any other),
bit 1 a point of a patrol run that loops back to that run's first point (a run is
closed back to its first point). The points are the ship's own targets: QOL spreads a
group over its formation, so each ship of a queue flies to the queue's point plus its place
(`qol/README.md`, QOL-10, "A group's targets"). `QOL_MoveQueueGroup` answers the same and
also the queue's own points (where the group's leader goes, the formation's anchor) and
which queue. With it, a queue with one selected ship draws that ship's own path; a queue
with several selected ships draws **the group's path once**, through the queue's points
with their big dots, and for each ship only its own leg (from the ship to where it heads)
and a small dot (`TICK`, 0.55 of a point's) at each of its own targets, so six ships do
not draw six parallel lines. Without it, each ship's own path is drawn. The line runs from
the ship to point 0 and on. QOL adds
each Tab-click at once, so with its queue live (the last call answered 0 or more) **no
sketch is drawn**. Without the export, or when it answers -1, the engine's own path
(`+0x88` / `+0x12c`, only paths that came from a sketch: an ordinary move is a path of
one point and is not drawn) and the sketch are drawn as before, while the key is held and
for up to a second after, until a ship's copy ending at the same point has arrived;
the filter and the sketch are only the fallback's. Stock's call of `mDrawPath` is
re-pointed and draws nothing while ours are drawn. Points are lifted a unit above where
they were clicked, clear of the grid's lines; depth-tested (a hull in front hides them),
not written, blended.

Render3D.log (the first 200 lines): once `overlays: move paths: QOL's queue found
(version 1)` (`(version 1, groups)` with `QOL_MoveQueueGroup`), or `QOL's queue not found (the engine's paths, and the sketch)` (then looked
for once a second without more lines) or `QOL's queue is version N: not used`; and, when
one of the counts or the source changes, `overlays: move paths: N selected ship(s) on a
path, M point(s) ahead (qol|engine|-), sketch K`. `render3d-move-paths` checks it with the
QOL change.

| Key (`[Overlays]`) | Default | |
|---|---|---|
| `MovePaths` | 1 | 0: no paths drawn; stock's line, while the key is held |
| `MovePathColour`, `MovePathBrightness` | 0.85 0.85 0.85, 1 | the line's colour (a very light grey) and the factor on it |
| `MovePathOpacity` | 0.85 | the line's opacity |
| `MovePathDot`, `MovePathSpacing` | 1.9, 8 | the line's dots' diameter and the distance from one to the next, pixels at 1080 lines |
| `MovePointColour`, `MovePointBrightness` | 0.55 1.0 0.55, 1 | the points' colour (a light green) and the factor on it |
| `MovePointOpacity` | 0.85 | the points' opacity |
| `MovePathPoint` | 6 | the points' diameter, pixels at 1080 lines |

#### overlays: Hooks

`mods/overlays/src/hooks.rs` holds every address and field offset the module reads of
the engine, behind named accessors, and its two hooks. The step bodies (the terrain, the
selection bubbles), the two features (`Ghost`, `PlaceholderFeature`) and the pivot
marker's feature are reached through the frame's dispatch and patch nothing: while the
module is off they run as stock.

| Hook | Site | Used for | Gate shut |
|---|---|---|---|
| `DrawHeightPost` | the five calls (the build ghost's, and the overview's `mNormalModeRender`, `mRenderWhere`, `mDrawPosts` and its one-point helper) | the height posts, noted for the view and drawn as anti-aliased lines | the call that was there (`Prev`): the engine's one-pixel line |
| `mDrawPath` (`render3d.overlays.paths`) | `mNormalModeRender`'s call (0x522247), the sketch while the waypoint key is held | nothing drawn: the move paths draw the sketch (or QOL's queue) | the call that was there: the engine's line |
| `DrawHeightPost` at its entry (`render3d.overlays.posts_callee`), on a build whose profile says so (`ov_posts_at_callee`, Roots) in place of the five calls | its first six bytes (`ov_post_moved`) moved to a trampoline | the posts, noted as above only while the caller is the build's companion module (FleetOpsHook's two post callbacks, which skip a post during its clean screenshot and under `DISABLE_HEIGHT_CHANGE`); every other caller, render3d's own engine posts included, gets the function as it was | the function as it was |

*Move paths on Fleet Operations: Roots* (FO-5 step 7, verified). FleetOpsHook edits
`mNormalModeRender` at three places, none of them the paths': the cursor's height post
(+0x40) and the drag box's colour and draw (+0x191, +0x1ca); the call of `mDrawPath` at
+0x207 and the overview's fields the module reads (`+0x264`, `+0x30c`, both read the same way
by the function's own unchanged code) are stock's. Two of its changes do concern the module.
FleetOpsHook constructs the overview itself, an enlarged object in its own memory, in place
of the static one beside `g_pOverView` (it re-points the static constructor and destructor
at its own object), so the static object is never built on Roots; the module reads the
overview through `g_pOverView` (`overview_ptr`), which the constructor sets, on stock as on
Roots (stock's pointer holds the static object, so nothing changes there). And it moves the
selection's array into that object (`GetSelectList` returns another offset), which the
module already follows by calling `GetSelectList` and `GetSelectNum` through the overview's
vtable. On the bench the overview is FleetOpsHook's, the selection is read (two ships), and
the craft process's command, path and point (`+0x38`, `+0x88`, `+0x12c`) and the path's
count and points read as on stock (a stop: one point where the ship stands; a move: the
command GO and a new path at each order). The sketch itself, held `enter_waypoint` (Tab in
Roots' `Input.map` too), was not reached on the bench: with the bench's input daemon holding
Tab (and then Shift), each right-click was still a new order of its own, so no sketch was
made and no sketched path has been drawn on Roots yet.

It is installed at the first switch-on, whether or not `HeightPosts` is on, so the key
can be switched live: the detour notes a post only while one of our views is running
(`IN_VIEW`, which `HeightPosts=0` and a module that is off keep false). A site that is not
stock's is logged and left to the engine.
`mDrawPath`'s call is re-pointed at the same time, whether or not `MovePaths` is on: its
detour draws nothing while the paths are ours (`MovePaths=1`, the module on, every entry of
`render3d.overlays.paths` resolved), else calls what was there. When the entries are not
stock's no path is drawn and the engine keeps its line.

The grid's colour is what `Terrain_Geometry::DrawGrid`'s first line is drawn in, read every
frame from the pointer its `push` holds (`site_grid_colour`, 0x5934c9): stock's
`ST3D_Colour_Dark_Orange`; on Roots the file points the push at another colour and
FleetOpsHook at its own grid colour as it loads, so the grid takes Roots' colour without an
address of FleetOpsHook's. The pointer is used only when it is committed, readable memory;
stock's dark orange otherwise.

#### overlays: Content

`content.rs` holds the selection bubble's model name. The module has no recipe.

#### overlays: Settings

`[Overlays]`, read by `configure` at load and at every settings change (`Fog`, `Grid`,
`Selection`, `Ghosts`, `HeightPosts`, `Pivot`, `MovePaths`, each a live row, and the keys without a
row: the fog's filter and thresholds, the posts', the pivot's and the paths' looks). `Modules.overlays`
is live: off, the engine draws the fog, the grid, the bubbles, the ghosts, the posts
the pivot marker and the sketched move path (no path once let go), and the hooks above hand every call back.

#### Hooks, content and settings

**Hooks** (`ordnance/hooks.rs`, the only file of the module that names an address; installed
through `r3d_core::hooks` behind the module's flag, so a module switched off hands every call to
what was there before, and `hooks: ordnance <used_by> at <symbol> installed` says which):

| Symbol | Kind | What it does |
|---|---|---|
| `WarpSpriteEffect::Render`, the call of `DrawScaled3D` (0x59cc6b) | call, at the first switch-on | notes the flash for our warp step while one of our views runs and the flash is on (`[Warp] Enable`); otherwise the engine's sprite is drawn |
| `Entity::Render`, slot 11 of an ordnance vtable | slot, when a class is first met | for the classes the engine submits for us (mines, pods, the beams' models): our draw of the instance while our ordnance step submits the shot, else what was there |

`RenderWarpSpriteEffects` (the body's call at 0x598423) and each class's `Submit` are
called, not patched. The detours call the target the registry found (`Prev`), never a
constant: the warp sprite hook was seeded with `DrawScaled3D` and `ord_render` called
`Entity::Render`, which unchained any plugin that had sat on the slot first. The flash is
`[Warp] Enable` read again with the settings; with it off the hook stays in and hands the call
on.

*On Fleet Operations: Roots* (FO-5 step 7, verified). `RenderWarpSpriteEffects`,
`WarpSpriteEffect::Render` and `DrawScaled3D` are stock's in Roots' file and FleetOpsHook
writes none of them; the body's call of the first (+0x313 of `Armada_RenderAllOurStuff`) is
a plain call well past FleetOpsHook's edit of the object loop (+0x185..+0x1a4), and the
warp step calls whatever that call holds. So `ordnance.warp` is marked verified beside the
body's run-time change, and the sprite hook goes in (`hooks: ordnance the warp flash at
WarpSpriteEffect::Render (0x59cc6b) installed`). No flash has been seen on Roots yet: the long
move staged on the bench (across the minimap) did not go to warp.

**Content.** The special weapons' recipes, the models' looks and `[Warp]` are
`render3d/content/ordnance/ordnance.ini`, compiled in through `ordnance/content.rs` and
`r3d_core::recipes` (kind `ordnance`, no game directory); a section or key of the same name in
`Render3D.ini` overrides it. Nothing is installed.

**Settings.** `configure` reads `[Ordnance]`, `[Ordnance.<odf>]`, `[Model.<odf>]`,
`[Wave.<odf>]`, `[Warp]` and the `[Lights]` beam keys at load and at every
`Render3D_SettingsChanged`; the recipes, looks and flags are applied at once. When the
recipes changed, the effects' ages, the flashes in flight and the classes' choice of who
draws them are dropped; a settings change that leaves them as they were (another
module's flag) touches nothing. The module has no rows of its own: its switch is
`Modules.ordnance`, live. Switched off, `off()` drops the flashes, ages and muzzles; shots
and flashes in flight are the engine's from then on, and a switch-on starts them clean.

### particles

#### particles

`mods/particles/src/`, `mods/particles/shaders/particles*.hlsl`, on by default; stands down while
`Venting.asi` is loaded. Our own particle simulation and drawing in place of the
engine's, after `spec-particles.md` in the notes; ports `Venting.asi`'s look for the
vents (the look settled with the user, and the settings keep its keys, in
`Render3D.ini [Particles]`).

- **Who owns which emitters.** Every game object in `GameObject::objectList` whose SOD
  (named after its ODF) has emitter nodes: the damage vents, the drones' steam, a gate's
  or a colony pod's emitters. Each frame the SOD is posed with `scene::pose` at the
  object's transform scaled by its instance scale and the ODF's `ScaleSOD`, and each
  emitter fires from its node's world matrix (spray: one particle per frame at most,
  the node's +Z tilted by a random pitch and yaw within the spread; cloud: the whole
  list topped up to its count, with stock's per-frame drift taken at `CloudFps=30`).
  The flight axis is a unit one: the engine puts `ScaleSOD` into the model's positions,
  not into its node matrices, so a plume is speed × life long whatever the ship.
  Simulated once per displayed frame on the sim clock, for every object whether drawn
  or not: stock's double ageing in the action camera and its freeze of an off-screen
  object's particles are not reproduced. A destroyed object's particles go with it.
- **Damage nodes.** A craft shows or hides its damage nodes per system from the bytes at
  `+0x9c..+0xa1` of its `CraftInstance` (vtable 0x6b3e2c), which the instance's `Update`
  (slot 14, 0x4cb390) writes from the craft's systems and `CraftInstance::RenderInternal`
  (0x4cb0e0) applies: a node is hidden while its byte is set, except `borg`'s, shown
  while it is. The node of a system is the first in the SOD whose name is the engine's
  (`CraftClass::InitializeDamageNodes`, 0x4bfc70: `shields`, `engines`, `target`,
  `sensors`, `life`, `borg`), so a node named otherwise is never hidden, as in stock. An
  object a module draws in the engine's place never gets its `Update`, so the module
  calls it through the vtable (it only reads the craft; the bench's `Scene.asi` wraps
  it for `Vent=`). While the instance shows another database than its class's
  (`+0x80` against the class's `+0x1dc`), none of its emitters fire.
- **The global list.** The chain pulsar (vtable 0x6b7c74, its `crpulsar` emitter at
  `+0xdc`) and the Manheim effect (0x6b89fc, `gemini` at `+0xd4`), found in
  `Ordnance::ordnanceList` (0x771fa8): their emitters fire at the shot's transform and
  their particles outlive the shot, as the engine's global list's do.
- **No double draw.** `ST3D_DeviceDirectX8::RenderParticle` (vtable 0x6bc6ac slot 50,
  0x6262f0) is chained: while one of our views runs, a particle the engine hands over
  while its current instance (`Storm3D+0x100`) is an object we own, or none (the global
  list), is dropped. With the hulls module on the engine never renders those instances;
  with it off, it renders them and simulates their lists as before, and only their draw
  is dropped. The special-object effects' clouds are fed by the `specials` module.
  Every other instance's particles (the emitters on weapons' SODs: contrails, rockets,
  rifts) are still the engine's, drawn into our target. A module that draws such an instance in the engine's place calls
  `particles::feed` with it every frame instead.
- **Freighters' latinum spray** (`sim::object_set`, `cargo_rate`): the golden trail of
  a freighter carrying latinum. It is in no SOD: `FreighterClass`'s constructor
  (0x4a87c0) reads the ODF's `latinum_spray_hardpoint` (`hp08` on `ffreight`, `hp10`
  on `kfreight` and `rfreight`, `hp04` on `cfreight`; none on `bfreight`, the Species
  8472 collector or `fdata`, which have none) and, when the class's database has that
  node, makes the emitter its `latinum_spray_emitter` names (`freight.odf`:
  `latinum_spray`, a spray of `starfade` in `emitter.spr`) with
  `CreateParticleEmitterInstance` (0x62cf90) under the database's root, at the
  hardpoint's own transform. `Freighter::RenderInternal` (0x4b22c0) finds it by name
  (`latinum_spray`) every frame and sets its rate multiplier (`+0xf8`, which
  `ST3D_ParticleEmitter_Spray::Simulate`, 0x612a70, takes the time step times) to the
  amount over the capacity while the resource carried (`+0x25c`) is latinum (3), else
  0: full, a particle every firing period; half, every second one; empty or another
  resource, none. Other resources make no trail; they light the cargo markings in
  their colour ("hulls", "A freighter's cargo markings"). The module adds that
  emitter to a freighter's set (vtable 0x6b26a8): placed at the hardpoint's transform
  times the object's, shown whatever the damage and the level of detail, and counted
  among the owner's shown emitters (Q1) at any rate, as the engine renders the node
  whether it fires or not. The rate is read every frame, for every freighter drawn or
  not.
- **The look.** Drawn at `Step::Translucent` as one batch per texture and material
  through the frame's vertex ring: quads parallel to the image plane, half extents = the sprite node's
  size plus growth × age (times the instance's scale), the node data's colour animation
  and the sprite's flipbook on the particle's age, additive. Each particle's colour is
  multiplied by 1 + the emitters its owner showed this frame, which is what stock's
  repeated hand-off of the whole list by every emitter node adds up to. The vents
  (`ventplas`, `ventgas`) are drawn as `Venting.asi` drew them (`vent.rs`,
  `particles_vent.hlsl`): billowing puffs that cool, linked into one trail behind a
  moving ship, a glow and a jet core at each vent, and a light on the hull.
- **Vent lights.** One `ST3D_Point_Light` per vent (up to 16; `operator new` 0x652710,
  the constructor 0x62f240), registered for each view with
  `ST3D_GraphicsEngine::RegisterLight` (0x62d250) so the hulls the engine draws take it,
  and added to the scene layer's list for ours.
- **Billboard effects** (the transporter shimmer and the scripts' `AddBillboardEffect`):
  the node data at `GameObject+0x3c`, drawn at the object's bounding sphere
  (`Entity::GetBoundingSphere`, 0x4cfd70) centre pulled towards the eye by its radius,
  that radius in half size. The engine's call to `RenderSpriteNodeEffect` (0x62ebd0) in
  `GameObject::RenderInternal` (0x4d544c) is re-pointed at a stub that skips it while
  our view runs.
- **Nebula puffs.** `object()` takes nebulae (vtable 0x6b148c) and latinum nebulae
  (0x6b132c, drawn by the plain `GameObject::Render`, which is why the module is
  registered before hulls) whose class has no volume recipe (`Nebulae\<odf>.ini` while
  the world module or `Nebulae.asi` runs): their SODs' sprite nodes as additive
  billboards (half extents) or, for `lightning#`, quads of the full size in the node's
  XY plane, coloured by the class's animation on absolute sim time. `ScaleSOD` spreads
  the nodes but does not grow the quads. `Nebula::sCullOccludedNebula` still runs in the
  step body: its hidden flag at `+0x1b0` skips a nebula, its intensity at `+0x1d0`
  dims it, times `GameObjectInstance::ComputeFadeOut` (0x4d5a20) when it reports a near
  fade, as `NebulaInstance::ComputeFadeOut` (0x4a5680) does.

| Key (`[Particles]`) | Default | |
|---|---|---|
| `Enable` | 1 | 0: the engine draws its particles, as without the module (read again every 2 s with `Reload=1`, for a comparison at the same moment) |
| `CloudFps` | 30 | the frame rate a cloud particle's per-frame drift is taken at |
| `EngineLights` | 1 | 0: the vents' lights only for the hulls we draw |
| `LogEvery` | 600 | frames between log lines |
| `Plasma`, `Gas` | 1 | 0: that kind of vent is the engine's sprite, stock's look |
| `Plasma<K>`, `Gas<K>` | | the vent look, one key per kind: `Brightness`, `Size`, `Stretch`, `MinPixels`, `Flicker`, `Cooling`, `Glow`, `GlowSize`, `GlowMinPixels`, `GlowColour`, `Jet`, `JetLength`, `JetWidth`, `Light`, `LightColour`, `LightReach`; also `PlasmaHot`/`Mid`/`Cool`, `GasBright`/`Cold`, `BillowScale`/`Flow`/`Contrast` and `EdgeSoftness`. Defaults in `Cfg::read` (`vent.rs`) |

#### Hooks, content and settings

**Hooks** (`particles/hooks.rs`, the only file of the module that names an address;
installed through `r3d_core::hooks` at the module's first switch-on, behind its flag):

| Symbol | Kind | What it does |
|---|---|---|
| `ST3D_DeviceDirectX8::RenderParticle` (vtable 0x6bc6ac, slot 50) | slot, chained | drops the engine's draw of a particle handed over while the engine's current instance is an object we own, or the global list, in a view of ours |
| `GameObject::RenderInternal`, the call of `RenderSpriteNodeEffect` (0x4d544c) | call | skipped in a view of ours, since we draw every object's billboard effect; if the site is not stock's the engine's call draws them and we do not |

*On Fleet Operations: Roots* (FO-5 step 7, verified). In `GameObject::RenderInternal`
Roots' file detours two wave-light calls (+0x9c, +0x1fc) and FleetOpsHook rewrites the
cloak's two tests (+0x1f, +0x1a1 and the bytes after each); the billboard call at +0x19c,
its three pushes and its callee are stock's in the file and in memory, and FleetOpsHook's
second cloak call begins on the very instruction the billboard call returns to (and where
the branch that skips it lands), so the call site is whole. The function still runs for
every object Roots' loop draws (the hulls claim one level further down, at the instance's
database draw), so skipping the engine's call in our views and drawing every object's
effect from the object list stays the same trade as on stock.

Both call what was there before (`Prev`) when the gate is shut: the billboard stub called
`RenderSpriteNodeEffect` by its constant, which dropped a plugin chained onto the call.
They went in from the first `lights()` before; they are installed at the first
switch-on now, so a flip of `Modules.particles` live has them in place. If the particle
slot is not stock's or chained, the module stands down rather than draw over the
engine's.

**Content** (`particles/content.rs`): the damage systems' node names, the vent sprites
and the two free emitters of the chain pulsar and the Manheim effect. Their offsets
and vtables are in `hooks.rs`.

**Settings.** `configure` reads `[Particles]` at load and at every
`Render3D_SettingsChanged` (and, with `Reload=1`, every 120 frames, as it did). The
module has no rows of its own; its switch is `Modules.particles`, live. Switched off,
`off()` drops the simulation, the owned set and the feed queue the other modules filled,
so a switch-on starts clean: particles in flight at the switch end with it, and the
engine's draw is back at once. `particles::live()` is false while it is off, so the
modules that feed it (ordnance's models, effects, specials) hand their emitters to the
engine.

### hulls

#### hulls

**Layout (phase 7).** `hooks.rs` is the engine glue: every address, call and field offset
the module reads of an object and its instance, behind named accessors (it was
`game.rs`). The module installs no hook of its own: the frame's object loop calls it,
and it draws `GameObject::Render` by returning true there, so with it off the engine's
own draw stands as stock (`Modules.hulls` is live, checked by `render3d-live-hulls`).
`content.rs` holds its names of the game's data (SOD nodes, the cocoon's SOD, the
scrap glow's sprite) and stock's resource colours. The scene keys are the core's
(`core/src/scene/scene_lights.rs`, the `SceneLightSource` the core registers itself): the
`Lighting.asi` test is made at the first gather (every plugin is loaded by then, not at
load), and the replacement goes on with the module at 0 or not built in, since the other
modules light by it. `configure` reads `[Hulls]` at load and at every settings change; `Reload=1` and `ABTest` (the
bench) work as before. The module has no settings row of its own beyond
`Modules.hulls`. `Style`/`set_style` stay in `scene/lights.rs`: they are a light's
semantics for the cluster pass, set by effects and ordnance, and no hulls code reads or
sets them.

`mods/hulls/src/`, `mods/hulls/shaders/hulls.hlsl`, on by default; supersedes `Lighting.asi`'s
hull path (and stands down while a stale `Lighting.asi` is loaded). Settings in `[Hulls]`
(Lighting.asi's keys and settled values: `Ambient`, `Specular`, `SpecularPower`,
`RimLight`, `RimPower`, `HullSun`, `HighlightKnee`, `SelfIllumination`, the `Borg*`
profile, `PointLights`, `NearFade*`, `Cloak*`, plus `Shadows`, `ShadowSize`,
`ShadowStrength`, `PlanetShadows`, `BumpStrength`, `SpriteNodes`, `AdditiveMeshes`,
`TeamColours`, `Scrap`, and the damaged sensors' `SensorSteady`, `SensorSize`,
`SensorLight`, `SensorLightReach`, under "Sprites") and the scene lights in `[Lights]` (`Replace`, `KeyColour`,
`KeyAxis`, `FillColour`, `FillAxis`, `SkyLight`).

**What it claims.** An object whose class's `Render` (vtable slot 11) is
`GameObject::Render` (0x4d54e0), `Shipyard::Render` (0x4bb900, Starbase too),
`MiningStation::Render` (0x4b44a0), `ConstructionRig::Render` (0x4af9b0) or
`Scrap::Render` (0x4a58e0), with a `RenderInternal` (slot 22) of `GameObject`
(0x4d52b0), `Craft` (0x4c8630) or `Freighter` (0x4b22c0), whose instance (+0x80) draws
a plain `ST3D_Database` (vtable 0x6bc56c) that is its class's visible database
(`GameObjectClass+0x1dc`); and the Evolver (`RenderInternal` `Evolver` 0x4b1170, "The
Evolver" below, `[Hulls] Evolver=1`). Anything else (planets, other databases) is
left to the engine. In `object` the module does what `GameObject::Render` does around
the draw: expired and unseen objects draw nothing; an Evolver's cocoon first ("The
Evolver"); a hologram (+0x174) in place of the hull ("Holograms"); +0x26 suppresses the
draw; a rig's next station as a build ghost for the player's team ("The build ghost");
the instance's
frustum test (instance slot 3, as `Entity::Render` makes it) is called, not redone,
because for a craft it also applies the scale effects into the instance's scale
(+0x28, valid while +0x74); a pass sets the on-screen byte (+0x25) the HUD reads. Those
are the only writes; the instance's `Update` (slot 14) is called as stock calls it
before it reads the damage bytes.

**Posing** (`begin_view`). Every object the object loop will reach is posed for the
view (the scene layer's `pose`, on the instance's clock: start +0x18, flags +0xc), with
the nodes the engine shows or hides per instance by writing into the shared database
applied to this pose instead: `hardpoints` never; `lights`/`light` at detail level 0
(`ST3D_GraphicsEngine_Configuration+0x40`) or while the instance's +0x90 says so; the
damaged systems' nodes (`CraftClass::InitializeDamageNodes`: class +0x40c/+0x418/+0x410/
+0x414/+0x41c against the instance bytes +0x9c..+0xa0) hide their children; the crew
fires (class +0x408) shown one more for each fifth of the crew lost (`+0xa4`, as
`CrewDamageTraverse`, children in the engine's reversed order); `borg` (class +0x420)
only while assimilated (+0xa1, which also swaps every mesh to its second texture as
`CraftClass::SetBorgMeshTextures` does); the shipyard's build node (class +0x4a4, shown
while +0x2b8 is not `NO_ENTITY`) and the mining station's dock node (class +0x4b0).
`hulls::posed(o)` gives other modules this view's pose of an object (every node's
world matrix and whether it is shown).

**The look** (`hulls.hlsl`). `Lighting.asi`'s: ambient, the scene's directional lights
with the Key times `HullSun` and taken away by shadows, Blinn-Phong specular weighted by
the texture's brightness, the rim light, self-illuminating groups folded in by the
texture's alpha, team colour on `team*` groups (the team's colour, `Team::GetTeam(+0xec)
+ 8`, or the player's own in a game of type 2, while `Storm3D+0x40` allows team colours),
and its highlight shoulder: the lit hull rolls off towards white above
`HighlightKnee` and stays within 1, as it did when `Lighting.asi` drew into the same
target, so a hot spot whitens plating without blooming over the hull. Bump-mapped
meshes (the Borg) take the Borg profile and a normal map built from the height map as
`ST3D_CreateNormalMap` builds it, in a tangent frame worked out per pixel. On Fleet
Operations: Roots nearly every hull is bump-mapped, as a switch to the engine's GPU
vertex path rather than a look, so the content set (`fleetops`, `hulls.ini`) says
`ProfileBy = race`: the Borg profile goes to the hulls whose ODF `race` (the key is the set's `RaceKey`, default `race`; read through its
includes, from `ODF.fpq`) is in `ProfileRaces` (`borg`, `freeborg`; `[Hulls] BorgRaces=`
replaces the list), every other hull takes the hull profile, bump-mapped or not. Roots'
`*_bump` textures are tangent-space normal maps already (mean about 128, 128, 254;
measured), so `BumpMaps = normal` uploads them as they are, and a flat one (`all_bump`,
one colour, used by 976 meshes) counts as no bump map: the plain hull shader. Normals:
the SOD's winding makes them inward, as the engine's are (`scene::NORMAL_SIGN` = -1
turns them out); a mirrored node swaps the cull and needs no flip of its own. A mesh's
texture flipbook (a SOD animation reference) and the engine's wave lights
(`GameObject::RenderInternal`'s four special-weapon getters, `Craft::RenderInternal`'s
white one while +0x240; colour +0xf4, wavelength +0x13c, speed +0x140, direction
+0x144) are reproduced; the construction face limit (+0x10) too.

**A freighter's cargo markings** (`game::cargo_markings`). `Freighter::RenderInternal`
(0x4b22c0) calls `Freighter::UpdateDilithiumMarkings` (0x4b1fa0) before
`Craft::RenderInternal` draws: the children of the node `ore` that
`Freighter::InitializeGeometry` (0x4b1660) found in the database (`+0x250`) are
numbered by their names after three characters (`ore1`..`ore10`), and each one's
lighting material gets the colour of the resource carried
(`ResourceComponent::s_ResourceColor`, 0x764664, a pointer per resource: crew green,
officers white, dilithium pale blue, latinum yellow, metal grey, biomatter red) while
its number times 10 is at most the fill in percent (amount `+0x258` over the class's
capacity for the resource `+0x25c`, from the float per resource at `+0x254`), black
otherwise. So the cargo pods are a gauge in the cargo's colour, for every resource;
only latinum adds a trail (the particles module, "Freighters' latinum spray"). The
colour lands on the shared material, and the engine walks children last in the file
first, so where meshes share one material the first child in the file decides for all
of them: the Klingon freighter's ten share `ore` (its first is `ore10`: lit only when
full), the Romulan's too (`ore1`: lit from 10 %), and the Federation's have one each
(a true gauge); the Cardassian and Borg freighters have no `ore` node. Seen on the
bench in stock (2026-10-21: the Federation's three pods of five at half, the
Klingon's dark at 80 %, the Romulan's lit at 13 %). The module draws those materials'
groups unlit in that colour, black when off: the hull look ignores a SOD's lighting
material colours, which drew the pods as plain lit plating whatever the hold.

**Alpha-tested meshes in old SODs** (`scene::mesh_blend`). Before version 1.7 a mesh
names its texture material by an ident in the file's table (`a2fmt`'s
`TextureMaterialDef`), not by a blend mode, and `ST3D_Stream::ReadTextureMaterial`
(0x63eb10) builds each with alpha testing on unless both of its first two mode bytes
(the material's two `ST3D_BlendMode`s) are 2. So such a mesh is a cut-out, its
texture's alpha holes: the Klingon and Romulan freighters' cargo pods are lattices
over the `ore` markings, which were drawn solid and hid them. Of the stock files, 70 are
older than 1.7; their meshes with modes `2 1` (275) are cut-outs, those with `2 2` (46)
are drawn as before. The textures among them with alpha are `Kfreight`, `Rfreight`,
`Bcruise1` (the assimilated parts), `Bbuildng`, `Fbee`, `Fresear` and `Mdmoon`
(`Mmooninf.SOD`).

**Point lights**, per mesh, as `Lighting.asi`'s `pick_points` chose them for its
shaders: a hard light (fading over under a quarter of its start: torpedoes, the
shields-down discharges, the welds) is picked within its reach plus 150 and scored by
its peak and nearness; a soft one (nebulae, explosions) within its reach, scored by its
falloff at the mesh's origin; of one colour only the strongest; at most `PointLights`
(12). The shader takes each where it is, falloff per pixel. A module that registers a
light may say how it is to be taken with `scene::lights::set_style` (hard, never merged
by colour, wrap, falloff power): the ordnance module's phaser emitter (hard, distinct,
wrap 0.6, power 2) and impact (hard, distinct) lights, as `Lighting.asi` took its own.
The engine's stock meshes have inward normals, which is why a point light given where
it is lit the far side of a hull in `Lighting.asi`'s fixed-function path (it handed a
soft light to Direct3D mirrored through the draw's origin); the shaders here take every
point light where it is and turn the normal round instead (`scene::NORMAL_SIGN`), so no
mirroring is needed.

**Clustered lights** (`scene/clusters.rs`, `core/shaders/clusters.hlsl`,
`core/shaders/clustered.hlsli`; `[Lights] Clustered=1`, Vulkan and Direct3D 12 only). Per
mesh, the strongest twelve lights pop as a volley passes, and a light that falls on
one end of a station counts for the whole of it. So on a backend with compute every
light of the view lights every pixel it reaches, hulls, planets, moons and asteroids
alike:

- **One list a view.** After every module has registered its lights (the body's
  `light clusters` mark, after `mods::begin_view`), `clusters::build` takes the
  scene's list (`scene::lights()`: the engine's records, torpedoes, pulses, shield
  hits, welds, vents, fireballs, the modules' points) and the shapes render3d adds
  for the view: a phaser beam as a **line light** (`add_line`, `BeamLight`; "Beams",
  below) that lights the plating, rocks and planets it passes, a tractor or
  assimilation beam's wash as a **spot
  confined to its cone** (`add_spot`, `ConeSpot`: from the emitter, as wide as the
  cone, out past the target; ART-DIRECTION.md, "It lights the target"). The ordnance
  module's own points (phaser emitters and impacts, muzzles, the fx lights) go into
  the scene's list as points of its own rather than the engine's light objects, which
  spares the engine's heap and the 32-bit address space a light object each.
- **Groups.** Lights of one colour share a group, as `pick_points` merged them (a
  volley's bolts are one colour); a styled `distinct` light, a line, a spot and a
  planet's day side are each their own. The list goes up sorted by group.
- **The cull** (compute, one thread a cluster): the view's rectangle in tiles of
  `ClusterTile` pixels by `ClusterSlices` slices of view depth, exponential between the
  projection's near and far planes. Each cluster's box in view space is tested against
  each light's reach (a point's sphere; a line's distance from the box's middle less
  its half diagonal; a spot's sphere about its cone), and the cluster keeps at most
  `ClusterLights` of them and `ClusterPerColour` of a group, the strongest at the box,
  in the list's order.
- **The shading.** A lit draw binds the view's buffers (`clusters::bind`: storage
  slots 0 and 1, pixel constants c100..c103) and draws with its pixel shader's
  clustered twin, which walks its pixel's cluster in place of the point lights in its
  constants: each light's falloff (start, fade, power) and wrap as before, a line from
  its nearest point, a spot only inside its cone, a day side by the share facing the
  pixel; and of each group only the strongest at that pixel counts, so a volley of
  one colour lights as its brightest bolt did, without the per-mesh cut. The highlight
  shoulder keeps plating within 1, and the Borg profile and the inward normals are as
  before. Cloaked hulls, ghosts and holograms keep the per-mesh lights. The nebulae's
  march reads the same clusters for the light inside the gas (`[Nebulae] Lit=`, under
  world, "Nebulae"); each light's `t.z` says what it is to the gas (its medium, below).
- **Media and flags** (`scene/media.rs`, light semantics as data). What a light is to
  the gas is a *medium*: a module names one with `lights::register_medium(name,
  MediumParams)` (or a `static lights::Medium`, registered on first use) and marks a
  light with `set_medium`, or passes it to `scene::add_point` for a point it adds by
  hand. A medium is a response of the march (`Scatter`, `Fireball`, `Corona`, `Unlit`)
  and the gas share it takes (`LitExplosion=`, `LitBolt=`, `LitSpark=`), and the march
  knows its responses by the literal it reads from `t.z`, so the slot is fixed by the
  response in `media::MEDIA`, not by who registers first: 0 `plain` (the core's), 1
  `fireball` and 4 `spark` (effects; hulls' sensor glows take `spark` too), 2 `bolt`
  (ordnance), 3 `own_glow` (world); 5 to 7 are reserved. The shaders are unchanged by
  this: `render3d/hosttest` (`cargo test`, in CI) asserts the packed word is the one
  the old `Gas` enum gave, for every kind with and without the day side, that the
  march's literals are the table's slots, and pins the falloff functions by hash.
  `FLAG_SHARE` (`t.x` bit 4) is a generic flag: a light that lights what is near it
  times the share of a sphere about it facing the Key, never that sphere; world sets
  it on its planet glows and keeps `is_day_side` to itself. A line light's parameters
  (`LineParams`) and a spot's (`SpotParams`) come from the module that adds it
  (ordnance, from the `[Lights] Beam*` keys).
- **Per view.** Each view (`View::key`: the HUD's action camera draws the scene a
  second time) has its buffers; a draw binds them only in the view they were built for
  (the menu scene has none).
- **Cost** (`Timing=1`; the `battle` scene, sixteen ships and a planet, 73 lights,
  1280x720, RX 5700, 2026-10-23): the cull 0.20 ms of GPU time a view on Vulkan and on
  Direct3D 12; the objects step 0.91 ms against 0.90 without (Vulkan), 0.45 against
  0.42 (Direct3D 12). At 3440x1440 (the user's) the cull was 0.20 ms (Vulkan) and
  0.19 ms (Direct3D 12) and the objects step 1.38 and 0.95 ms. The cull's time hardly
  moves with the resolution (29808 clusters against 5760) and goes with the lights
  (0.04 ms for the 32 of `nebula-battle`): it is each thread's walk of the list, not
  the number of threads. So the lights come through group-shared memory, put into
  view space once a group; a group leaves out the points and spots that cannot reach
  its slices; and the per-colour count and weakest are kept as the sorted list goes
  by, the list scanned only to replace an entry and sorted only when one was: 0.16
  and 0.17 ms, the same clusters (the frame at RMSE 0 on both). What remains looks
  like the private arrays' traffic and the upload and barriers round the dispatch.
- **Off** (`Clustered=0`): nothing of it runs, and every draw is as
  before: the port scenes draw the same pixels as without it (`gpu/README.md`,
  "Measured").
- **Beams** (`cl_line` in `clustered.hlsli`). A phaser beam emits alike along its
  whole length, so what reaches a point is the sum of 1 / r² over the segment: the
  angle the segment spans from the point over its distance from the beam's line,
  `(θ1 − θ0) / h`. Beside any part of a long beam that is the same (π / h), so no part
  of the beam lights more than another; it falls as 1 / distance, and past either end
  it dims by itself as the angle closes. It is scaled to `BeamLight` at `BeamCore`
  from the middle of a long beam (nearer, `h` is held at the core, so a hull the beam
  grazes does not white out), and windowed, `(1 − (d / BeamReach)²)²` of the
  distance from the segment, to nothing at `BeamReach`, where the cull stops. Its
  direction is the representative point: the point of the segment at the middle of
  the angle it spans (beside the middle, the nearest point; near an end, towards the
  rest of the beam). The two ends are brighter by points of their own, the ordnance
  module's emitter and impact lights (`PhaserBrightness`, `PhaserImpact`, under
  ordnance). In the gas the line is taken as it was tuned for the march: from its
  nearest point, full to a tenth of `BeamGasReach` and gone at it, in the beam's
  colour (`BeamLight` does not scale it; `[Nebulae] LitBeam=` does).
  Until 0.26.0 the line was a point light stretched along the beam, full to 4 units
  and gone at `BeamReach=40` with its falloff squared: a Galaxy class is about 100
  units long, so a beam passing a hull or a rock 30 units off lit almost nothing (the
  bench, `beam-pass`; the user, in the Klingon intro's fire over the station). An
  install replaces a player's `BeamLight=1.0` and `BeamReach=40`, the old defaults,
  by the new ones (`install.sh`, `RETIRED`); a value they set stays.
  Cost (`Timing=1`, the `battle` scene, 1280x720, Vulkan on the RX 5700,
  2026-10-23): the objects step 1.27 ms of GPU time against 0.91 with the old line
  and 0.76 with no beam lights (`BeamLight=0`); the cull 0.22 ms against 0.18. The
  time goes with how many clusters a beam reaches: `BeamReach=120` takes the objects
  step to 1.16 ms. The angle is one `atan2` and the direction a bisector (two
  `atan2`s and a `tan` cost the same within 0.03 ms).
  With `Clustered=0` (and always on the retired Direct3D 9 path) a beam lights nothing but its two end
  points: the per-mesh constants hold twelve points, and a point standing in for a
  line would light one patch of it, not the length.

**Scene lights.** `scene::Lights::gather` replaces the map's directional lights with
`Lighting.asi`'s Key, Fill and sky light (the sky's mean colour weighted by chroma,
from its brightest side; the background name is read where
`Starfield_Load_Background_Geometry` keeps it, 0x738538), unless `Lighting.asi` is
loaded or `[Lights] Replace=0`.

**Shadows** (`shadow.rs`). At the start of each view every posed hull that can throw a
shadow into it is drawn into an R32F depth map from the Key, fitted round them
(width in steps of 2^(1/4), centre snapped to texels): this frame's hulls, not the last
frame's as `Lighting.asi` had to. Planets (vtable 0x6b2b3c) are tested as spheres.
`hulls::shadow_map()` lends it to other modules as `LightingShadowMap` did.

**Fades, cloak, warp.** The far fade is the engine's (`GameObjectInstance::
ComputeFadeOut`, 0x4d5a20); crafts take `Lighting.asi`'s near fade (the near plane
cutting into the hull's box) while `Storm3D+0xe8` says near fades are on. Faded hulls
are drawn at `Step::Translucent`, far to near, depth first. A cloaking device (+0x108:
state +0x3c, alpha +0x44, phase +0x48) draws the hull as `Lighting.asi`'s glass over a
copy of the frame, in the look of `ART-DIRECTION.md`, "Cloaking". While it cloaks or
decloaks, bands travel along the hull's longest axis (`CloakWaveBands` over its
length, `CloakWaveSpeed` a second) and `vs_hull` sways the geometry across the screen,
at right angles to the bands and never in depth, by up to `CloakWave` of the radius
(swayed through the hull's own axes it bent in 3D and looked comical from the RTS
camera), on the envelope `4·vis·(1−vis)`; the sway is a
function of the world position, so the parts stay joined. The hull thins into the
glass band by band (crests first) in place of the old noise threshold, and the green
front (`CloakFront`) is a thin line along each band. Fully cloaked, slow waves drift
in the refraction offset (`CloakShimmer` of the radius, `CloakShimmerSpeed`): only the
bent background moves, so it shows against a nebula and hardly against black. The
hull's floor is per owner: `CloakHull` (0.1) for the player's own ships, by the
object's team (+0xec) against `Entity::GetUserTeam` (0x4d0060, the call `QOL.asi`
makes), and `CloakHullOthers` (0) for everyone else's. Only hulls the engine already
draws are touched. The warp stretch is `Craft::RenderInternal`'s while +0x244 is above
0 (`spec-chunks-anim-lod.md` §5).

**Sprites** (`sprites.rs`, `anim.rs`). SOD sprite nodes (running lights, strobes, crew
fires) and the billboard effect pinned to an object (`GameObject::SetBillboardEffect`,
+0x3c) are drawn as `ST3D_SpriteNode::RenderInternal` and `ST3D_Instance::
RenderSpriteNodeEffect` draw them, their animations played as `ST3D_AnimationChannel::
Evaluate` plays them (quirks kept: unevenly judged keys played evenly, `linear`
wrapping), at `Step::Translucent` after the hulls. A scrap object's `glowdmoon` node
takes `Scrap::Render`'s colour.

**Damaged sensors.** A craft's `sensors` group holds the `sensor` sprite nodes
(`sensorftur` and the rest on Federation stations). `damage.spr` draws them as the
`origin` sprite, green: the `xspark01` spark flipbook, 16 frames in 0.5 s. They show
while the craft's sensors are down (byte `+0x9f`, `game::DAMAGE_NODES`), and in the
Mutara nebula, which takes every ship's sensors down, that means on most Borg ships
(the scout has two). Played as stock, the flipbook read in game as a flickering green
light. The user wanted it steady, smaller, and a light source. So, with
`SensorSteady=1`, such a node is held on the flipbook's first frame (a full star) at
`SensorSize=` (0.5) of stock's size. Each shown node is also a steady point light of
the node's colour times `SensorLight=` (2), reaching `SensorLightReach=` (4) times its
drawn half size: a small green glow on the plating beside it (about 7 units on a
scout). `begin_view` adds these lights to the scene's list by hand, at most 32 a view.
The cluster list takes them, and the gas takes them as sparks (the `spark` medium). A
cloaking hull gives none.

**Holograms** (`TimedHoloEffect`, the object's `+0x174`). `GameObject::RenderInternal`
draws an object with a hologram as the class the hologram shows, in place of its hull:
`TimedHoloEffect::RenderHologram` (0x60f730) calls that class's `DisableSpecialNodes`
(class vtable slot 7) and renders the effect's `ST3D_Instance` (`+0xb4`), made by
`TimedHoloEffect::init` of the class's (`+0xc4`) visible database `+0x1dc` and put at the
object's transform every tick by `Simulate`; the call's answer becomes the on-screen
byte. Two weapons make one: the Romulan holo-emitter (`gholoemt` on `rspecial`; the
target shows as the firing ship's class, 0x57a8ff) and the holo-generator (`ghologen`,
`HoloGenWeapon::Simulate` 0x66b711: the ship disguises itself as its target's class).
Here (`Holograms=1`): the class's SOD (named by `GameObjectClass::GetOdfName` 0x4ce370,
at its `ScaleSOD`) at the instance's transform, clean (`hardpoints`, the craft class's
damage nodes' children, its crew fires and Borg node hidden), its running lights drawn,
the instance's frustum test (slot 3) for the on-screen byte, and drawn at the flush by
`ps_holo` in the object's team colour: the lit hull of the class it shows, drawn part
of the way towards that colour (`HoloTint`), at `HoloAlpha`, scanlines `HoloScanPitch`
pixels apart rolling down the screen (`HoloScan`, `HoloScanRoll`), a bright band
sweeping along the hull's up axis (`HoloSweep`, `HoloSweepRate`), a stepped flicker
(`HoloFlicker`) with rare dropouts (`HoloDropout`), and a glow at the silhouette past
white (`HoloEdge`, `HoloEdgePower`) that blooms; premultiplied, after a depth pass. The
disguise still reads as the ship it shows. The real hull is not posed for the shadow
map while it wears a hologram. A hologram whose class is not a plain SOD is the
engine's.

**The Evolver** (8472's `8472_passive` and `8472_active`, class label `evolver`).
`Evolver::RenderInternal` (0x4b1170): while it evolves it has a cocoon (`+0x2ac`, a
`CocoonInstance`, vtable 0x6b2630, of `8472_cocoon.sod`, whose database the engine
finds once and keeps at 0x73fccc), culled by distance (`GameObjectInstance::
DistanceCull`), its strategic-view scale effect made (0x4d5eb0) and its scale effects
applied (`ApplyScaleEffects` 0x599760, which `Evolver::mUpdateConstructionEffect`
0x4b0a10 sets every tick to grow it from the Evolver's box to the new class's), then
drawn; past a third of the evolution (`+0x2b0`) only the cocoon is drawn and its
answer is the on-screen byte, before that the ship is drawn too (`Craft::
RenderInternal`). Here the same, the cocoon posed at its instance's transform and scale
and drawn as a hull, solid: stock's fade for it (`CocoonInstance::ComputeFadeOut`
0x4b1320, the base far fade times a ramp `+0x98` over the first third) comes out 0 on
the bench throughout (the base reads the cocoon's `+0x98`, an instance's class
elsewhere, as a class), yet stock's draw shows the cocoon solid from its first frame.
The burst is the effects module's debris ("Debris"). `Evolver=0` leaves the Evolver
to the engine.

**The build ghost** (`hulls::placeholder`, for the overlays module's
`PlaceholderFeature` and for `ConstructionRig::Render`'s next item). Stock
(`PlaceholderFeature::sRenderInternal`, 0x473aa0) makes a `PlaceHolderInstance` (a
`CraftInstance`, vtable 0x6b0094) on the stack of the class's databases, culls it by
distance, hides its `lights` (`+0x90`), takes the strategic view's scale, sets the team
colour to the colour it is given and renders it; `PlaceHolderInstance::ComputeFadeOut`
(0x473c30) halves the far fade, or gives a half. So it goes down the engine's sorted
translucent path: every layer of the hull (the inside through the plating) blended at
half, which reads pale and washed towards white with hot patches where layers pile up.
Then `DrawHeightPost` from where it stands. Here: the class's SOD clean, `lights`
hidden, culled by distance as `DistanceCull` decides (the height counted a quarter,
against `cfgOBJECT_CULLING_DISTANCE`), at the flush with every layer (`GhostDepth=0`;
1: only the nearest), by `ps_ghost`: the lit hull lifted towards its texture at full
light (`GhostLift`), a broad highlight towards each directional light past white
(`GhostSheen`, `GhostSheenPower`) that blooms, the blocked placement's red added on the
texture (`GhostBlocked`, stock's ambient), at `GhostAlpha` (stock's half); then its
additive meshes; its height post through the overlays module. A construction rig's
next station (`ConstructionRig::Render`, 0x4af9b0, the player's own rig, through
`PlaceholderFeature::sRenderInternal` at 0x4afa11) is drawn the same way.

| Key (`[Hulls]`) | Default | |
|---|---|---|
| `Holograms` | 1 | 0: the engine draws holograms (the class plainly) |
| `HoloAlpha`, `HoloTint` | 0.8, 0.35 | how opaque, and how far towards the team colour |
| `HoloScan`, `HoloScanPitch`, `HoloScanRoll` | 0.35, 3, 0.5 | the scanlines' depth, pitch (pixels) and roll (pitches a second) |
| `HoloSweep`, `HoloSweepRate` | 1.2, 0.35 | the sweeping band's strength and sweeps a second |
| `HoloFlicker`, `HoloDropout` | 0.12, 0.03 | the flicker's depth and the chance of a dropout (6 a second) |
| `HoloEdge`, `HoloEdgePower` | 2.5, 2.5 | the silhouette's glow and its exponent |
| `Evolver` | 1 | 0: the Evolver and its cocoon are the engine's |
| `BuildGhost` | 1 | 0: the build ghost (and a rig's) is the engine's |
| `GhostAlpha`, `GhostLift` | 0.5, 0.5 | its opacity (stock's half) and how far the plating is lifted |
| `GhostSheen`, `GhostSheenPower` | 1.5, 6 | the broad highlight past white and its exponent |
| `GhostBlocked` | 0.5, 0, 0 | what a blocked placement adds |
| `GhostDepth` | 0 | 1: only the ghost's nearest layer |
| `Reload` | 0 | 1: `[Hulls]` read again every second (the bench; itself read every second) |

**The survey.** `mods::object` logs, once per class (vtable), every object no module
draws: `survey: left to the engine: vtable ... (Render ..., RenderInternal ...),
database vtable ..., odf '...'`.

**For other modules**: `posed(o)`, `shadow_map()`, `draws_plain_sod(o)`,
`instance_scale(inst)`, `placeholder(v, class, place, colour, blocked)`, and
`draw_instance(v, key, inst, craft, sod, world, team)`: an
`ST3D_Instance` that is no game object's (a station being built, a worker bee) drawn
now with the hull look, its additive meshes and sprite nodes at `Step::Translucent`
(the effects module's construction and bees). Under a face limit (`+0x10`) the faces
are counted in the engine's walk of the node tree (children in reverse file order),
groups and faces in file order (`fbase` on the bench: the engine's mesh groups and
first faces matched the file's), and the hull is drawn two-sided, as the engine's
limited draw shows the inside of a part-built station.

### world

`mods/world/src/`, `mods/world/shaders/world_*.hlsl`, on by default, registered first (it takes
the nebulae and the moons before `particles` and `hulls` see them). The objects whose
class overrides `Render` and draws a world of its own: planets, the dilithium moons,
asteroid fields, nebulae. It ports `Surfaces.asi` (the recipes, `Surfaces\<texture>.ini`),
`Lighting.asi`'s planets and its nebula and planet lights, `Planets.asi` (moot: our own
sphere) and `Nebulae.asi` (the recipes, `Nebulae\<odf>.ini`); the looks settled with
the user are kept. Each part stands down while a stale copy of the plugin that drew it
is loaded: the planets beside `Lighting.asi`, the moons and asteroids beside
`Lighting.asi` or `Surfaces.asi`, the gas beside `Nebulae.asi`, the lights beside
`Lighting.asi`. The recipes live in this folder: `content/sky/` (built into the plugin),
`content/surfaces/` and `content/nebulae/` (copied by `install.sh` into the game's
`Surfaces\` and `Nebulae\`, where an edited copy is re-read while the game runs, and
built into the plugin too: "Content" below); see "Recipes" below. Settings in `[World]` (the parts: `Planets`, `Rocks`, `Nebulae`,
`Surfaces`; the planet look under `Lighting.ini`'s names: `PlanetAmbient`, `PlanetDusk`,
`PlanetWrap`, `PlanetFill`, `PlanetSun`, `PlanetHaze*`, `PlanetGlint*`, `CityLights`,
`CityLightColour`, `PlanetHalo*`, `PlanetGhost`; the rocks' `RockAmbient`, `RockSpecular*`,
`RockRimLight`, `RockRimPower`, `RockSun`, `MoonShell`, `MoonCracks`, `MoonPulse`,
`MoonCrackCore`, `MoonCrackDepth`, `MoonCrackFalloff`, `MoonNimbus`; the lights' `PlanetGlows`,
`PlanetGlow`, `PlanetGlowRange`, `NebulaLights`, `NebulaBrightness`, `NebulaRange`,
`NebulaRecipeColour`, `NebulaCull`) and `[Nebulae]` (`Enable` 1, `Steps` 8, `Denoise` 1.5, `Footprint` 1, `Temporal` 0.1, `TemporalClip` 1.25, `Jitter` 1, `Slices` 40, `MinStep` 6, `NoiseSize` 128, `Obscure` 0.5, `Samples` 2, `KeyShare` 0.8, `Scatter` 0.2, `Bake` 1, `VolumeSize` 256, `VolumeHeight` 160, `VolumeTexel` 3, `Shadows` 1, `ShadowSoftness` 0.02; light inside the gas, below: `Lit` 1, `LitStrength` 1, `LitTint` 0.5, `LitExplosion` 1.5, `LitExplosionCore` 2, `LitExplosionFar` 0.12, `LitBolt` 3.5, `LitBeam` 1.5, `LitSpark` 0.3, `LitCorona` 0.25, `LitKnee` 6, `LitTemporal` 0.5, `LitClip` 0.5), read again
every second with `[World] Reload=1`; `[World] Enable=0` hands everything back to the
engine at once, for comparing with stock at the same moment. `BesidePlugins=1` (bench
only) draws even with an old plugin loaded, so `Enable` flips between theirs and ours.

**On screen.** Every object the module draws in the engine's place (planets, moons,
asteroid fields, nebulae) gets the byte `Entity::Render` (0x4cfda0) writes as the
engine draws it: `+0x25`, set when the instance's frustum test (its vtable slot 3)
passes, cleared for every object each frame by `Entity::ResetVisibility`. The
pointer's pick (`DisplayInterface::s_PrecomputeMouseOver`) passes over any object with
it clear: without it a planet could be selected only with a drag box and a colony ship
could not be right-clicked onto it (the hulls module writes it too).

**Surfaces** (`surfaces.rs`, `world_surface.hlsl`): `Surfaces.asi`'s bake, unchanged,
into three cubes per recipe (albedo, relief, clouds) that are render targets in the
default pool with device-made levels; a `Reset` drops them and the next draw bakes
again. Asked for by texture name; a name without a recipe keeps its stock texture.

**Planets** (`planets.rs`, `world_planet.hlsl`), objects with the `Planet` vtable
(0x6b2b3c) and a `Planet_Database` (vtable 0x6bab74, the instance's +0x80). Stock draws
two `GroundMesh` hemispheres (database +0xa8, +0xac) and the atmosphere's cloud shell
(+0xb0), radius at each mesh's +0x150, re-tessellated against the camera every frame;
we draw one sphere of our own whose vertices carry the gore UVs `GroundMesh::mSetUVs`
gives (the class textures are a four-lobe unwrap), so stock's art lands where it did.
`PlanetInstance::RenderInternal` draws only while the planet's flags (+0x14) have bit 2
and its byte +0x113 is clear. The ground is the class's `groundTextureName`
(`PlanetClass+0x4bc`) with 1 and 2 appended, or its recipe; the shell the
`atmosphereTextureName` (+0x4ac), turned about the axis by minus the instance's +0xb4
times the database's +0xa4, its turbulence on the instance's +0xb8 while the class's
+0x4cc is positive, tinted (`mSetupAtmosphere`) from white towards the race's colour
(race +0x464) as the population (+0x2c8) nears the class's maximum (its level at
+0x490 indexes the four `cfgPOP_*` at 0x702130). The look is `Lighting.asi`'s
`planet.hlsl` (night side, dusk, the limb's haze, the air past the limb, the glint off
water, cities at night), less its highlight shoulder, which the tone map does. The
shell writes no depth. **Cities** (pass 1 of `ST3D_PlanetaryMaterial`): the race's
(`Planet+0x2c4`) development texture `PD_<cityTextureName><1|2>` (race +0x452) times
the planet's own map of where its people live, made again as `PlanetInstance::
mSetupHemisphere` makes it from the race's `CityAllocArray` for that hemisphere
(+0x514, +0x518: width, height, floats): each texel's alpha 0.4 x clamp((P - value) k),
k the race's +0x470 and P the population over `cfgPOP_HEAVY` (0x70213c, set from
`RTS_CFG.h` at load; the 25000 in the exe's data is not what runs) plus 1 / k. On the
bench a full colony's map was 0x66 everywhere, the engine's and ours.

**Rocks** (`rocks.rs`, `world_rock.hlsl`). An asteroid field (vtable 0x6b0cb4) holds
its rocks as `ST3D_Instance`s (instance +0x98, count +0x9c, 0x98 bytes each: transform
+0x44, database +0x80), the class's databases at +0x1e0 named by the table at 0x6b07e8
(`master`, `master1`..`master5`, `master4` again, `master7`); each is culled as
`GameObjectInstance::DistanceCull` culls it (height counted a quarter, against
`cfgOBJECT_CULLING_DISTANCE`, 0x6fcac0) and drawn at the field's `ScaleSOD`. A `Scrap`
(vtable 0x6b15b4) is ours only when a mesh of it has a recipe (the dilithium moons'
`Mdmoon`); any other is the hulls module's. A mesh with a recipe is drawn as
`Lighting.asi`'s `moon_ps` drew it (relief, the recipe's cut-out as holes, triplanar
for the asteroids), about its mesh's centre scaled to its extents. The moon's inner
shell (`glowdmoon` material, whose colour `Scrap::Render` sets to (0, 0.5, 1) times the
share of dilithium left, `ResourceInterface::GetTotalValue` over the class's
`GetTotalMaxValue`, resource at +0x17c) is drawn dark and opaque as under `Lighting.asi`
(`MoonShell=dark`): the settled look, black cracks with the glow showing only where a
crack opens past the rock's edge; `MoonShell=glow` adds it in its colour, as stock
did. That shell only shows with `MoonCracks=0`: by default the cracks are not holes but
the moon's light (`ps_rock`, c35), so it glows from inside and through its cracks only.
The light is the moon's sprite node's animated colour (`oreglow`'s `moonpulse`, (0, 0.5,
1) down to 0.4 of it every 2 s; `dmooninf`'s `infpulse`, purple, on the infinite moons)
at its brightest key, swung by `MoonPulse` of the pulse, times the share of dilithium
left (the shell's colour over its full colour) and `MoonCracks`, past 1 so the bloom
spreads it along the crack. It whitens towards the fissure's core (the relief cube's
alpha past `Cut=`, times `MoonCrackCore`), and the recipe's `Glow=` beside the cracks
takes its hue and pulse at the recipe's strength. The light lies at the bottom of each
crack, not on its surface: a crack is a ravine `MoonCrackDepth` deep (in the mesh's
radii), opening where the cut does and deepest at the fissure's core (the relief alpha
at twice `Cut=`), and the view's ray is marched down into it (eight fixed steps along
the radial lookup, branch-free) to the wall it meets. How deep that is sets the light,
to the power `MoonCrackFalloff`, so the walls are dark rock, the sun only on their top,
and seen at an angle the near wall hides the light. Opening the ravine wider than the
cut was tried and dropped: the finer net of branches peaks just under `Cut=` and opened
as thin blue threads. A crack only a pixel or two wide has no room for walls (they broke
it into dots at play distance), so the ravine fades in as the depth spans 2 to 5 pixels
and a far moon keeps the flat look. The nimbus itself, a round billboard
behind the moon that showed only past its edge as a halo, is drawn only with
`MoonNimbus=1`, at `Step::Translucent`, its animation on the instance's clock.

**Nebulae** (`nebulae.rs`, `world_nebula.hlsl`): `Nebulae.asi`, its decisions kept
(the envelope, the bake, the slices on a ladder fixed in distance from the
eye, per view, the blue-noise thresholds that stay put, the spectral noise). A class
with a recipe draws as gas at `Step::Translucent`, its billboards skipped in the object
loop; any other is left to the particles module. Every nebula the loop reaches is
marked on screen (+0x25) as `Entity::Render` marks it when the instance's frustum test
(slot 3) passes: `Nebula::Simulate` registers the nebula's light only while it is.
The Key is the main view's (the action camera's view keeps the bake), the ships'
shadows the hulls module's map (`hulls::shadow_map()`, sampler 8). A ghosted nebula of
a gas class keeps its gas: slot 3 of the `GameObjectSpirit` vtable (0x6b1ca4) is
wrapped. Everything is in the default pool; a `Reset` bakes again. A class is baked
again only when its nebulae, the Key or its recipe change (the log says which); until
render3d 0.22.1 every class was baked twice on its first two frames, since a class made
in the middle of a gather started its signature from 0 and every later frame from
FNV's offset basis (`SIG0`). On Vulkan the bake's read-back is from host-cached memory
(`gpu/README.md`, "Read-backs"): from the card's memory it took a second a class.
The gas is **marched** at full size, not sliced. Each class draws its gas box once
(the faces facing the eye, or those facing away when the eye is in or near it), and each
pixel finds its own ray (c41..c43: the ray at view depth 1 is linear in the pixel), cuts
it to the box and to the scene's depth (a RESZ copy into INTZ, read as a texture; with
this projection `z - _33` is below 0), and takes `Steps=` steps (8, 12, 16 or 24, an
entry point each, since the loop is unrolled by its count) spaced in proportion to the distance,
each jittered in its span by the pixel's blue-noise threshold walked by the golden
ratio. Its sums (light, optical depth) are added into a half-float target; `comp_ps`
filters them with a 5x5 Gaussian of `Denoise=` pixels (sigma) that leaves out pixels
whose scene depth differs by over 5%, so hull edges stay a pixel sharp, and applies
`Opacity=`/`Obscure=` onto the frame. The slices (`vol_ps`) remain only without INTZ
and for `Bake=0`. The Direct3D 9 route (vkd3d, until 1.0.0) had no `tex3Dlod`, and a read's level from the hardware's
derivatives is undefined inside a branch that differs per pixel, so the march reads the
noise with `tex3Dgrad` (the pixel's footprint at the step's distance, times
`Footprint=`) and can then skip rays that miss the gas, columns the occupancy record
says are empty at that height, and points the volume says are clear. The flow, a slow
field, is read every fourth step and drawn straight in between.

Measured on the bench (`nebula` scene, 1920x1080, RX 5700, the camera inside a cloud
about 1700 x 1000 units, so every ray crosses a long path of gas), with the gas split
out by `Timing=1`'s marks:

| | march | filter, composite | total | grain |
|---|---|---|---|---|
| `Nebulae.asi`'s half-size slices, 40 x 2 reads | | | 3.39 ms | 0.65 |
| full-size slices, 40 | | | 8.16 ms | 1.21 |
| march, 8 steps | 4.79 ms | 0.81 ms | 5.6 ms | 0.68 |
| march, 12 steps | 6.05 ms | 0.77 ms | 6.9 ms | 0.61 |
| march, 16 steps | 7.12 ms | 0.72 ms | 7.9 ms | 0.59 |

Grain is the mean |shot - its 1.5-pixel blur| over the middle of the frame, in 8-bit
steps. Unfiltered, the march's grain goes as one over the root of the steps (24 steps
3.1, 12 steps 4.6): a few samples of a long path, not a lattice. The half-size path's
tent (about 10 screen pixels wide) had hidden the same noise; the 1.5-pixel filter takes
it out while the gas's own structure stays, since the volume's texels (6.6 units here)
are several pixels wide. Per sample the march costs about what the slices did; what it
saves is the blending of every slice, and what it spends is four times the pixels.
About 2.5 ms of the march does not depend on the step count (8 to 16 steps adds 0.29 ms
a step). It is not the unrolled shader's size: Microsoft's compiler (`d3dcompiler_43`,
run under Wine for the measurement only, never part of the build) makes the 8-step march
a real `rep` loop of 2,200 dwords instead of vkd3d's 10,400, and on the bench (720p) the
march took 2.53 ms either way against vkd3d's 2.52, measured when the bytecode was still vkd3d's: DXVK's SPIR-V and the driver end
up at the same code.

**`Temporal=`** (default 0.1) adds the frames up, since in game 8 steps' stipple still
showed through the filter. The thresholds then walk the golden ratio from frame to frame
(`k_ray2.w`, the frame), so each frame's samples fall between the last's, and
`temporal_ps` blends this frame's march into the view's history, a share `Temporal=` of
it. A pixel's gas is found in the history where it was the frame before: the march
writes a second target (MRT), the view depth of each step weighted by its optical depth,
and that point goes through the last frame's VIEW x PROJECTION. History the present
frame's 3x3 neighbourhood does not support (gas a hull now covers, gas that moved) is
clipped to its mean +- `TemporalClip=` (1.25) spreads; a value the half floats cannot
hold, or a NaN, is replaced by the mean so it cannot stay. The clip alone let a ship
that moved over the gas trail its outline in game (eight steps' noise makes the
neighbourhood's spread wide), so history is also dropped wherever the scene's view depth
differs by over 5% from what it was there (the march stopped at another hull): beside
the history a second target keeps that depth and how many frames the history holds,
and a pixel takes each frame at 1 / (frames + 1) until that reaches `Temporal=`, so a
dropped pixel is clean again within a few frames. In thin gas (the Big Cerulean
Nebula's specks) a frame's eight steps find gas in few pixels, and a 3x3 that found
none has a mean and a spread of zero, which wiped the history whatever
`TemporalClip=` was (1.25, 2.5 and 4 measured the same) and left the stipple there:
the clip's top also reaches the brightest pixel of the ring two pixels out, which
smoothed those wisps on the bench. Zoomed out, a grain stayed in game that zooming in
smoothed away: the volume's texels and the swirl's folds were then finer than a pixel,
and a ray through the pixel's centre alone gave each pixel a value of its own that every
frame repeated, so no number of frames averaged it out. So the ray also walks within its
pixel from frame to frame (`Jitter=`, its span in pixels, default 1; an R2 sequence on
the frame, `k_ray1.w`), as TAA's camera jitter does, and the frames add up the gas over
the whole pixel. It costs nothing in the march. On the bench (720p, high above the Big
Cerulean Nebula) the wisps came out finer and less blotchy than with `Jitter=0`, whose
grain number was the same (0.94 / 0.95; `Footprint=2` 0.94): the metric mostly measures
the gas's own structure there. The history is four targets
per view (the action camera keeps its own), dropped with the device and whenever no gas
was marched. `comp_ps` then filters and composites the history as it did the march.
The pass costs 0.22 ms at 720p. With `Temporal=0` the thresholds stay put, as before:
moved without adding the frames up, they made the gas shimmer. Shadows cost 0.6 ms of 7.6 at 24 steps; the noise reads (flow, fine
detail) were half of it before the flow was thinned. `Footprint=2` saved nothing.

**Light inside the gas** (`[Nebulae] Lit=1`, Vulkan and Direct3D 12, with the clustered
lights: "Clustered lights" under hulls). A breach lights the cloud about it, and
torpedoes, pulses and phaser beams light the vapour they pass through. The march takes
a twin (`march*_lit_ps`, dxc's alone) that, at every step with gas, walks the step's
cluster (the pixel's tile, the step's slice of view depth: the clusters' own mapping,
bound with `clusters::bind`, so the march's pass needs nothing of its own) and adds
the light the gas scatters there: density times step times `LitStrength=` / 100 per
world unit, the light's colour tinted `LitTint=` of the way towards the gas's hue; no
phase (the gas glows alike from every side) and no shadowing by the gas between. Of
each colour group only the strongest light at the step counts, as on the plating, and
the cull's `ClusterPerColour` holds, so a volley of one colour does not blow out. The
lights' light is added up along the ray and joins the gas's own at its end through a
soft knee, L / (1 + L / `LitKnee=`) on its luminance: a light or two in thin gas is as
it was, and a battleship's breach, whose fireball reaches across half the view, rolls
off instead of whiting the frame out (at the knee's peak, 0.35 s after the kill in
`render3d-nebula-lit`, 6 kept the glow's gradient where 0.75 to 2.5 flattened it into
a uniform wash). The composite takes it through the gas's opacity, and the gas with
no lights is drawn as before. Which light is what, to the gas
(`scene::lights::Gas`, the cluster list's `t.z`): a fireball's takes `LitExplosion=`
in a **core** about the breach and a dim **wash** beyond it. Its light is full out to
the dead craft's bounding radius r (at least 40) and gone at `[Explosions]
LightRange=` radii (10), linearly: as a plain light in the gas that lit everything
within some ten radii almost alike, and a view ray inside the cloud crosses hundreds
of units of that gas, so the sum along every ray in view went up to the knee and the
whole cloud went white to the frame's edge (the user, in game, 2026-10-23: a breach
*inside* the gas "out of proportion"; one just outside it looked right). So the gas
takes that light times wash + (1 - wash) / sqrt(1 + (d / (c r))^4), c =
`LitExplosionCore=` (2) and wash = `LitExplosionFar=` (0.12): about full within c
radii, falling as the inverse square past them, down to the wash (`LitExplosionFar=1`
is the plain light as before), and `LitExplosion=` went from 0.75 to 1.5, so the core
is hotter than before while the cloud beyond takes about a quarter of what it did (an
install moves a player's 0.75 to 1.5: `install.sh`, `RETIRED`). Judged on the bench
(`render3d-nebula-explosion`, Vulkan on the GPU, 1280x720, 2026-10-23) against a core
of 1.5 to 4 radii, a wash of 0.1 to 0.3 and `LitExplosion=` 0.75 to 2: a wider core or
a stronger wash brought the pink over the whole cloud back. A falloff by distance
cannot tell apart the two cases the user judged: gas a few hundred units from a breach
just outside the cloud is as far from it as the far gas is from a breach inside, so
the cloud's edge beside a breach outside now glows less than it did (pink, not
salmon), while gas within two radii of a breach glows as before or brighter. The hulls
take the fireball's light as before;
a torpedo's or pulse's (each ordnance class's light, which rides with its shots) is a
**corona**, bright within `LitCorona=` of its reach and fading softly to it,
(1 - x)^1.5 (0.45 + 0.55 / (1 + (x / LitCorona)^2)) for x the distance over the reach,
since a ball lit evenly to its edge read as a sphere in the gas. A **spark** at a hull
gets the same corona, at `LitSpark=` (0.3) of its light: the repair and construction
welds, the bees' welds, the shields-down discharges and the damaged sensors' glow
(the `spark` medium, `scene::media::SPARK`; `Pool::register_in`, or
`scene::add_point(.., medium)` for a point added by hand). Taken as plain lights, the shields-down discharges lit a ball of
gas 1.2 bounding radii wide about the strike they were at, blue. In the Mutara, which
takes every ship's shields down, the user saw these as big round blue blobs beside
every Borg ship (2026-10-23). A/B on the bench with `[Shields] DownLight=0` put the
blobs on the discharges, not the welds or the hits. A beam's line light takes
`LitBeam=`; the nebulae's own lights (`world::lights`) and a planet's day side
light no gas at all. The shape is the clusters' first choice (the alternative, a froxel
volume filled by compute and integrated, would spend a pass and a 3D storage image on
what the march, which already walks the depth, reads for one buffer word per step
where no light reaches).

Lights move and the gas adds its frames up (`Temporal=`), so a torpedo would trail
its glow through the history. The march writes the luminance of the lights' share
into its second target's green; `temporal_ps`'s twin weighs it against the gas's own
over the 3x3 and keeps that share in the history's meta (blue), and where either is
large (now, or where a light just was) the clip narrows to `LitClip=` of
`TemporalClip=` and this frame's share rises to `LitTemporal=`, so the glow follows its
light within a frame or two. The pass with `Lit=0`, or where no
clusters run, is the plain one (the same shader; the port scenes as before).

Cost (`Timing=1`, `render3d-nebula-lit`'s paused frame: the `nebula-battle` scene, 32
lights in the view, the camera in the gas; RX 5700, 2026-10-23), the march's GPU time
without and with `Lit=1`: at 1280x720 2.63 against 2.70 ms on Vulkan and 2.61 against
2.69 ms on Direct3D 12; at 3440x1440 6.76 against 7.16 ms on Vulkan and 7.06 against
7.20 ms on Direct3D 12. Measured, this settled the design: the march already walks
the depth through the clusters' own mapping, so a step where no light reaches costs a
buffer word, and a froxel volume (a compute fill of some 215x90x64 cells and a pass to
integrate them, about the cost of the lit march's whole addition before any light) had
nothing left to win. Checked: the plain march with `Lit=0` at RMSE 0 against the
clustered lights' commit on Vulkan and Direct3D 12 (`port-nebula`, all 25 frames of
its dump), and (when Direct3D 9 still ran) at RMSE 0 against `pre-gpu-nebula`; on lavapipe the
validation layer finds nothing but `Timing=1`'s query messages, which the scene
without the gas gives too.

**Lights** (`lights.rs`), without `Lighting.asi`. *Planet glows*: per planet and view
(`Module::lights`), a point light at its centre (the instance's transform +0x44 plus
its bounding sphere +0x34) in the Key's colour times its ground's mean colour times
`PlanetGlow`, full at the surface and gone at `PlanetGlowRange` radii, marked a day
side (`scene::lights::FLAG_SHARE`, `is_day_side`): a draw takes it times the share of the day side
facing it (`Lights::share`), and the planets take none. *Nebula lights*: the call to
`Simulate_Nebula_Lights` (0x4a5160) at 0x4a4a7a in `Nebula::Simulate` is re-pointed;
stock's colour (the class's glow, +0x22c, swung by a noise term) came out black, so
the light is the glow times `NebulaBrightness`, steady, in a gas class's recipe hue,
its falloff (class +0x244, +0x248) times `NebulaRange` (written on the nebula's light
object, +0x1ac). *Nebula cull*: slot 3 of the `NebulaInstance` vtable (0x6b1448,
`ST3D_Instance::FrustumTest` 0x62e990) tests the bounding sphere (the node's at +0x1c
when the instance has one at +0x80, else its own at +0x34; radius +0xc) with its radius
times `NebulaCull`, and puts it back.

**Recipes.** Three folders, one per kind, all plain `.ini` files you can edit.

- `content/sky/<name>.ini`: the sky recipes, built into the plugin ("Sky recipes" above).
- `surfaces/<texture>.ini`: planets (classes D, H, J, K, L, M), the dilithium moon
  `Mdmoon` and the asteroids `Master1`/`Master2`. Installed by `install.sh` into the
  game's `Surfaces\`; a name without a file keeps its stock texture. A recipe for a hull
  texture is found at that texture's first draw only, so adding one needs a restart.
- `nebulae/<odf>.ini`: the gas classes (`mnebula6`..`10`, `mnebula12`, `latinum`,
  `fluidicnebula`, `generatednebula`). Installed into the game's `Nebulae\`; a class
  without one keeps its billboards.

Edited copies in the game directory are re-read within half a second with `Reload=1`,
and a changed recipe bakes again. Each file's header gives what it aims at.

*Surface recipes* (`[Surface]`; colours 0-255, heights in field units, about -4 to 5;
directions are yaw about the planet's +y and pitch up from its equator):

| Key | Default | |
|---|---|---|
| `Kind` | `rock` | `gas` for a gas giant |
| `Seed`, `Scale`, `Warp`, `Detail` | 1, 2.5, 0.4, 0.5 | as for the sky; high `Warp` turns the ground wormy |
| `Continents`, `Ridges` | 0 | the slow field that lifts and sinks regions; mountain chains on land |
| `Sea`, `Shore` | none, 1 | the sea level (left out: a dry world); how deep the water must be to reach `Deep` |
| `Relief` | 0.02 | the height's scale against the radius per field unit: the normal's tilt |
| `Craters` | 0 12 1 0.3 | share of cells with one; cells across; depth; rim |
| `Fissures` | 0 0.05 1 1 | cells across (0: none); trench width; depth; glow. `FissureShape` (2.5 0.33): the warp, and the share of the net's borders kept; `Branches` (0.6): the finer net |
| `Cut` | 0 | the glow at which an alpha-tested model is cut through (the moon's cracks are holes) |
| `Ramp1`..`Ramp6`, `Alt1`..`Alt6` | | `height r g b` (gas: `0..1 r g b` across one band); `Alt` is a second ramp mixed in by region (`Region`: scale, share, colour noise, its scale) |
| `Deep`, `Shallow` | | the sea; a gas giant's poles are `Deep` |
| `Ice`, `IceColour` | | latitude it starts at (0..1), softness, ragged edge, amount; colour |
| `Cavity` | 0.25 0.15 | hollows darkened, crests lightened |
| `Glow`, `GlowDay` | 0, 0.3 | the fissures' light, and how much of it shows by day |
| `Clouds` | none | scale, coverage (higher is clearer), softness, warp; `CloudShape`, `CloudColour`, `CloudOpacity`, `CloudDrift` (degrees per second), `CloudShadow` (strength, reach in radians) |
| `Depth` | 1 | the relief's strength in the light, at draw time (no re-bake) |
| `Bands`, `Storm`, `StormColour` | | gas giants: bands, turbulence, warp share, stretch; the storm's yaw, pitch, size, strength |
| `Face` | `[Surfaces]`'s | this recipe's cube edge (64-4096) |
| `Mapping` | `radial` | a model's lookup: `radial`, the direction from its mesh's centre; `triplanar`, each axis's cube faces across its extents. The asteroids are `triplanar`: the direction from the centre smeared their flat faces into streaks |
| `Gloss` | 1 | a model's highlight, times the hulls' |

*Nebula recipes* (`[Nebula]`; `Cls::read` in `nebulae.rs` is the authority for defaults).
Shape: `Extent`, `Height`, `Lumps`, `EdgeWarp`, `Radius`. Noise: `Seed`, `Tile`, `Warp`,
`WarpScale`, `Detail`, `FineDetail`, `Coverage`, `Softness`, `Gamma`, `Ridge`. Colour:
`Brightness`, `GasA`, `GasB`, `Glow`, `HueScale`, `HueMix`, `HueCycle`, `HueSwing`.
Light: `Light`, `ShadeStep`, `Shading`, `Knots`, `KnotScale`, `Lanes`, `LaneScale`.
Motion: `Flow`, `FlowScale`, `FlowSpeed`, `Swirl`, `SwirlScale`, `SwirlPeriod`. Edges:
`Fringe`, `FringeReach`, `FringeScale`, `Fuzz`, `FuzzSpeed`. Filaments: `Filaments`,
`FilamentScale`, `FilamentSharpness`, `FilamentReach`. Viewing: `Opacity`, `EdgeOnDim`,
`KeyShare`, `Scatter`, `NearFade`. Lightning: `Lightning` (flashes a minute, a nebula),
`LightningSize`, `LightningBrightness`, `LightningColour`. The core: `Core`,
`CoreSharpness`, `Pulse`, `PulseSpeed`, `PulseWander`, and its ring of colours `Ring`,
`RingAt`, `RingWidth`, `RingScale`, `RingPatch`, `RingSoftness`, `RingSpread`,
`RingCycle`, `RingA`..`RingD`. A shadow takes only the Key's share of the gas's own
light (`KeyShare`), scattered forward (`Scatter`); dimming self-lit gas by a fraction
left the shafts invisible except in a difference image, and they show looking towards
the Key.

#### world: Hooks

`mods/world/src/hooks.rs` holds every address, call site, vtable slot and field offset
of the module, behind named accessors, and its three detours. They install through
`r3d_core::hooks` at the module's first switch-on (not at the first frame): at load for a
module that is on, at the first live switch-on for one that was off. Each detour starts
with the module's gate and hands the call to the `Prev` it replaced while the gate is
shut, so a module switched off live puts the engine's code back in charge.

| Hook | Site | Used for | Gate shut |
|---|---|---|---|
| `Nebula::Simulate_Nebula_Lights` | the call inside `Nebula::Simulate` | the nebulae's lights: the class's glow times `NebulaBrightness`, steady, in the recipe's hue, the falloff times `NebulaRange`, lit hulls only (`own_glow`) | the engine's own call (it registered a colour that came out black frame after frame) |
| `NebulaInstance::FrustumTest` | slot 3 of the nebula instance's vtable | a nebula is on screen while its sphere times `NebulaCull` is, so a nebula beside a zoomed-in ship keeps its light | stock's test, the sphere unscaled |
| `GameObjectSpirit::Render` | slot 3 of the ghost spirit's vtable | a fogged nebula whose class is drawn as gas draws no billboards | the engine's `Render` |

The first two are ours unless `Lighting.asi` is loaded (it has the lights); the third
unless `Nebulae.asi` draws the gas (`BesidePlugins=1`, bench only, installs them beside the
plugins). The planets, moons and asteroids are not hooked: they are drawn in
`GameObject::Render`'s place through the frame's object loop, so while the module is off
the engine's `Render` runs. A planet's glow light is an ordinary point light registered
in `lights()`.

The module reads its parts again at every settings change (`configure`): the rows
`World.Planets`, `Rocks`, `Nebulae` and `Surfaces`, `Nebulae.Lit`, the nebulae's keys, the
planets' and rocks' looks, the light keys. A part is ours when its row is on and no old
plugin has it; `off()` stops the ghost hiding and the planets' glows, and the next view
lets go of the GPU objects.

#### Content sets

A content set is a folder, `render3d/content/sets/<set>/`, with one small ini table per
module (`effects.ini`, `hulls.ini`, `ordnance.ini`, `overlays.ini`, `particles.ini`,
`sky.ini`, `specials.ini`, `world.ini`): the game's names each module matches or draws
(nodes, sprites, SODs, faction keys and colours, the kinds of special effects) and the
recipes it builds in, named by their file under `render3d/content/` (`mbg02 =
sky/mbg02.ini`; a set's own recipes can sit in its folder). `core/build.rs` builds every
`.ini` under `render3d/content/` into the core (`include_str!`, `r3d_core::content::file`);
a module's `content.rs` holds only the `Content` type, its `parse` (through
`a2ini::Table`, which borrows from the built-in text) and an empty `NONE`, and
`content::get()` parses, once, the table of the set `profiles/<build>/content.toml` gives
this build (`Engine::content()`, `crates/a2engine/README.md`, "Content";
`r3d_core::content::Cache`). So `by_set` is a folder lookup: a set with no table for a
module is the empty table, and a new set (Fleet Ops) is a copy of `sets/stock/` named in
its `content.toml`, with no code change (MODDING.md, "Content sets").

**The `fleetops` set** (Fleet Operations: Roots, FO-5 step 6; `profiles/fo-roots-5.0Z11/content.toml`
names it for all of render3d; until then every module but the hulls read the stock set,
inherited from gog-1.1's `content.toml`). Names and keys only: every look in it is the
stock set's, never one fitted to Roots' art (PLAN-FLEETOPS.md, decision 7). Two tables
say how Roots' names map onto stock's looks, generic keys any set may use:

- `ordnance.ini` `[Names]`: `Ordnance.<odf> = <recipe>` (and `Wave.<odf>`), which
  ordnance ODF each built-in recipe draws. With it a built-in recipe draws only the ODFs
  named, because a stock name Roots shares may be another weapon there: Roots' `ionstm` is
  a utritium beam (`utribeam`), stock's the Klingon ion storm torpedo, so it is left out
  and drawn as the engine draws it. Each pair is a Roots ordnance ODF (a weapon's
  `ordName` in `ODF.fpq`) with the recipe of the stock weapon of the same `classLabel` and
  use: the construction ships' tractor beam (`npc_tractorbeamo`, `tracbmo`), the Borg
  holding beams, Species 8472's Clairvoyance, the Cardassian Energy Drain, stock's own
  holo-generator, shield inverter and computer override, the miners' dilithium and
  tritanium beams, the repair beam, two waves. A section only `Render3D.ini` has draws the
  ODF of its name, as on stock. The ordnance module's first line for each ODF
  (`ordnance: <odf> (vtable ..., Submit ...): Recipe(n)`, `Stock`, `Model` or `Engine`) is
  the survey to extend the table from; `hosttest` checks every named ODF against
  `ODF.fpq`'s `classLabel` where Roots is installed.
- `effects.ini` `[Races]`: `<race> = <faction key>`, a ship's shield faction by its ODF's
  `race` (through its includes, `assets::race`), not by its name's first letter: Roots'
  Dominion ships start with `d` and `n` and its Free Borg with `f`. Races it does not
  name (the Dominion, the Ferengi, the neutral `npc_*` races) take the last faction.

The rest: `world.ini` names Roots' nebula classes (`map_nebula_*`), each with the stock
recipe of the stock nebula of the same `nebulaType` and glow hue (Mutara, Nucleon as
metreon, Crystalid, Fluidic and Organic as metaphasic, Fluid as cerulean; Tetryon has no
match), no surface recipe (Roots' planets are its own art), and not `latinum`, which in
Roots is the class of its asteroid sets and debris fields. `particles.ini` names no vent
sprites (Roots' venting emitters draw `empty`) and keeps stock's chain-pulsar and Manheim
emitters, which Roots keeps; `sky.ini` names nothing (Roots' starfields are its own
art); `overlays.ini`, `specials.ini` and `hulls.ini` are stock's names, which Roots keeps
(`hulls.ini` with the race-keyed profile, FO-5 step 4).
`hosttest/src/sets.rs` holds the stock tables to what the modules held as constants
before (render3d 1.2.0). A build with no set gets `NONE`: no faction keys (shields colour by stock's level), no node or sprite
names (a clean model shows everything; no vent or pulsar emitter), no recipes (no special
weapon is told; skies, surfaces and nebulae are the engine's own), no selection model. The
modules register their set's recipes with `r3d_core::recipes::builtin`, and the game
directory's files still win. Each name is tied to its user in the module's section.

#### world: Content

`world.ini` names the built-in recipes (files of `render3d/content/surfaces/` and
`render3d/content/nebulae/`, by lower-cased name) and the moons' glow material. A recipe is
read through `r3d_core::recipes`: the game directory's `Surfaces\<texture>.ini` or
`Nebulae\<odf>.ini` first (`install.sh` copies the same files there, where an edited copy
is read again while the game runs), else the one built in, so the look needs nothing
installed.

#### world: Settings

`[World]` and `[Nebulae]`, as above, read by `configure` and, with `[World] Reload=1`,
again every second for the keys without a row. The rows `Modules.world`,
`World.Planets`, `World.Rocks`, `World.Nebulae`, `World.Surfaces` and `Nebulae.Lit` are live
(`render3d-live-world.md`).
