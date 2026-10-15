# render3d — the 3D view through our own renderer

`Render3D.asi` takes over the engine's 3D view: both calls of `Armada_RenderAllOurStuff`
(0x598110) come to it, so the scene is drawn into render3d's own targets and put into
the back buffer by render3d. It is written in `no_std` Rust (`src/`), and the plan, its
milestones and every change to it are in [`PLAN.md`](PLAN.md).

What it does today (milestone 2, "wrap mode"):

- **The frame is ours.** The view is drawn into a half-float (`A16B16G16R16F`) target
  with 4x MSAA and a depth of its own, then resolved, bloomed and tone-mapped into the
  engine's back buffer at the view's rectangle. Additive light (explosions, weapons,
  shield hits, glows) now sums past white instead of clipping, and what lies past
  white glows by how bright it is: bloom by energy, which an 8-bit back buffer cannot
  hold. Below the tone map's shoulder every colour is stock's.
- **Every step inside the view is still the engine's**, so every plugin hooked into
  those steps (`Lighting.asi`, `Sky.asi`, `Nebulae.asi` and the rest) keeps drawing
  its part, into our target. Modules of our own replace the steps one category at a
  time (PLAN.md, milestone 4).
- **`MSAA.asi` is superseded**: the scene target has samples of its own, and the back
  buffer stays 1x. `install.sh` removes it.

## Building and installing

    render3d/install.sh            # build, install Render3D.asi + Render3D.ini, remove MSAA.asi
    render3d/install.sh --remove   # take it out again

`./install` runs it in MSAA's place. Building needs rustup with the 32-bit Windows
target (`rustup target add i686-pc-windows-msvc`) and `lld-link` (LLVM); `build.sh`
uses `~/.cargo/bin/cargo` first, since a distribution's cargo has no such target. The
crate links with `/nodefaultlib`, `kernel32` its only import (an import library made by
`llvm-dlltool --kill-at` from `kernel32.def` in `build.rs`), and brings its own
allocator, memory functions and stack probe (`src/rt/`). Shaders are HLSL in
`shaders/`, compiled by `hlsl-rs.sh` with `vkd3d-compiler` into `src/shaders.rs`, which
is committed with a stamp so a machine without vkd3d builds as long as it is current.

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

**The first row and column.** DXVK offsets a Direct3D 9 viewport by half a pixel, so
with MSAA half the samples of the view's first row and column are never drawn
(`msaa/README.md`, "The top row and left column"). The final pass reads those pixels
from their neighbours instead, which does what `MSAA.asi`'s edge fill did without the
frame's delay.

**Reset.** Every target is in the default pool and must be gone before the device
resets. Slot 14 of the d3d8 device's vtable (`Reset`) is chained, as `Lighting.asi`
chains it: render3d drops its targets and passes on the call, and makes new ones at
the next frame, at the new size.

**Without Direct3D 9.** The d3d9 device is asked for behind the game's d3d8 one
(`platform/D3D9.md`). Under DXVK's own d3d8, or when the targets cannot be made, the
body runs as stock. An `A16B16G16R16F` target with MSAA falls back to one without, then
to 8 bits, and the log says which.
