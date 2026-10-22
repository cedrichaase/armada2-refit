# renderer — DXVK settings

One `a2mod` layer: **renderer** (`renderer-config.sh`, which writes `dxvk.conf`). It
applies to every frame at once and is independent of the texture pipeline.
Anti-aliasing is covered here too, as Tier 2, because it was chosen among these routes;
the multisampling itself is `Render3D.asi`'s (`render3d/`), as is every post-process
effect (bloom, tone mapping): see `render3d/README.md`.

This folder was `postfx/` until renderer 2.0.0, when the vkBasalt bloom layer it also
held was removed in favour of render3d's own.

Everything in this section is global — it applies to every texture and every model at
once, costs nothing per-asset, and is reversible by deleting one file. All of it is
independent of the texture pipeline. Verified against **DXVK 3.1.1**, the version Heroic
actually deploys (`~/.config/heroic/tools/dxvk/dxvk-3.1.1/`), by reading the config keys
out of the shipped `d3d9.dll` rather than from documentation — the key list below is what
this binary honours, not what some DXVK version honours.

All of it stands on the render chain in `platform/README.md`: DXVK's `d3d8.dll` and
`d3d9.dll` in the game directory. `d3d9.*` keys are read by DXVK's d3d9 layer, which is
in the chain whichever `d3d8.dll` sits in front of it (`d3d8-chain.py --use dxvk` or
`--use gog`).

## Tier 1 — `dxvk.conf`, free and reversible

`renderer/renderer-config.sh` writes it, in **cumulative stages**, because this project's
method is one change at a time and three keys at once is not attributable.
All three stages are installed. They went live across three launches and were never
A/B'd one at a time, so how much each contributes is unmeasured; `--stage N` and
`platform/ab-shot.sh` are what would settle it. It verifies every key against
the `d3d9.dll` that actually loads — the one in the game directory — before writing,
because a key DXVK does not recognise is silently ignored:

    renderer/renderer-config.sh              # stage 1: anisotropic filtering only
    renderer/renderer-config.sh --stage 2    # + mip LOD bias
    renderer/renderer-config.sh --stage 3    # + seamless cube filtering
    renderer/renderer-config.sh --bias -0.25 # milder stage 2 bias (default -0.5)
    renderer/renderer-config.sh --show       # what is installed now
    renderer/renderer-config.sh --remove     # delete dxvk.conf, full revert
    renderer/renderer-config.sh --print --stage 3   # the file on stdout; the release zip's copy

- **`d3d9.samplerAnisotropy = 16`** (stage 1). Without it the game gets whatever a 2001
  renderer requests, which is likely none. In a top-down RTS every hull
  and every planet is drawn at a steep angle to the camera, so all of it is sampled with
  plain trilinear and blurs along the axis of foreshortening. This is the single biggest
  free win and the one to judge on its own.
- **`d3d9.samplerLodBias = -0.5`** (stage 2). Biases mip selection toward the sharper
  level. This matters *because of the texture work*, not independently of it: there is
  now 1024–2048 art under a camera that views the map plane obliquely, and trilinear
  picks a blurrier mip than the art can support. **Only safe with AF already on** —
  without it a negative bias aliases rather than sharpens, which is why it is a separate
  stage and not part of stage 1. `d3d9.clampNegativeLodBias` is the guard if it
  overshoots. **The failure mode is shimmer on movement, not blur when
  still** — and in this game it shows on the billboard layer first, so if the map crawls
  while scrolling, halve the bias with `--bias -0.25` before abandoning the stage.
  Below about -1.0 it aliases faster than AF can clean up.
- **`d3d9.seamlessCubes = True`** (stage 3). Filters across cube-map face edges. Directly
  relevant to the finished skybox class: many maps bind a `.sod` cube model, and a
  face-edge seam gets *more* visible at 2048/face, not less.
- **`d3d9.cachedWriteOnlyBuffers = True`** (every stage). Not an image setting: it
  makes the engine's CPU mesh path cheap under DXVK. Without it, 30 selected ships
  cost 70 ms a frame. The next section explains why. A DXVK build that does not
  know the key gets the stages without it and a warning.

### Reading back a dynamic vertex buffer

Selecting many ships cost frame rate in proportion to the number selected, and the
frame rate came back as soon as they were deselected. The cost is the **selection
bubble**, the grey translucent ellipse around each selected ship. It is
`SelectionEffect` (`SelectionEffect.obj` in `armada2.map`): an `ST3D_Instance` of
`SOD/select.sod` (one GeoSphere, 162 vertices, 320 faces) that
`SelectionDisplay::PreRender` adds per selected object (`AddSelectionEffect`,
0x599800) and `RenderSelectionEffects` (0x599900, called at 0x5984c0 from
`Armada_RenderAllOurStuff`) draws once or twice per ship. Its shared material,
"selection", is set as the engine's override material for the draw, coloured by team
relation, and scaled to the ship's shield ellipse. The draws run after the scene,
with z-sorting off and a different z-compare. Because the material is translucent,
`ST3D_Mesh::RenderInternal` sends it down the CPU path (`RenderInternalNonVB`), as
`models/README.md` found for the moons' glow shells.

The mesh is small. The cost is how the CPU path writes it. A test-only plugin timed
the call at 0x5984c0 and sampled the main thread's instruction pointer (bench,
1920x1080, 30 Galaxy class, `testbench/scene`). With 30 selected, the bubbles took
67–71 ms of a 69–73 ms frame. 95% of the samples fell on the two compares in
`ST3D_VertexLighting_Group::RenderFacesInsideImmediateNoSpecular` (0x645f40) that
check a face corner's texture coordinate against the vertex already written for it.
`perf stat` measured 0.07 instructions per cycle, with no kernel time or page faults
to speak of. The vertex that is compared against lives in the workspace's output
array, which is the locked Direct3D dynamic vertex buffer. In `/proc/<pid>/maps` that
array is a mapping of `/dev/dri/renderD128`: DXVK puts a write-only dynamic buffer in
GPU memory, where every CPU read is uncached and crosses the bus, at about 1.2 µs a
compare.

`d3d9.cachedWriteOnlyBuffers = True` keeps those buffers in cached host memory. The
engine draws exactly as before (same triangles, states and order), so the bubble
looks the same. Measured with the timer above (frame times at 60 Hz vsync):

| Selected | Frame, before | Bubbles, before | Frame, after | Bubbles, after |
|---|---|---|---|---|
| 0 | 16.7 ms | 0 | 16.7 ms | 0 |
| 15 | 37.3 ms | 35.4 ms | 16.7 ms | 0.24–0.37 ms |
| 30 | 72.6 ms | 70.6 ms | 16.7 ms | 0.60–0.81 ms |

Why a renderer key and not a hook in the old `Lighting.asi`, which moved the hulls onto
vertex buffers: the slow part is not the CPU transform of 320 faces, it is reading
uncached memory. A GPU path for the bubble would also have to reproduce a translucent,
team-coloured, depth-unsorted draw through the fixed-function pipeline. The key fixes
every other draw on the CPU path as well: planets, the moons' glow shells, cloaking
ships, and every hull with `GPU=0`. How much those gain has not been measured. The
key also costs the GPU a little: it now reads those vertices from system memory. With
the frame at the vsync cap on the bench, that cost did not show.

`DXVK_HUD=fps,frametimes` is the measuring aid; it is deliberately **not** written into
`dxvk.conf`, so it cannot be left on by accident. Expect the GPU to be near-idle — a 2001
engine against an RX 5700 XT — and all three stages to be free.

## Telling whether a renderer setting did anything

Two separate questions, and they need separate tools, because "the setting was
ignored" and "the setting worked and is subtle" look identical in game.

**Is it applied?** A `dxvk.conf` key DXVK does not recognise is *silently ignored* — no
error, no warning. DXVK does print its effective configuration at startup, so ask it:

    platform/dxvk-logging.py --diagnose  # logging + HUD + d3d9=n,b, all at once
    # launch the game once
    platform/dxvk-logging.py --check     # report the effective configuration
    platform/dxvk-logging.py --off       # take all of it back out again

`--diagnose` answers three questions in one launch, ordered so each makes the next
meaningful:

| observation | conclusion |
|---|---|
| no DXVK HUD overlay in game | DXVK is not in the chain; **no** `dxvk.conf` key can work |
| HUD shown, but no log file | DXVK's d3d9 layer never loaded |
| log written, keys absent from `--check` | DXVK ran but never found or read `dxvk.conf` |
| keys present in `--check` | it is applied, and the effect really is that subtle |

**Why `d3d9=n,b` is part of the diagnosis.** DXVK's `d3d8.dll` imports `d3d9.dll` by
name — confirmed with `objdump`, not assumed — so every `d3d9.*` key is read by a layer
that only exists if Wine resolves `d3d9` to DXVK's build rather than its own builtin
WineD3D. Setting `WINEDLLOVERRIDES` at all *replaces* whatever Proton would have set, so
without that entry the d3d9 slot is not native. The working chain carries it
permanently (`d3d8-chain.py` owns it); `--diagnose` adds it only in case it is missing,
and `--off` never removes it.

**Heroic must be closed** for `--on`/`--off`: it rewrites `GamesConfig` on exit and
would discard the edit. The script refuses rather than losing the change silently, and
backs the file up regardless. Note Heroic's key is spelled `enviromentOptions`, missing
an `n` — matching its typo is required.

**Did it change the picture?** `platform/ab-shot.sh` grabs frames and diffs them
numerically, so the answer is a number rather than an impression:

    platform/ab-shot.sh grab before
    # change one thing, relaunch, return to the same save without moving the camera
    platform/ab-shot.sh grab after
    platform/ab-shot.sh diff before after 600 400 1200 300     # W H X Y, region only

For a before/after of the project as a whole rather than one setting, `./a2mod stock`
and `./a2mod refit` flip every visual layer at once in about 4 s, with DXVK kept
in both states, so the two grabs differ only in what this project changed. Quit the game
between them; Heroic can stay open.

Aim at a region, not the whole frame: a whole-frame diff of this game is dominated by
ships drifting and sprites animating between the two grabs, which will swamp the effect
and make any setting look like it did something.

## Why anisotropic filtering is structurally quiet in THIS game

Worth stating plainly, because the usual "AF transforms an old game" advice assumes
content this game does not have. AF only acts on surfaces **oblique to the camera**, and
it is a correction to *mip selection* — so it can only touch geometry that is both
mipmapped and foreshortened.

- **The dominant visual layer here is immune by construction.** Map nebulae, resource
  clouds, weapons and explosions are camera-facing **billboards**. A billboard is never
  oblique — that is what makes it a billboard — so AF cannot affect any of it. In a
  frame like the one that prompted this, that is most of what the eye is drawn to.
- **What AF *can* reach is small on screen.** Hull flanks, planet limbs and the skybox
  at grazing angles. A Sovereign draws ~340px wide in a top-down RTS, so its
  near-grazing side surfaces are a few dozen pixels tall. Real, and not dramatic.

So a subtle result at stage 1 is the expected result, not evidence of a broken setting —
which is exactly why `--check` exists to separate the two.

**The corollary is the useful part: stage 2 should be the visible one here.** A mip LOD
bias is not conditional on obliquity — it shifts mip selection for *every* mipmapped
surface, camera-facing billboards included. That is precisely the layer AF cannot touch
and precisely where the upscaled art lives. Expect stage 2 to do more for this game's
appearance than stage 1, which inverts the usual ordering. Stage 1 still goes first:
AF is what keeps the sharper mips stage 2 selects from aliasing on the oblique surfaces.

## Tier 2 — anti-aliasing: route 2, in the plugin

**Route 2 is `Render3D.asi`'s `MSAA=4`** (0, 2, 4 or 8; `render3d/README.md`): it draws
the 3D view into a multisampled half-float target of its own and resolves it into the
back buffer. It replaced `MSAA.asi`, which redirected the two `call GetWindowHandle`
before `IDirect3D8::CreateDevice` in `ST3D_DeviceDirectX8::CreateDevice` (`0x6235d0`) to
set `MultiSampleType` on the device's back buffer (8x, `SwapEffect=DISCARD`, a
non-lockable back buffer). That plugin was removed; the reasoning below is why an
ASI-side route was the one taken. Its minimap trap went with it: the minimap's
back-buffer read-back (`CopyOffscreenToTexture`) worked only because DXVK's D3D8
CopyRects resolves a multisampled source through `StretchRect`, and render3d's engine
back buffer is not multisampled.

Why that route, of the two:

**DXVK 3.x ships no MSAA-forcing key.** Read out of the binary that actually loads, not
from documentation: there is no `forceSwapchainMSAA` or equivalent.
`d3d9.forceSampleRateShading` exists but only does anything once MSAA is already on. So
AA cannot come from `dxvk.conf`, and the routes are:

1. **`dxcfg.ini`'s own `antialiasing=` key — inert.** It is read by the GOG d3d8to9
   translator, which is not in the chain; DXVK's own d3d8 is. It would come back only
   with `d3d8-chain.py --use gog` (GOG d3d8to9 → DXVK d3d9 → Vulkan), which would make
   both `anisotropic=` and `antialiasing=` live again. Untested, and its accepted values
   are unknown because `dxcfg.exe`'s strings are packed.
2. **An ASI hook**, first on `IDirect3D8::CreateDevice` setting `MultiSampleType`, now a
   target of render3d's own — the one taken. Squarely inside the toolchain `menus/` proves — clang + lld-link + the
   Ultimate ASI Loader — and `armada2.map` gives the call site. DXVK implements D3D8
   multisampling properly on Vulkan, which wined3d's D3D8 path did not reliably do.

**Supersampling by rendering above display resolution is a trap.** It looks free given
the GPU headroom and is not: nothing in this chain scales a mode larger than the screen
down to it, and the HUD, menu and cursor corrections all treat the display mode as the
screen.
