# Display8 — the game's `d3d8.dll`, ours

`Armada2.exe` imports `Direct3DCreate8` from `d3d8.dll` and does all its drawing through
the Direct3D 8 device it gets back. Display8 is a `d3d8.dll` of our own, in `no_std`
Rust like render3d and the UI. The aim (the plan in the user's `plans/`) is that the
engine ends up drawing nothing: Display8 gives it a device that accepts everything, and
`Render3D.asi` and `UI.asi` draw the whole frame on Vulkan or Direct3D 12 directly.

    Armada2.exe -> d3d8.dll (Display8) -> d3d8_real.dll (crosire's d3d8to9) -> d3d9.dll (DXVK; Windows' own)

## Status

**Forward mode.** Display8 stands in front of the real chain and passes every call
through. `./install` puts it in (`display/install`), `a2mod` switches it as the
`display` layer, and `platform/d3d8-chain.py` switches the chain behind it. On the
test bench it traces every call (`Trace=auto`: on in an `a2test` clone). It is not in
the release package: in forward mode it changes nothing a player sees; it joins with
the stub.

    ./a2test session start --res 1280x720 --install .

## Installing

`display/install` (and `--remove`):

| File | What |
|---|---|
| `d3d8.dll` | Display8 |
| `d3d8_real.dll` | the `d3d8.dll` that was there: crosire's d3d8to9 after `./install`'s platform step. Display8 loads it (`Real=auto`) and passes every call to it |
| `d3d8.dll.a2display-backup` | the same file again: what `a2mod stock` copies back into `d3d8.dll`. Stock is the stock chain's binary, not ours passing through |
| `Display8.ini` | the settings; a key the player's copy lacks is added (`ui/ini-merge.py`) |

With no `d3d8.dll` in the game directory (the system's is the chain), Display8 goes in
alone and loads the system's own (`GetSystemDirectory`: SysWOW64 on 64-bit Windows,
Wine's builtin under Wine). Run again, the installer replaces only `d3d8.dll`.
`d3d8-chain.py` knows Display8 by the panic message every build carries, not by a hash
(ours changes with every build); behind it, the chain's slot is `d3d8_real.dll`, and
every switch there rewrites the backup with it, so `a2mod stock` restores the chain in
use.

Wine loads the game directory's `d3d8.dll` because of the `d3d8=n,b` override
(`platform/README.md`); `d3d8_real.dll` is loaded by full path, so it needs none.

## What the plugins can ask of it

Plain C exports, found with `GetProcAddress(GetModuleHandleA("d3d8.dll"), name)`
(`src/api.rs`); a plugin that does not find them is on another `d3d8.dll` and works
as before:

| Export | What |
|---|---|
| `display8_version()` | the version, a NUL-terminated string |
| `display8_device_epoch()` | moves on at every `CreateDevice`, every `Reset` that succeeds, and when the device goes: whatever a plugin made at another epoch must be made again |
| `display8_device()` | the engine's device now (ours, not referenced), or null |
| `display8_frame()` | frames presented so far |

## How it works

**An object of ours for every interface.** `IDirect3D8`, the device, the swap chain,
the three kinds of texture, surfaces, volumes, vertex and index buffers: each is a
vtable of ours (`src/d3d8/`) and the real object it stands for (`src/com.rs`). Every
method sits in its slot with its exact `__stdcall` argument list, from Wine's
`include/d3d8.h`; one argument too many or too few unbalances the caller's stack. A
method that takes an object swaps ours for the real one (`SetTexture`,
`SetRenderTarget`, `CopyRects`, ...), and one that hands an object back wraps it
(`CreateTexture`, `GetBackBuffer`, `GetSurfaceLevel`, `GetContainer`, ...).

**One object of ours per real object.** A registry maps each real pointer to ours, so
`GetTexture` hands back the very pointer the engine set. The reference count is the real
object's: `AddRef` and `Release` pass through, and ours is freed when the real one
reports 0. In Direct3D 9 a texture's surface shares its texture's count, so a surface
of ours may outlive its real one; a later real object at the same address and of the
same kind takes it over, which is sound because ours holds nothing but the pointer.

**`QueryInterface` for anything that is not Direct3D 8's goes to the real object.**
That is how the plugins reach Direct3D 9 through the engine's objects
(`IID_IDirect3DDevice9` on the device; `IID_IDirect3DTexture9` on the minimap's and the
fog's textures), as they did on d3d8to9 itself (`platform/D3D9.md`).

**The plugins chain slots of our device's vtable** (`Reset`, `Present`: `ui/src/engine/device.rs`,
`render3d/src/frame/mod.rs`) with `VirtualProtect` and a write, as they did on
d3d8to9's. The vtables are therefore mutable statics, filled at start-up, and nothing in
Display8 calls its own methods through them.

## Callers

Each method reads its own frame pointer (every function keeps one:
`force-frame-pointers` in `.cargo/config.toml`), so `[ebp+4]` is the exact return address
into whoever called it. The engine is built without frame pointers, so for the callers
beyond that one the stack is scanned for values that point just past a `call`
instruction in `Armada2.exe`'s code (`E8`, or `FF /2` of 2 to 7 bytes): up to five. The
first is exact; the rest are the usual heuristic, and a stale return address left in an
uninitialised local passes it too. State setters and getters, called thousands of
times a frame, keep only the exact one.

Addresses in the exe are written as they are and named offline through `armada2.map`;
an address in any other module is written `Name.dll+offset`. The map and the names
never go into this repository (hard rule 9).

## The inventory

With `Trace=1` in `Display8.ini`:

- **`Display8.inventory.tsv`**, beside the exe, rewritten every two seconds of
  presenting and at each new context: a row per context, interface, method and callers,
  with the calls, the frames they fell in, and how many came from a thread other than
  the device's. `#` lines at the end give each context's frames, engine draws and time
  spent in the real `Present`.
- **`Display8.log`**: each distinct line once per context: the answers the engine
  branches on (`ANSWER`: caps, formats, modes, `ValidateDevice`), the present
  parameters of every `CreateDevice` and `Reset`, gamma ramps (`GAMMA`), every lock
  (`LOCK`) and read-back (`READBACK`), render targets, the plugins' queries for
  Direct3D 9 (`QI`), and the `user32`/`gdi32` calls that read the mode or move the mouse
  (`MODE`, `MOUSE`, `WINDOW`: `src/user32.rs`, hooked in the exe's import table).
- **Contexts** are labels the bench sets: `Mark the display context "mission"` writes
  `Display8.mark`, read at most four times a second at `Present` and at every
  `CreateDevice` and `Reset`, then deleted. Before the first mark the context is
  `start`.
- **`Expect the engine to draw nothing in "mission"`** reads the inventory: no `Draw*`
  on the device whose caller is in `Armada2.exe`, in that context.

## Drop switches

`[Drop]` in `Display8.ini` (`src/drop.rs`): the engine's draws, accepted and not passed
on. Read again every 60 presents, so one session can shoot a frame with and without.

| Key | Drops |
|---|---|
| `Drop2D=1` | the engine's 2D: its batched quads (the trifan buffer), its lines, the fades |
| `Drop3D=1` | the meshes it still draws (render3d's hand-backs): every draw whose immediate caller is a workspace's `Submit` (`ST3D_WorkspaceDirectX8::Submit` 0x648af0..0x648c30, `ST3D_WorkspaceDirectX8NonVB::Submit` 0x648520..0x6485d0), which every mesh the engine draws goes through |
| `DropLoading=1` | everything drawn from the engine's loading loop: any draw with a return address in `RenderLoopAssetLoadStatusUpdate` (0x598ba0..0x598ff0) among the sixteen found up its stack, which is the function the loop calls to draw each step's frame (the logo scene, the load bar) |
| `Callers=0x..., 0x...` | any draw with one of these return addresses on its call chain |

A draw is the engine's when its immediate caller is in `Armada2.exe`; the plugins draw
through Direct3D 9 and never pass here. With the refit installed and every category's
work taken over, the game must look the same with the switch on: that is how each
context is proved done before the stub drops everything.

## Naming a draw

- **`Display8.go`** (`src/detail.rs`): while it exists beside the exe (looked for every
  30 presents, then deleted), the next `[Trace] Frames=` frames (default 3) log a `DRAW`
  line for each draw from `Armada2.exe`: the blend and depth state, the texture, and for
  vertices that can be read, their box on the screen and their colours. The engine's 2D
  goes through one dynamic vertex buffer it writes by lock (its trifan buffer), so
  during a capture each `Unlock` copies what was written. This names a draw without a
  picture: the fade from black is a full-screen black quad whose alpha falls frame by
  frame, the drag box four white lines.

## Stub mode

`Mode=stub` (`src/stub/`): the real chain is never loaded. The engine gets an
`IDirect3D8` and devices of ours that accept everything and draw nothing, and Display8
makes a Direct3D 9 device of its own (`d3d9.dll` as the loader finds it: DXVK's beside
the exe under Proton, Windows' own) on the game's window. render3d and the UI find it
exactly as they found d3d8to9's, through `QueryInterface(IID_IDirect3DDevice9)` on the
engine's device, and draw the whole frame on it.

- **One Direct3D 9 device for the game's life**, made at the engine's first
  `CreateDevice`. Its later `CreateDevice`s and `Reset`s get a new device object of
  ours on the same Direct3D 9 device, or nothing at all.
- **One display mode: the desktop's.** The engine is offered only that one (X8R8G8B8,
  the adapter's refresh), so it draws at the size the swap chain has and no mouse
  coordinate needs translating (phase 0 found the engine reads the mouse only through
  `GetCursorPos`, `ScreenToClient` and `GetClientRect`). The game's window becomes a
  borderless one covering the screen, and the swap chain is windowed on it.
- **What the plugins read of the engine's state reaches the Direct3D 9 device**: the
  transforms (render3d reads the view and the projection), the viewport, `Clear` of the
  engine's own back buffer and depth, `BeginScene` and `EndScene`. Everything else is
  kept for the engine's own `Get*` and goes nowhere.
- **Resources are memory.** A texture's levels, a surface's pixels and a buffer's bytes
  live in Display8, and a lock hands out a pointer into them, as the managed pool's
  system copy did. Draws are counted (the inventory) and dropped.
- **The mirror table.** A texture a plugin asks for as Direct3D 9 (the radar's two
  scan-grid sprites, phase 0's only ones; the unit view's Bink frame) is made there
  once with every level and brought up to date when the engine unlocks a level, at
  once on the device's thread, else at the next `BeginScene` or `Present`
  (`BinkThread` writes from its own).
- **The back buffer's pixels** (the movie path locks it and writes each frame) are
  copied into the swap chain's at `Present`.
- **The gamma ramp in the last pass** (`src/stub/gamma.rs`, `shaders/present.hlsl`): a
  windowed swap chain ignores `SetGammaRamp`, and on the real chain the ramp the engine
  sets at start-up lifts every frame (DXVK's presenter applies it: 2 to 3, 80 to 90).
  So the back buffer is copied and drawn back through a 256x1 lookup. Measured on the
  showcase frame against the real chain's screen: the frames the plugins render are the
  same to the last pixel (RMSE 0: scene, bloom levels, presented frame), and the screen
  differs by at most 1/255 (RMSE 0.28): DXVK's mapping of a ramp value to 8 bits is none
  of rounding (64 values of 256 per channel off by one), truncating ramp/257 (142),
  interpolating half a texel back (13) or the high byte (13, what is used).
- **Stands down** to forward mode when `HUD.asi`, `GridLayout.asi` or `Menus.asi` is
  loaded (each changes what the engine draws, which the stub drops; `GridLayout.asi`
  would leave no button bar at all, since the UI's stands down beside it), or when
  Direct3D 9 cannot be made. The log says which.

`display/bench-mode.sh stub` sets the mode in a bench clone from a scenario's `Setup:`;
`display-stub` walks the whole game on it.

## Backends

`Backend=` in `Display8.ini` says what the stub draws on: `d3d9` (above, the default),
`vulkan` (`src/vk/`) or `d3d12` (`src/dx12/`). On the last two no `d3d9.dll` is loaded
at all: Display8 owns the device, the swap chain and the frame, and render3d's and the
UI's `gpu` record into that frame through the plain C exports of
`gpu/src/display.rs`, the contract both sides build against (its doc comment is the
specification). `gpu` has its Vulkan backend (`gpu/README.md`, "The Vulkan backend"),
so on `vulkan` render3d and the UI draw the whole frame as on `d3d9`. Where `gpu` does
not yet have the backend (`d3d12`), the plugins find no device to draw with and stand
down (the UI's menus run as stock dialogs, "no Direct3D 9
device to draw with"), and what is seen is what Display8 draws itself: the engine's
`Clear`s, the movie frames the engine writes into the back buffer by lock, and the last
pass through the gamma ramp. `Validate=1` asks for the Vulkan validation layer or
Direct3D 12's debug layer, where the system has one.

### Vulkan

`Backend=vulkan` (`src/vk/`): no Direct3D 9 at all. Display8 opens `vulkan-1.dll` and
makes the instance and device (Vulkan 1.1, one queue family that draws and presents,
`VK_KHR_swapchain`; anisotropic filtering where the device has it), a swap chain on the
game's window, an offscreen `B8G8R8A8` colour image and a `D16` depth image standing for
the engine's back buffer and depth, and two frames in flight, each with a command
buffer, a fence, a descriptor pool and an upload ring. render3d and the UI draw through
`gpu`'s Vulkan backend (`gpu/README.md`, "The Vulkan backend") into the frame's command
buffer, reaching all of it through the exports of `gpu/src/display.rs` (the contract);
at the engine's `Present` Display8 draws the colour image through the gamma ramp into
the swap chain's image, submits and presents.

- **The backend is said early.** `display8_backend` answers Vulkan from the engine's
  `Direct3DCreate8` on, before the device: an object a plugin makes then is made for
  the right backend. A stub on Vulkan has no Direct3D 9 to fall back to.
- **The engine's targets** are in their attachment layouts between recorders, the
  mirrors in `SHADER_READ_ONLY`; whoever changes one puts it back. The depth image is
  sampled and copied too (render3d's depth resolve, the dumps).
- **`display8_vk_flush`** submits what the frame has recorded and waits, then begins the
  same command buffer again (its pool reset): a plugin's read-back (the frame dumps).
- **Mirrors** are the engine textures a plugin samples (`display8_vk_mirror`), as the
  Direct3D 9 mirror table has them, brought up to date when the frame is asked for.
- **`Device=`** takes only a device whose name has it (the test bench: `llvmpipe`,
  Mesa's CPU Vulkan, never the GPU); **`Validate=1`** the Khronos validation layer, its
  warnings and errors in `Display8.log` (`vulkan 0x...:`); **`PixelCentres=d3d9`**
  Direct3D 9's pixel centres for the plugins (`gpu/README.md`, "Pixel centres"), for
  comparing with `Backend=d3d9` pixel for pixel.

- **The pipeline cache is kept on disk**, in `%LOCALAPPDATA%\armada2-refit\Display8.pipelines`
  (`src/vk/cache.rs`), not the game directory: GOG installs into `Program Files`,
  which is not writable, and an exe with no manifest is silently redirected to
  VirtualStore there. It is read when the device is made, only if its header names this
  device and driver (vendor, device, cache UUID), and written again, through a
  temporary file and a rename, whenever it has grown (looked at every 600 frames).
- **Samplers as DXVK makes them.** On Direct3D 9 the game's `dxvk.conf` (the renderer
  layer, `renderer/README.md`) has DXVK give every linear sampler 16x anisotropy and
  every sampler with mips a LOD bias of -0.5. Display8 reads the same keys
  (`d3d9.samplerAnisotropy`, `d3d9.samplerLodBias`, `d3d9.clampNegativeLodBias`) and
  hands them to `gpu` (`VkDevice8::sampler_anisotropy`, `sampler_lod_bias`), so the two
  backends filter alike. Without it every hull read a level blurrier on Vulkan
  (showcase RMSE 5.07 -> 4.05). Vulkan's cube maps are always seamless, as
  `d3d9.seamlessCubes = True` makes DXVK's.

On the bench every Vulkan game runs on lavapipe with the layer on (unless
`A2TEST_VULKAN_GPU=1`, for a deliberate run on the GPU): the game's
environment gets `VK_DRIVER_FILES` (and `VK_ICD_FILENAMES`) naming
`/usr/share/vulkan/icd.d/lvp_icd.json` and `VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation`
(umu passes the bench's environment on; pressure-vessel honours both;
`testbench/bench/session.py`, `vulkan_env`, sets them from the clone's `Display8.ini`),
and `display/bench-mode.sh` gives `Display8.ini` `Device=llvmpipe` and `Validate=1`, so even an environment that lost the variables
cannot reach the GPU. `Display8.log` names the device (`vulkan: physical device
llvmpipe ...`); the process's maps show `libVkLayer_khronos_validation.so` and
`libvulkan_lvp.so`, and no other driver. A frame takes seconds there: its time means
nothing.

### Direct3D 12

`src/dx12/`, the same stub as on Vulkan, by the same plan:

- `d3d12.dll` and `dxgi.dll` by `LoadLibraryA`; the device at feature level 11_0 on the
  default adapter, one direct queue. Every interface is called by vtable slot, declared
  by hand from Wine's IDLs (`src/dx12/raw.rs`, each structure's 32-bit size checked at
  compile time). A method that returns a structure (a descriptor heap's start) takes a
  hidden pointer after `this`, as Microsoft's compiler passes it.
- A flip-model swap chain (`FLIP_DISCARD`, two B8G8R8A8 buffers) on the game's window,
  with DXGI's Alt+Enter and window changes off; sync interval 1 unless the engine asks
  for `D3DPRESENT_INTERVAL_IMMEDIATE`.
- The engine's back buffer and depth are B8G8R8A8 and D16 resources of ours, in
  `RENDER_TARGET` and `DEPTH_WRITE` between recorders; `Clear` clears them by
  rectangles, clipped to the viewport.
- Two frames in flight, each with a command allocator and list, its fence value, an
  upload ring (upload-heap buffers mapped for good, 4 MB chunks added as a frame needs
  them) and the frees deferred to it.
- Descriptors: two shader-visible heaps, CBV/SRV/UAV (32768 a frame) and samplers (1024
  a frame, Direct3D 12's 2048 in all), set on the frame's list when it begins; the
  views of Display8's own textures live in a heap that is not shader-visible and are
  copied into a frame's range to be bound. A recorder takes its descriptors from those
  ranges and never sets heaps of its own (only one of each type can be set).
- Mirrors as on Vulkan: B8G8R8A8 textures with every level, X8R8G8B8 through a view
  whose alpha reads one, in `PIXEL_SHADER_RESOURCE | NON_PIXEL_SHADER_RESOURCE`.
- The last pass is `shaders/present.hlsl` as DXIL (`src/shaders/present_dxil.rs`), with
  a root signature of four root constants (b0) and a table of the frame and the ramp
  (t0, t1) under two static point samplers.
- A device that is removed is logged with its reason, and nothing is drawn after.

**On the bench** (Proton-CachyOS), the 32-bit game gets Direct3D 12 with no setting:
the prefix's `syswow64/d3d12.dll` is vkd3d-proton and its `dxgi.dll` DXVK's, and
Proton's own launch makes both native (the bench's overrides name only `winmm`, `d3d8`
and `d3d9`). DXVK's DXGI logs that it ignores `MakeWindowAssociation`'s flags.
Measured 2026-10-23: the device comes up at 11_0 on the RX 5700, the intro movies play
through the back buffer, the gamma pass and the flip-model swap chain, and a mission
(`-nointro a2_borg01`, the plugins standing down) presents past frame 1000 and opens
the in-mission menu.

## Will Vulkan work on a Mac?

`vkprobe/` is a small 32-bit program that does what the Vulkan backend does (a mapped
upload ring, dynamic uniform offsets, a texture upload, a present pass) and checks the
pixels: the question is whether winevulkan on MoltenVK maps host-visible memory where
a 32-bit process can reach it, in the new-style WoW64 that macOS Wine always is. How to
run it on a Mac, and what it showed here: `vkprobe/README.md`.
