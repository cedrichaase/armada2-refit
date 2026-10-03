# D3DTrace — what the renderer asks Direct3D for

`D3DTrace.asi` logs the Direct3D 8 calls Armada II makes: the lights, the
fixed-function stages, the vertex formats and shaders, per draw. It is a bench tool.
`install` refuses any game directory that is not an `a2test` clone, so it never goes
into the player's install, and `a2mod` does not know about it.

    ./a2test session start --res 16:9 --install . --install testbench/d3dtrace
    # get to the scene to trace, then:
    touch ~/.cache/a2test/sessions/<id>/game/D3DTrace.go
    testbench/d3dtrace/summarize.py ~/.cache/a2test/sessions/<id>/game/D3DTrace.log

The log is also gathered into the case's `logs/` when the session stops.

## How it hooks

Armada2.exe imports `Direct3DCreate8` by name; the plugin points that IAT entry
(`0x7b82cc` in GOG 1.1) at a wrapper, after checking that the entry holds
`d3d8.dll!Direct3DCreate8`. Otherwise it logs `NOT HOOKED` and does nothing. The
wrapper patches `IDirect3D8::CreateDevice` (slot 15), and that wrapper patches the
device's vtable. Slot numbers are Wine's `include/d3d8.h`. No code in the exe is
touched, so it composes with `MSAA.asi`.

## What it writes

- **Always:** `CreateDevice` with its behaviour flags, every vertex and pixel shader
  created (declaration and token stream), and every 600 frames one line of per-frame
  counts (`draws`, `prog` = draws with a programmable vertex shader, `setlight`).
- **On demand:** while `D3DTrace.go` exists beside the exe (checked every 30 frames,
  deleted when seen) the next `Frames=` frames (`D3DTrace.ini`, default 2) are traced in
  full: the light table, every `SetLight`, `LightEnable`, `SetMaterial` and
  `SetVertexShaderConstant`, and one `D` line per draw with the state that decides how
  it is lit. `summarize.py` names the enums and groups the draws by lighting path.

## What it showed: how Armada II lights a frame

Observed on 2026-10-03, first Borg mission, 1920x1080, refit, with MSAA (session
`20261003-214418`). These findings come from observing the running game.

- **Direct3D's fixed-function lighting is never used.** `D3DRS_LIGHTING` is 0 on every
  draw. Every mesh outside the bump-mapped path, including ships, stations, planets and
  the skybox, arrives **pre-transformed** (`XYZRHW|DIFFUSE[|SPECULAR]|TEX1`): the engine
  transforms and lights vertices on the CPU and hands the GPU screen-space triangles with
  a lit colour. A shader at the D3D level cannot relight these draws, because their
  normals never reach the device. This holds although the device is created with mixed
  vertex processing (`flags=0x86`) and *Hardware Vertex Processing* is on.
- **Bump mapping is a real per-pixel path, and on by default.** *Graphics Settings →
  Bump Mapping* (default On) sends meshes that have a bump map through a `vs.1.0` vertex
  shader, one per game, created at load and assembled with D3DX8. Borg meshes go through
  it. Each mesh is drawn in **four passes over the same geometry**:
  1. `DOTPRODUCT3(normal map, light vector in diffuse) × TFACTOR` (light colour), opaque.
     The normal maps are 256x256.
  2. The same for the second light, additive (`ONE/ONE`).
  3. The diffuse texture, multiplied in (`DESTCOLOR/ZERO`).
  4. The diffuse texture again, alpha-blended (`SRCALPHA/INVSRCALPHA`): the night-lights.
  In the traced frame these four passes were 256 of 1500 draws and 32,600 of 35,730
  triangles.
- **With Bump Mapping off** the same Borg hulls fall back to the CPU path: two passes,
  texture × CPU-lit vertex colour with `SPECULARENABLE` on (CPU specular), then the
  alpha pass. No shader is bound at all.
- **The scene's lights are two directionals, set once.** Light 1 is the key light, grey
  0.753 from direction (0, 0.707, −0.707). Light 0 is a dim blue fill (0.125, 0.125,
  0.251) from the opposite direction. Neither has a specular colour. `SetLight` is not
  called per frame. No point light appeared in a calm frame; whether weapon lights
  (`lightColor`/`lightFalloff*` in 74 weapon ODFs) reach the device in combat has not
  been traced yet.
- **What bump mapping changes on screen, at play zoom.** Four idle Borg ships at
  ~100 px, bump on against off, same camera (grey mean / standard deviation over each
  ship's box): 13.0/32.3 vs 12.6/29.7, 9.4/16.7 vs 14.0/25.0, 6.8/15.2 vs 10.4/22.3,
  20.0/35.3 vs 23.5/37.6. On means darker hulls, by up to a third, and no visible relief
  at that size. Both paths render correctly under DXVK.

## Experiment A: Federation hulls on the bump path

Became `models/hull-bump.py`. What a SOD needs, why the height map is flat, and the
measurements are in `models/README.md`, "Hull lighting". To try a variant on the
bench, run it against a clone: `A2_GAME=~/.cache/a2test/sessions/<id>/game
models/hull-bump.py --install`, after `session start --no-launch`.
