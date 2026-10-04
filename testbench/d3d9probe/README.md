# D3D9Probe — can a plugin reach Direct3D 9 behind the game's d3d8?

`D3D9Probe.asi` asks the game's `IDirect3DDevice8` for `IDirect3DDevice9` and, when it
gets one, uses it: shader model 3.0 and 2.0 shaders, drawn into the game's own frame.
It is the spike behind `platform/D3D9.md`, kept so the chain can be re-checked after any
change to it. A bench tool: `install` refuses any game directory that is not an `a2test`
clone, and `a2mod` does not know about it.

    # the clone's own chain (DXVK's d3d8: QueryInterface fails, the probe only logs)
    ./a2test session start --install . --install testbench/d3d9probe
    # a d3d8to9 in the clone's d3d8 slot
    D3D9PROBE_D3D8=<path>/d3d8.dll ./a2test session start --install . --install testbench/d3d9probe
    # mid-scene shader swap, while the file exists:
    touch ~/.cache/a2test/sessions/<id>/game/D3D9Probe.tint

`D3D9Probe.log` is gathered into the case's `logs/` with the others.

## What it does

It hooks like `testbench/d3dtrace/` (the `Direct3DCreate8` IAT entry, then
`IDirect3D8::CreateDevice`, then the device's vtable), so the game's code is not
touched and `MSAA.asi` composes with it.

- **After each `CreateDevice`:** `QueryInterface(IID_IDirect3DDevice9)`, and on success
  the Direct3D 9 caps that matter for lighting: vertex and pixel shader version, vertex
  shader constants, `MaxActiveLights`, `MaxSimultaneousTextures`.
- **Every `Present`:** a 256x256 pattern in the top-left corner, drawn through the d3d9
  device with a `vs_3_0`/`ps_3_0` pair. The pixel shader colours by `frac(vPos / 32)`,
  so 32-pixel gradient tiles mean per-pixel shading ran. State is saved and restored with
  a `D3DSBT_ALL` state block and the render target is put back.
- **While `D3D9Probe.tint` exists:** every `DrawIndexedPrimitive` with a stage-0
  texture runs with a `ps_2_0` shader that multiplies the texture, the fixed-function
  vertex colour and a red tint. That is a shader substituted per draw, in the middle of
  the game's scene, with its textures and its vertex lighting still feeding it — the
  first thing a shader-based `Lighting.asi` has to be able to do. The previous pixel
  shader is restored after each draw.
- **Every 600 frames:** the mean frame time. On the bench it is pinned at 16.7 ms by
  vsync, so it shows a chain that drops frames, not throughput.

The shaders are hand-assembled token streams (no shader compiler is installed here);
`platform/D3D9.md` says how real ones would be built.

## Reading the log

    QueryInterface(IDirect3DDevice9) 0x00000000 -> 0x021006c0     the route exists
    QueryInterface(IDirect3DDevice9) 0x80004002 -> 0x00000000     E_NOINTERFACE: it does not
    create: vs_3_0 0x00000000 ...                                  every object created
    overlay frame 0 1920x1080 ms 8  draw 0x00000000                drawn, on the 8x MSAA back buffer
    frame 1801 tint 1 tinted draws so far 1260                    the swap ran 1260 times
