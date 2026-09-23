# MSAA.asi — multisample anti-aliasing

    tools/msaa/install.sh               # build + install, 8x
    tools/msaa/install.sh --samples 4   # a different count; 0 = patch nothing
    tools/msaa/install.sh --remove      # complete uninstall (three files)
    cat "…/Star Trek Armada II/MSAA.log"   # what the last launch actually got

An ASI plugin of the same shape as `tools/menuscale/`, loaded by the same
Ultimate ASI Loader (`winmm` override). It patches `Armada2.exe` in memory only.

## Why code, and why MSAA rather than SMAA/FXAA

The engine creates its D3D8 device with `MultiSampleType = NONE` hard-coded.
DXVK 3.x has no key that forces MSAA, and `dxcfg.ini`'s `antialiasing=` belongs
to the GOG translator the chain no longer uses (`SETUP.md`, Tier 2). vkBasalt could
run SMAA or FXAA. But those estimate edges from the finished frame, HUD text
included. MSAA resolves real geometric coverage and leaves flat UI quads as they
are, so it's the right tool for hull silhouettes.

## How it works

`ST3D_DeviceDirectX8::CreateDevice` (`0x6235d0`) fills a `D3DPRESENT_PARAMETERS`
at `this+0xac` and passes it to `IDirect3D8::CreateDevice`. It makes that call
twice: a first attempt, and a retry after `E_OUTOFMEMORY`. Just before each call it
runs `push edi` (the struct), then `call 0x62bc70` (`GetWindowHandle`). The
plugin redirects both of those `call`s to a stub. The stub edits `*edi` and then
jumps on to `GetWindowHandle`, with every register preserved.

| field | engine | plugin | why |
|---|---|---|---|
| `MultiSampleType` | 0 | 8 → 4 → 2 | the first count `CheckDeviceMultiSampleType` accepts for the back buffer **and** D16 depth |
| `SwapEffect` | FLIP / COPY | DISCARD | D3D8 allows MSAA only with DISCARD |
| `Flags` | `LOCKABLE_BACKBUFFER` | cleared | illegal with MSAA; the engine never locks it anyway |

The engine keeps the struct, and `Reset` after a device loss
(`TestCooperativeLevel`, `0x623c70`) reuses it. So MSAA survives alt-tab without
a second hook. DXVK validates neither the swap effect nor the flag, but the
plugin follows the D3D8 contract anyway rather than relying on that.

Each site's bytes are checked before anything is written, and both sites must
match or neither is patched. A different `Armada2.exe` leaves the plugin inert and
the log says so.

## What was checked before building it

- **Nothing else renders off-screen.** Every D3D device call in
  `ST3D_DeviceDirectX8` was read. No `SetRenderTarget`, no
  `CreateRenderTarget`, no `CreateDepthStencilSurface`, and no back-buffer
  lock. The remaining `+0x64`/`+0x68` calls are the engine's own
  `ST3D_Device` virtuals.
- **The one read-back is the minimap.** `CopyOffscreenToTexture`
  (`0x6261f0`) runs `GetRenderTarget` + `CopyRects` to put the radar terrain into
  the `minimap` texture. CopyRects from a multisampled surface is illegal in
  native D3D8. DXVK v3.0.2's `D3D8Device::CopyRects` sends every render-target
  source through `StretchRect` first, for every destination pool, and that
  resolves it. **If the minimap ever comes up black, this is the place to look.**
- **Nothing switches it off.** `D3DRS_MULTISAMPLEANTIALIAS` (161) defaults to
  TRUE, and the engine never sets it. The three `push 0xa1` in the exe are two
  dialog procs and a debug helper.

## Limits

MSAA smooths geometry edges: hulls, stations, planets, the skybox cube.
**Alpha-tested edges are not smoothed**, meaning sprite cut-outs and any texture
whose silhouette comes from alpha rather than polygons. Nothing in DXVK forces
alpha-to-coverage. If those still crawl, SMAA through vkBasalt on top is the next
step, and `tools/postfx.py` already has the machinery for it.
