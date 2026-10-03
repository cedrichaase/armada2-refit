# MSAA.asi — multisample anti-aliasing

    msaa/install.sh               # build + install, 8x
    msaa/install.sh --samples 4   # a different count; 0 = patch nothing
    msaa/install.sh --remove      # complete uninstall (three files)
    cat "…/Star Trek Armada II/MSAA.log"   # what the last launch actually got

An ASI plugin of the same shape as `menus/`, loaded by the same
Ultimate ASI Loader (`winmm` override). It patches `Armada2.exe` in memory only.

A launch's `MSAA.log` at 8x:

    --- MSAA samples=8  sites patched 2/2, edge fill on
    CreateDevice 3440x1440 fmt 22 depth 80 fullscreen swap 2 flags 0x00000001  -> MSAA 8x, swap DISCARD, flags 0x00000000
    CreateDevice 640x480 fmt 22 depth 80 fullscreen swap 2 flags 0x00000001  -> MSAA 8x, swap DISCARD, flags 0x00000000
    CreateDevice 3440x1440 fmt 22 depth 80 fullscreen swap 2 flags 0x00000001  -> MSAA 8x, swap DISCARD, flags 0x00000000
    edge fill: row 0 and column 0 of 3440x1440 at 8x, after each Present

That is `X8R8G8B8` (22) over `D16` (80), and DXVK's log is free of errors, CopyRects
included. The 640x480 device is the mode `PlayIntroMovie` hard-codes for the launch
reels (`cutscenes/binkproxy/README.md` raises it), not something the plugin does. The
engine creates the device several times
per launch, and the hook covers each one, because every creation goes through the
same function.

**Alt-tab does not exercise `Reset` here.** DXVK signals device loss on focus loss
only with `d3d9.deviceLossOnFocusLoss = True`, which is off by default and not set
in `dxvk.conf`. So the in-place edit that `Reset` would reuse is correct, but under
this chain it is never called.

## Why code, and why MSAA rather than SMAA/FXAA

The engine creates its D3D8 device with `MultiSampleType = NONE` hard-coded.
DXVK 3.x has no key that forces MSAA, and `dxcfg.ini`'s `antialiasing=` belongs
to the GOG translator, which is not in the chain (`postfx/README.md`, Tier 2). vkBasalt could
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

## The top row and left column (`EdgeFill=`)

With MSAA on, a one-pixel line appeared along the top and the left of the 3D view in a
mission. It filled up with the map grid's colour as the camera panned: yellow dots in
column 0, a flat grey row 0 over the fog. Measured on the bench at 3440x1440 after the
same pan, in the first Federation mission:

| | column 0 | column 1 | fog row 0 | fog row 1 |
|---|---|---|---|---|
| `Samples=8` | 61,55,0 | 0,0,0 | 70,70,70 | 74,80,96 |
| `Samples=0` | 0,0,0 | 0,0,0 | equal | equal |
| `Samples=8`, edge fill | 0,0,0 | 0,0,0 | 74,80,96 | 74,80,96 |

**Why.** DXVK maps a D3D8/9 viewport to Vulkan offset by +0.49 px in x and y (D3D9
puts pixel centres on integers), and every primitive is clipped to the viewport, so
the scene covers from 0.49 onward. Without MSAA, pixel 0's single sample sits at 0.5
and is covered. With 8x, the samples in the left half of column 0 and the top half of
row 0 never are. The engine clears only depth in a mission (`ClearDepthBuffer`,
`0x623bf0`, flags `ZBUFFER`) and relies on the scene to overwrite every colour pixel,
so those samples keep whatever last reached them. That is the grid's lines, whose
width runs past the viewport edge. Each resolve then mixes the stale half in. No
D3D8 viewport can start below 0, and `dxvk.conf` has no key for the offset.

**The fix.** `ST3D_DeviceDirectX8::RefreshDisplay` ends every frame with one `Present`
(`call [edx+0x3c]` at `0x624735`, then the frame counter `inc [esi+0xa4]`). The plugin
replaces those two instructions with a jump to a stub. The stub presents, copies row 1
onto row 0 and column 1 onto column 0, increments the counter and jumps back. The copy
is sample for sample: same sample count, so DXVK's `StretchRect` copies rather than
resolves. The samples the next frame covers are drawn over. The ones it cannot reach
hold the neighbouring pixel from one frame earlier. HUD pixels on screen are therefore
unchanged, and the stale grid colour that was under the HUD's left edge is gone too.
DXVK refuses `CopyRects` within one surface, so each strip goes through a 1-pixel-thick
render target. It is created and released in the same call, so nothing outlives a frame
and device re-creation or `Reset` is unaffected. The site is byte-checked like the
others, and `EdgeFill=0` leaves it alone. Clearing those samples to black instead was
rejected: that leaves a half-brightness edge, which is a dark line over the fog.

Not this fix: the 3D view also stops one pixel short on the **right and bottom**
(column 3439 and row 1439 black at 3440x1440, with or without MSAA). That is the
engine's own viewport, constant, and unchanged here.

## Limits

MSAA smooths geometry edges: hulls, stations, planets, the skybox cube.
**Alpha-tested edges are not smoothed**, meaning sprite cut-outs and any texture
whose silhouette comes from alpha rather than polygons. Nothing in DXVK forces
alpha-to-coverage. If those still crawl, SMAA through vkBasalt on top is the next
step, and `postfx/postfx.py` already has the machinery for it.
