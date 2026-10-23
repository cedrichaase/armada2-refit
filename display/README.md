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

## Will Vulkan work on a Mac?

`vkprobe/` is a small 32-bit program that does what the Vulkan backend does (a mapped
upload ring, dynamic uniform offsets, a texture upload, a present pass) and checks the
pixels: the question is whether winevulkan on MoltenVK maps host-visible memory where
a 32-bit process can reach it, in the new-style WoW64 that macOS Wine always is. How to
run it on a Mac, and what it showed here: `vkprobe/README.md`.
