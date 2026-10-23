# render3d — the 3D view through our own renderer

`Render3D.asi` takes over the engine's 3D view: both calls of `Armada_RenderAllOurStuff`
(0x598110) come to it, so the scene is drawn into render3d's own targets and put into
the back buffer by render3d. It is written in `no_std` Rust (`src/`), and the plan, its
milestones and every change to it are in [`PLAN.md`](PLAN.md).

What it does today:

- **The frame is ours.** The view is drawn into a half-float (`A16B16G16R16F`) target
  with 4x MSAA and a depth of its own, then resolved, bloomed and tone-mapped into the
  engine's back buffer at the view's rectangle. Additive light (explosions, weapons,
  shield hits, glows) now sums past white instead of clipping, and what lies past
  white glows by how bright it is: bloom by energy, which an 8-bit back buffer cannot
  hold. Below the tone map's shoulder every colour is stock's.
- **The body is ours**: `Armada_RenderAllOurStuff`'s steps run in render3d's own
  sequence (`frame/steps.rs`), and seven modules (`## Modules` below) draw every
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
  loaded (`SUPERSEDED` in `src/lib.rs`), and `install.sh` takes any such copy out.
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

`./install` runs it. Building needs rustup with the 32-bit Windows
target (`rustup target add i686-pc-windows-msvc`) and `lld-link` (LLVM); `build.sh`
uses `~/.cargo/bin/cargo` first, since a distribution's cargo has no such target. The
crate links with `/nodefaultlib`, `kernel32` its only import (an import library made by
`llvm-dlltool --kill-at` from `kernel32.def` in `build.rs`), and brings its own
allocator, memory functions and stack probe (`src/rt/`). Shaders are HLSL in
`shaders/`, compiled by `hlsl-rs.sh` with `vkd3d-compiler` into `src/shaders/<name>.rs`
and with `dxc` into SPIR-V in `src/shaders/<name>_spv.rs` ("One source, two
compilers"), both committed with a stamp so a machine without the compilers builds as
long as they are current.

### Shaders on Windows

**Windows' own Direct3D 9 refuses vkd3d's bytecode; DXVK takes it as it is.** On
Windows the chain is crosire's d3d8to9 over the system's d3d9, with no DXVK, and the
runtime validates every shader at `CreateVertexShader`/`CreatePixelShader`. Measured
on Windows 10 (Radeon RX 5700 XT, a 32-bit process): 51 of the 86 shaders of
Render3D.asi and UI.asi came back `D3DERR_INVALIDCALL`, on every kind of device
(software, hardware, mixed and pure vertex processing). The state block and the vertex
declarations were made. So both plugins stood down: UI.asi drew nothing (a black
screen over working menus) and render3d left the 3D view to the engine.

Taking the refused shaders apart instruction by instruction (each prefix of a shader,
closed off with a write of `oC0`) found what the runtime refuses:

1. **One constant register and one input register per instruction.** `mul r0.x, c0.x,
   c12.x` and `mul r1.x, v0.x, v2.y` are refused; the same register twice is
   accepted. vkd3d writes both freely.
2. **`sincos` may not write the register it reads**: `sincos r1.y, r1.x` is refused,
   `sincos r2.y, r1.x` accepted.
3. **`oDepth` takes the full write mask**: `mov oDepth.x, r0.x` is refused.
4. **No `texld` (or `dsx`, `dsy`) inside an `if`.** The same `texld` on its own is
   accepted, on a 2D, cube or volume sampler alike, and so is any prefix that stops
   before the first one inside a branch.
5. **r0 to r31 only.** The nebula march shaders use r33.

These five predict the runtime's verdict on all 86 on that card. A sixth depends on the
card:

6. **No more instruction slots than the card takes** (`D3DCAPS9`'s
   `MaxPixelShader30InstructionSlots`). The Radeon above takes 32768, the most shader
   model 3 allows, so it took everything; other cards take 4096. A player's report
   (Windows 11, 2026-10-20) had the main menu's sky black and its planet stock: every
   shader up to 3659 slots was made and every one from 5861 refused. That is the
   sky's bake (8761), the planets' height (17812) and the nebulae's march (5861 at
   `Steps=8`), the last of which fell back to slices without a word. The bakes'
   noise is unrolled by vkd3d (a value-noise lookup is 143 slots, a lattice of 27
   craters 3891), and shortening it would change what it draws.

So **a bake longer than 4096 slots is a chain of passes** (`gpu/src/chain.rs`,
`shaders/chain.hlsli`): each pass works out part of the function at its texel and
hands it to the next through a float32 target. The sky is the warp, the field, the
gas, two slabs of the star lattice, and the last slab put together. A planet's height
is the warp, the ground, the mountains with the first slab of craters, two more crater
passes and the two fissure nets (a gas giant is the warp and itself). The values are
full floats, so it bakes what the one shader did. The targets are tiles of at most
1024 on a side, so the sky's 3072 supersampled face needs 48 MB of them, not 450.
`d3dbc-fix.py` counts slots as the runtime does and refuses a shader past 4096 unless
its entry is marked `:long`. Two are marked: `sky_ps`, the `Face=0` reference, and the
march, which falls back to slices; cutting the march per frame is still to do.
`Render3D.log` names every shader the runtime refuses, with its slots and the card's. `d3dbc-fix.py` mends four of them
in the bytecode. It copies the extra register into a temporary (only the lanes the
instruction consumes, so an input is never read outside its `dcl` mask), routes
`sincos` through one, and widens the mask. And it gives the temporaries out again when
a shader names more than 32: vkd3d never reuses a register, but at most 19 values are
live at once in the march (liveness over its if/else; a value live into or out of an if
keeps one register across all of it). `hlsl-rs.sh --fix` runs it on each shader, for
both plugins (`build.sh`, `ui/build.sh`); `d3dbc-fix.py --check <file.bin>` lists all
five.

Rule 4 is the HLSL's to keep. **Never sample with an implicit level (`Sample`) inside
an `if`, or after an early `return`** (vkd3d puts the rest of the function inside one).
Each place that did was rewritten so it samples what it sampled before:

- a texture with one level and the same min and mag filter (the noise volume of the
  fire, the surfaces' height cube, the nebulae's baked volume and blue noise) is read
  with zero gradients or `SampleLevel` at 0, which is level 0, as before;
- a mipped one (the rocks' baked cubes) is read with `SampleGrad`, the gradients taken
  before the branch from the same coordinates. A tap of the crack march at
  `q - vq * t` (t the same on every pixel) has the gradient `ddx(q) - ddx(vq) * t`, and
  needs no normalising: a cube lookup and its level read only the direction;
- where the branch only saved work in a pass that is rarely drawn (the nebulae's bake,
  their reference, the slices drawn without the march), the branches are `[flatten]`ed
  and the early returns turned into a choice at the end.

vkd3d (1.19) has no `tex3Dlod` or `texCUBElod` in the old dialect; the modern
`SampleLevel` it compiles to `texldl` on every kind of texture, and `SampleGrad` to
`texldd`.

How this was checked: every shader as committed is made on Windows 10's runtime on all
four kinds of device; and a d3dbc interpreter ran each shader as vkd3d wrote it and as
`d3dbc-fix.py` rewrote it, 300 random inputs and constants each (so the recipe's
branches go both ways), with identical outputs for all 86, and the old and new
compiles of every rewritten entry point with identical outputs too (textures there are
a function of the coordinate alone, so the levels are the argument above). With the
allocator's rule for ifs taken out, the interpreter found the marches wrong in every
run.

### One source, two compilers

Every shader of render3d and UI.asi is modern HLSL (`Texture2D`/`Texture3D`/
`TextureCube` with a `SamplerState`, `cbuffer`, `SV_Position`/`SV_Target`, `Sample`,
`SampleLevel`, `SampleGrad`, HLSL 2021's `select`), so that one source builds both the
Direct3D 9 shaders the plugins draw with now and SPIR-V for a Vulkan backend. vkd3d
writes the d3dbc as before and `dxc` (`-spirv`, Vulkan 1.1) the SPIR-V, which
`hlsl-rs.sh` puts through `spirv-val`. `gpu/shaders/compat.hlsli`, at the root because
both plugins include it, holds the few places the two compilers part (dxc predefines
`__hlsl_dx_compiler`; vkd3d defines nothing to tell it by, and takes no `-D` or `-I`, so
the shaders include it by a relative path that resolves from `shaders/` for both):

- **Constants keep their Direct3D 9 registers**: `CREG(n)` is `register(cn)` for vkd3d,
  which places a cbuffer member by it and ignores `packoffset`, and `packoffset(cn)`
  for dxc. So the Rust code sets the same registers as before, and in SPIR-V a member
  sits at byte 16 n of its buffer (`-fvk-use-dx-layout`): each stage's buffer is its
  register file. dxc wants a cbuffer's members in rising order, and refuses two that
  overlap; entry points of one file that use one register for different things have a
  cbuffer each.
- **Samplers keep their stages, though vkd3d does not**: for shader model 1-3 it makes
  each texture/sampler pair a combined sampler named `<sampler>+<texture>` and gives the
  pairs s0, s1, ... in order, whatever the source declares (any `register()` on either
  is dropped). `d3dbc-slots.py` puts each pair back on the stage the source names
  (`register(tn)` on the texture and `register(sn)` on its sampler, which must agree),
  in the code and the constant table, so the Rust code binds textures as before.
- **VPOS**: a pixel shader's `SV_Position` is the pixel's centre in Vulkan and its
  corner (VPOS, whole numbers) in Direct3D 9, where vkd3d reads it as VPOS.
  `d3d9_vpos()` gives the corner on both. vkd3d declares a float4 `SV_Position` as
  `vPos.xyzw`, which Windows refuses (the mask has to be `.x`, `.y` or `.xy`), so
  `d3dbc-fix.py` narrows it to `.xy`, and refuses a shader that reads its z or w.
- **`select`** is a macro over `?:` for vkd3d, which has the vector `?:` and no
  `select`; dxc refuses a vector `?:` under HLSL 2021.

The d3dbc this writes is the d3dbc the old sources wrote: the code token for token,
the constant table's sampler names aside, and in five pixel shaders (`fire_ps`,
`ps_cloak`, `ps_holo`, `neb_ps`, `vol_ps`) the `dcl` of vPos before the texture
coordinates' instead of after.

**SPIR-V's layout**, for the backend that will draw with it: every cbuffer of a vertex
shader is `register(b0)` (set 0, binding 0) and of a pixel shader `register(b1)` (set 0,
binding 1); several cbuffers of one stage alias that binding, which Vulkan allows. A
texture and its sampler are one combined image sampler (`STAGE(n)`: set 1, binding n,
the Direct3D 9 stage). Vertex shaders are compiled with `-fvk-invert-y`, so clip space
is Direct3D 9's (y up) under a viewport of positive height; window coordinates, and so
the winding a triangle is culled by, come out as Direct3D 9's. Interface locations are
dxc's: each struct member in declaration order, unused ones included, so a vertex
shader's outputs and the pixel shader's inputs agree wherever both use one struct.
Direct3D 9's half-pixel offset (pixel centres on whole coordinates) is the caller's:
DXVK moves the viewport by half a pixel, and the UI's `VP.zw` holds it.

## `Render3D.ini`

| Key | Default | |
|---|---|---|
| `Enable` | 1 | 0: the engine draws the 3D view, as without the plugin |
| `Log` | 1 | `Render3D.log` in the game directory |
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

**`[Modules]`.** Each module has a key of its own, `<name>=1` (the names are in the
sections below: `effects`, `ordnance`, `specials`, `sky`, `overlays`, `particles`,
`hulls`, `world`). **`<name>=0` leaves that part to the engine: its stock look, drawn by
`Armada2.exe` at the step as without the module.** There is no plugin to fall back to
any more, so the old plugins' looks exist only as these modules.

## Timing

`Timing=1` (`src/timing.rs`) times each view as a run of marks: the seam, our targets,
each step of the body (`setup` is the lights, the distance cull, the modules' lights
and the scene layer's shadow map; then `background`, `terrain`, `objects`, `ordnance`,
`specials`, `features`, `warp`, `hud prep`, `chunks`, `particles`, `translucent`,
`selection, end`) and `post` (resolve, bloom, tone map). Each mark notes the CPU clock
and issues a Direct3D 9 timestamp query; a span is booked to the mark that ends it.
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

`[Bench] Clock=` (`src/clock.rs`) decides what time every animation of render3d's own
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
glow. The device's state is captured into a state block before the passes and put back
after, and the engine's own target and depth are bound again, so the engine (and
d3d8to9, which keeps its own record of what is bound) sees what it left.

**Through `gpu`.** Everything render3d draws goes through `gpu` (`gpu/README.md`),
the interface shared with `UI.asi`: the targets, the scene pass and the passes after
it, and every module's draws, each through a `gpu::Cmd` (`gfx::cmd()`) with pipelines
made once per combination of states (`gfx::Pipes`), dynamic geometry in the frame's
rings, and the scene's models in static buffers. No module calls Direct3D 9; the frame's
glue in `frame/` still reads the engine's device (its transforms, its viewport, its
`Reset` and `DestroyRenderSurfaces`), which Display8 will answer itself. The scene pass
is begun before the body and stays bound while the engine draws into it; a module that
draws into a target of its own (a bake, the shadow map, the nebulae's march) begins
the view's pass again after it (`gfx::view_pass`), with the viewport the engine had
set. Stock's black line along the view's right column and bottom row is cleared in it
after the body (`frame::black_lines`).

**The first row and column.** DXVK offsets a Direct3D 9 viewport by half a pixel, so
with MSAA half the samples of the view's first row and column are never drawn. The final pass reads those pixels from their neighbours instead, which does what
`MSAA.asi`'s edge fill did without the frame's delay. On Vulkan (below) there is no
offset and no such fill, and the post passes' quads do not move by half a pixel
(`gpu::Gpu::half_pixel`).

**On Vulkan.** In Display8's stub mode with `Backend=vulkan` the same frame records into
Display8's command buffer through `gpu`'s Vulkan backend (`gpu/README.md`): the engine's
back buffer and depth are Display8's images, the view's transforms and viewport
Display8's record of the engine's calls (`engine_transform`, `engine_viewport`), the
fog's textures Display8's mirrors (`gfx::engine_texture`), and every pipeline is made
from the SPIR-V compiled beside each shader's d3dbc. The frame is told a new device by
`gpu::Gpu::engine_id`, which on Vulkan never changes; `Reset` and
`DestroyRenderSurfaces` find nothing to let go of. The menu scene's viewport is handed
to the pass rather than set on the engine's device, so the modules there read the
engine's own.

**Reset.** Every target is in the default pool and must be gone before the device
resets. Slot 14 of the d3d8 device's vtable (`Reset`) is chained, as `Lighting.asi`
chains it: render3d drops its targets and passes on the call, and makes new ones at
the next frame, at the new size. A display mode chosen in Graphics Settings is not a
`Reset`: the engine releases its device and creates another. The first frame on a
device render3d has not seen ("frame: a new device") drops the frame's state, hooks
the new device's `Reset` and hands the device to every module, which makes its
resources again; scenario `render3d-mode-switch` checks it, switching to
800x600 in a running battle.

**Letting go of the device.** Under DXVK that is enough. Under native Direct3D 9 it
is not: every object holds its device, so whatever render3d still held kept the shell's
device alive, and the engine's `CreateDevice` for the mission's display failed ("Fatal
error in Direct3D. Please reboot." from `SetActiveDisplay`). The display change goes
through `ST3D_DeviceDirectX8::DestroyRenderSurfaces` (slot 28 of its vtable, 0x6bc6ac),
which releases the device in `DestroyDevice`; render3d chains that slot and first lets go
of everything on the device (`frame::forget`): what a `Reset` drops, the sky's noise
volume, and every object `gpu` made (`Gpu::release_device`: it keeps them all in one
registry for this, and makes each again from its description at its next use). The
posed hulls of the last views and the ghosts' queue go too: they hold models, and a
model holds its vertex and index buffers (missing them left 49 of each, and the mission
did not start). `gpu`'s raw layer (`gpu::d3d9::raw`) keeps a tally of every object made or handed out and not yet
released; the hook logs what is left by kind ("still held after letting go"), which is
how those were found, and unbinds every texture, stream and shader before the engine
releases the device. It
holds the d3d9 device across the engine's release and logs how many references are
left: 0 means it is gone. `UI.asi` does the same (`ui/src/engine/device.rs`); when it
is in the chain after render3d, render3d's count includes the UI's one reference.

**Without Direct3D 9.** The d3d9 device is asked for behind the game's d3d8 one
(`platform/D3D9.md`). Under DXVK's own d3d8, or when the targets cannot be made, the
body runs as stock. An `A16B16G16R16F` target with MSAA falls back to one without, then
to 8 bits, and the log says which.

## Frame dumps

A grim shot is 8 bits, taken after the swapchain and the compositor, so it cannot say
where in the frame two renderers part. `src/dump.rs` reads render3d's own targets back
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
not guessed at. **Depth is not dumped**: Direct3D 9 cannot read a depth surface back
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

`src/menu/`, the map in `menus/map.ini`: a map of render3d's own behind the front end's
main menu and campaign selection, every scene on it under one sun. A Federation fleet
in orbit over a class M planet, a starbase beside it, Klingon ships before a Metreon
nebula far out on one side, a Borg fusion cube and the queen's Diamond in a Mutara nebula
on the other. `UI.asi`
asks for a **view** of it by name (`[Menus] Scene=1`, ui/MENUS.md, "The menu scene")
through render3d's export `Render3D_MenuScene(name, x, y, w, h)`: inside its own
scene, with the back buffer bound, it gets the view drawn into that rectangle and draws
the menu over it. A 0 back (no such view, no Direct3D 9 device, no targets) leaves the
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
| `Sky` | sky | the recipe sky of `Sky=` (`skies/`), turned about +x by `SkyTurn` a second; a starfield of our own (`Stars` quads of the stock `bstar.0` and `mstar.0` sprites, seeded, the brighter ones lifted past white); the sun, `Sun=`'s SOD (a background object, `Msun1`) and its sprite nodes where the Key comes from, drawn about the eye as a map's suns are, so it stands in the same place from every view |
| `Planet` | world | `Planets::draw_menu`: the class's ground (`groundTextureName`, or its surface recipe) and cloud shell (`atmosphereTextureName`, stock's `PA_ECFR0` when the ODF names none) on render3d's sphere, the same body draw as a mission planet's (`draw_body`) without cities |
| (depth cleared) | | the ships and the gas are drawn on a projection of their own, 2 to 16000 |
| `Ships` | hulls | `menu_begin` poses every ship of the map (finest level of detail, its animation looping, `hardpoints` and the damage, crew and Borg nodes hidden by name, as no class says which) and draws the shadow map from those in view, with the menu planet in place of the engine's; then the hull look, opaque, of the ships in view, and their running lights collected |
| `Gas` | world | `Nebulae::draw_menu`: each `[Nebula.*]` field's nebulae as the class's gas, marched and lit by the Key with the ships' shadows, as a mission's; before the cloak, which bends a copy of what is behind a hull |
| `Translucent` | hulls | additive meshes, decloaking hulls (the cloak's glass, as the engine's state 3) and sprite nodes, as at a mission's `Step::Translucent` |
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
the camera has gone (`Blend`), then cloaks again for the next arrival. Each ship is
heard as its decloak begins (fully cloaked in one frame, part of it back in the next):
its group's `CloakSound`, by default `clkdisen.wav`, the `offSound` the Klingon
cloaking device's ODF (`gkcloak.odf`) gives for "the ship decloaks". It is a 2D sound
of the game's own sound system (`engine/sound.rs`: an `AudioSound2D` made with the
game's `operator new` at priority 8, as `ShellButton::SetButtonMode` makes its hover
sound), so the player's effects volume applies; it is a sound, not a simulation write,
and plays only in the menu scene.

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
  branch, as vkd3d needs. A soft wall of gas in the group's `WarpColour` streams past
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

**The map.** `menus/map.ini` is built into the plugin; `Menu\map.ini` in the game
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
| | `CloakView`, `CloakDelay`, `CloakTime`, `CloakStagger`, `CloakSound` | the view whose arrival decloaks its ships; the seconds before the first starts (`Blend` x 0.7), each takes (2.5), between one and the next (0.5); the sound each makes as it begins (`clkdisen.wav`; empty for none) |
| | `WarpColour` | the tunnel's tint when the group departs (0.45,0.62,1) |
| `[View.<name>]` | `Group`, `Eye`, `Target`, `Fov` | in the group's frame; vertical field in degrees |
| | `Sway`, `SwayPeriod` | the drift's amplitude and period |
| | `WarpEye`, `WarpTarget` | the camera a departure from the view swings to, in the group's frame (default: behind the departing ships, looking the way they go) |
| `[Ship.<name>]` | `Group`, `Odf`, `At`, `Yaw`, `Scale`, `Bob`, `Phase`, `Team` | its group; the ship's ODF (its `baseName` and `ScaleSOD`), its place in the group, a turn in degrees, a scale on top of `ScaleSOD`, how much it bobs, its phase, a team colour for its `team*` groups |
| | `Warp` | 0: stays when its group departs (a station) |
| `[Nebula.<name>]` | `Group`, `Odf`, `At`, `Size`, `Spacing`, `Jitter`, `Seed` | its group; a nebula ODF with a gas recipe; the field's centre (its plane's height is its y); half extents in x and z; the grid's step (the radius x 1.2) and jitter (0.35); the seed |
| | `Radius` | the class's radius (the ODF's `effectRadius`, else 300) |
| | `KeyShare`, `Scatter`, `Opacity`, `Brightness` | over the recipe's, for the menu's class only (one set per class: the first field of the class that sets a key) |

## Modules

### effects

`src/mods/effects/`, on by default. The game features that are effects, hull debris,
and their lights; it ports `Explosions.asi`, `Shields.asi` and `Repair.asi`, whose
looks were settled with the user (kept as they were; the settings keep the plugins'
keys, in `Render3D.ini` sections `[Explosions]`, `[Shields]`, `[Repair]` and
`[Debris]`, read once at start). Each part stands down while a stale copy of its old
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
- **Construction** (`construction.rs`, `weldfx.rs`, `shaders/effects_construction.hlsl`).
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
   unchanged (`fx.rs`, `fx_ps` in `shaders/ordnance.hlsl`), on the sim clock;
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

**Settings.** The recipes and `[Warp]` are `render3d/ordnance.ini`, built into the
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

`src/mods/specials/`, `shaders/specials.hlsl`, on by default (`[Modules] specials=0`
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
   - **With a recipe** (`render3d/skies/<name>.ini`, built into the plugin; a
     `Sky\<name>.ini` in the game directory takes precedence and is re-read when it
     changes), the procedural sky of `Sky.asi`, each recipe's shape and grade fitted
     to its stock sky, with soft billows and no filaments ("Sky recipes" below): one quad over the
     viewport carrying each corner's world ray (from the inverse of view x projection,
     far point minus near point, which is affine in screen position), sampling a cube
     baked once per recipe. The bake is on the GPU only: each face is drawn by the
     `bake_*_ps` chain (five passes, "Shaders on Windows") at twice its edge
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

`Render3D.ini`, `[Sky]` (read again every 60 views):

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

**Sky recipes** (`render3d/skies/<name>.ini`, section `[Sky]`, one per stock sky name;
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

### overlays

`src/mods/overlays/`, `shaders/overlays.hlsl`; on unless `[Modules] overlays=0`, and
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
`IDirect3DTexture8`, which d3d8to9 answers for its `IDirect3DTexture9`. They are never
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
time. While the engine's `RenderSelectionEffects` has been made to return at once
(its first byte `ret`: UI.asi's `[HUD] SelectionBubbles=0`), none is drawn, as the
engine would draw none. The material `selection` is constant: the colour is added as it is, depth-tested,
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

**Height posts** (`posts.rs`, `vs_line`/`ps_line` in `shaders/overlays.hlsl`).
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

### particles

#### particles

`src/mods/particles/`, `shaders/particles*.hlsl`, on by default; stands down while
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

### hulls

#### hulls

`src/mods/hulls/`, `shaders/hulls.hlsl`, on by default; supersedes `Lighting.asi`'s
hull path (and stands down while a stale `Lighting.asi` is loaded). Settings in `[Hulls]`
(Lighting.asi's keys and settled values: `Ambient`, `Specular`, `SpecularPower`,
`RimLight`, `RimPower`, `HullSun`, `HighlightKnee`, `SelfIllumination`, the `Borg*`
profile, `PointLights`, `NearFade*`, `Cloak*`, plus `Shadows`, `ShadowSize`,
`ShadowStrength`, `PlanetShadows`, `BumpStrength`, `SpriteNodes`, `AdditiveMeshes`,
`TeamColours`, `Scrap`) and the scene lights in `[Lights]` (`Replace`, `KeyColour`,
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
`ST3D_CreateNormalMap` builds it, in a tangent frame worked out per pixel. Normals:
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

`src/mods/world/`, `shaders/world_*.hlsl`, on by default, registered first (it takes
the nebulae and the moons before `particles` and `hulls` see them). The objects whose
class overrides `Render` and draws a world of its own: planets, the dilithium moons,
asteroid fields, nebulae. It ports `Surfaces.asi` (the recipes, `Surfaces\<texture>.ini`),
`Lighting.asi`'s planets and its nebula and planet lights, `Planets.asi` (moot: our own
sphere) and `Nebulae.asi` (the recipes, `Nebulae\<odf>.ini`); the looks settled with
the user are kept. Each part stands down while a stale copy of the plugin that drew it
is loaded: the planets beside `Lighting.asi`, the moons and asteroids beside
`Lighting.asi` or `Surfaces.asi`, the gas beside `Nebulae.asi`, the lights beside
`Lighting.asi`. The recipes live in this folder: `skies/` (built into the plugin),
`surfaces/` and `nebulae/` (copied by `install.sh` into the game's `Surfaces\` and
`Nebulae\`, where an edited copy is re-read while the game runs); see "Recipes" below. Settings in `[World]` (the parts: `Planets`, `Rocks`, `Nebulae`,
`Surfaces`; the planet look under `Lighting.ini`'s names: `PlanetAmbient`, `PlanetDusk`,
`PlanetWrap`, `PlanetFill`, `PlanetSun`, `PlanetHaze*`, `PlanetGlint*`, `CityLights`,
`CityLightColour`, `PlanetHalo*`, `PlanetGhost`; the rocks' `RockAmbient`, `RockSpecular*`,
`RockRimLight`, `RockRimPower`, `RockSun`, `MoonShell`, `MoonCracks`, `MoonPulse`,
`MoonCrackCore`, `MoonCrackDepth`, `MoonCrackFalloff`, `MoonNimbus`; the lights' `PlanetGlows`,
`PlanetGlow`, `PlanetGlowRange`, `NebulaLights`, `NebulaBrightness`, `NebulaRange`,
`NebulaRecipeColour`, `NebulaCull`) and `[Nebulae]` (`Enable` 1, `Steps` 8, `Denoise` 1.5, `Footprint` 1, `Temporal` 0.1, `TemporalClip` 1.25, `Jitter` 1, `Slices` 40, `MinStep` 6, `NoiseSize` 128, `Obscure` 0.5, `Samples` 2, `KeyShare` 0.8, `Scatter` 0.2, `Bake` 1, `VolumeSize` 256, `VolumeHeight` 160, `VolumeTexel` 3, `Shadows` 1, `ShadowSoftness` 0.02), read again
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
wrapped. Everything is in the default pool; a `Reset` bakes again.
The gas is **marched** at full size, not sliced. Each class draws its gas box once
(the faces facing the eye, or those facing away when the eye is in or near it), and each
pixel finds its own ray (c41..c43: the ray at view depth 1 is linear in the pixel), cuts
it to the box and to the scene's depth (a RESZ copy into INTZ, read as a texture; with
this projection `z - _33` is below 0), and takes `Steps=` steps (8, 12, 16 or 24, an
entry point each, since vkd3d unrolls the loop) spaced in proportion to the distance,
each jittered in its span by the pixel's blue-noise threshold walked by the golden
ratio. Its sums (light, optical depth) are added into a half-float target; `comp_ps`
filters them with a 5x5 Gaussian of `Denoise=` pixels (sigma) that leaves out pixels
whose scene depth differs by over 5%, so hull edges stay a pixel sharp, and applies
`Opacity=`/`Obscure=` onto the frame. The slices (`vol_ps`) remain only without INTZ
and for `Bake=0`. vkd3d has no `tex3Dlod`, and a read's level from the hardware's
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
march took 2.53 ms either way against vkd3d's 2.52: DXVK's SPIR-V and the driver end
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

**Lights** (`lights.rs`), without `Lighting.asi`. *Planet glows*: per planet and view
(`Module::lights`), a point light at its centre (the instance's transform +0x44 plus
its bounding sphere +0x34) in the Key's colour times its ground's mean colour times
`PlanetGlow`, full at the surface and gone at `PlanetGlowRange` radii, marked a day
side (`scene::lights::set_day_side`): a draw takes it times the share of the day side
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

- `skies/<name>.ini`: the sky recipes, built into the plugin ("Sky recipes" above).
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
